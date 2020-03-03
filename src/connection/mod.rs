use crate::crypto::insecure::InsecurePrivateKey;
use crate::mojang;
use crate::protocol::packet::*;
use crate::protocol::writers::AesCfb8;
use circular::Buffer;
use log::{error, info, trace};
use rand::{self, RngCore};
use std::error::Error;
use std::net::SocketAddr;
use stream_cipher::{NewStreamCipher, StreamCipher};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};

#[derive(Debug)]
pub enum ConnectionError {
    NotImplemented,
    InvalidTransition,
    InvalidVerifier,
    InvalidKey,
}

impl Error for ConnectionError {}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Self::NotImplemented => write!(f, "NotImplemented"),
            Self::InvalidTransition => write!(f, "InvalidTransition"),
            Self::InvalidVerifier => write!(f, "InvalidVerifier"),
            Self::InvalidKey => write!(f, "InvalidKey"),
        }
    }
}

pub struct Connection<R: AsyncRead + Unpin + Send + 'static, W: AsyncWrite + Unpin + Send + 'static>
{
    key: Box<InsecurePrivateKey>,
    read_buffer: Buffer,
    state: ConnectionState,
    addr: SocketAddr,
    player_name: Option<String>,
    verify: Option<Vec<u8>>,
    aes_in: Box<Option<AesCfb8>>,
    aes_out: Box<Option<AesCfb8>>,
    reader: R,
    writer: W,
}

impl<R: AsyncRead + Unpin + Send + 'static, W: AsyncWrite + Unpin + Send + 'static>
    std::fmt::Display for Connection<R, W>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        let player_name: &str = if let Some(player_name) = &self.player_name {
            player_name.as_ref()
        } else {
            "*"
        };

        match &self.state {
            ConnectionState::Open => write!(f, "({}-{} new)", self.addr, player_name),
            ConnectionState::AwaitingLogin => write!(f, "({}-{} login)", self.addr, player_name),
            ConnectionState::AwaitingEncryptionResponse => {
                write!(f, "({}-{} encrypt)", self.addr, player_name)
            }
            ConnectionState::RunningGame => write!(f, "({}-{} running)", self.addr, player_name),
        }
    }
}

macro_rules! read_packet {
    ($self:expr, $method:path, $type:ty { $($pat:pat => $handler:expr),* }) => {{
        let mut consume = 0;
        let result: Result<_, Box<dyn Error>> = loop {
            match $self.reader.read($self.read_buffer.space()).await {
                Ok(n) => {
                    if n == 0 {
                        break Err(Box::new(std::io::Error::from(std::io::ErrorKind::NotConnected)));
                    }

                    if let Some(aes) = &mut *$self.aes_in {
                        aes.decrypt(&mut $self.read_buffer.space()[0..n]);
                    }

                    $self.read_buffer.fill(n);
                    let len = $self.read_buffer.available_data();
                    let consume = match $method($self.read_buffer.data()) {
                        Ok((remainder, packet)) => {
                            match packet {
                                $(
                                    $pat => {
                                        consume = len - remainder.len();
                                        break $handler;
                                    }
                                ),*
                            }
                        },
                        Err(nom::Err::Incomplete(nom::Needed::Size(sz))) => sz,
                        Err(nom::Err::Incomplete(nom::Needed::Unknown)) => 4096,
                        Err(nom::Err::Error(error)) => break Err(error.into()),
                        Err(nom::Err::Failure(error)) => break Err(error.into()),
                    };

                    $self.read_buffer.consume(consume);
                }
                Err(error) => break Err(Box::new(error)),
            }
        };
        $self.read_buffer.consume(consume);
        result
    }};
}

impl<R: AsyncRead + Unpin + Send + 'static, W: AsyncWrite + Unpin + Send + 'static>
    Connection<R, W>
{
    pub fn new(reader: R, writer: W, addr: SocketAddr, key: InsecurePrivateKey) -> Self {
        Self::with_buffer(reader, writer, addr, key, Buffer::with_capacity(4096))
    }

    pub fn with_buffer(
        reader: R,
        writer: W,
        addr: SocketAddr,
        key: InsecurePrivateKey,
        read_buffer: Buffer,
    ) -> Self {
        Self {
            read_buffer,
            addr,
            state: ConnectionState::Open,
            key: Box::new(key),
            player_name: None,
            verify: None,
            aes_in: Box::new(None),
            aes_out: Box::new(None),
            reader,
            writer,
        }
    }

    pub fn execute(mut self) {
        tokio::spawn(async move {
            let e = loop {
                let result = match self.state {
                    ConnectionState::Open => self.execute_open().await,
                    ConnectionState::AwaitingLogin => self.execute_login().await,
                    ConnectionState::AwaitingEncryptionResponse => {
                        self.execute_encryption_response().await
                    }
                    ConnectionState::RunningGame => Err(ConnectionError::NotImplemented.into()),
                };

                match result {
                    Ok(_) => trace!("{} disconnected", self),
                    Err(error) => {
                        error!("{} client encountered an error: {}", self, error);
                        let error = format!("{:?}", error);
                        break error;
                    }
                };
            };

            self.disconnect_client(e).await;
        });
    }

    async fn disconnect_client<'a>(&'a mut self, reason: String) {
        if let Ok(chat) = crate::chat::trivial(&reason) {
            let _ = match self.state {
                ConnectionState::RunningGame => {
                    crate::protocol::packet::Disconnect::play(&chat)
                        .write(&mut self.writer, (*self.aes_out).as_mut())
                        .await
                }
                _ => {
                    crate::protocol::packet::Disconnect::play(&chat)
                        .write(&mut self.writer, (*self.aes_out).as_mut())
                        .await
                }
            };
        }
    }

    async fn execute_open<'a>(&'a mut self) -> Result<(), Box<dyn Error>> {
        read_packet!(self, open::take_packet, open::Packet<'a> {
            open::Packet::Handshake(handshake) => match handshake.next_state() {
                open::RequestedState::Login => {
                    trace!("{} request to transition to login state", self);
                    self.state = ConnectionState::AwaitingLogin;
                    Ok(())
                },
                open::RequestedState::Status => {
                    Err(ConnectionError::NotImplemented.into())
                }
            }
        })
    }

    async fn execute_login<'a>(&'a mut self) -> Result<(), Box<dyn Error>> {
        read_packet!(self, login::take_packet, login::Packet<'a> {
            login::Packet::LoginStart(login) => {
                trace!("{} request to login as: {}", self, login.player_name());
                let mut verify = vec![0u8; 16];
                rand::thread_rng().fill_bytes(&mut verify);
                login::EncryptionRequest::new(self.key.public_der(), &verify).write(&mut self.writer, (*self.aes_out).as_mut()).await?;
                self.player_name = Some(login.player_name().to_string());
                self.verify = Some(verify);
                self.state = ConnectionState::AwaitingEncryptionResponse;

                Ok(())
            },
            _ => {
                Err(ConnectionError::InvalidTransition.into())
            }
        })
    }

    async fn execute_encryption_response<'a>(&'a mut self) -> Result<(), Box<dyn Error>> {
        read_packet!(self, login::take_packet, login::Packet<'a> {
            login::Packet::EncryptionResponse(enc) => {
                trace!("{} encryption response received", self);
                let player_name = if let Some(player_name) = &self.player_name {
                    player_name
                } else {
                    return Err(ConnectionError::InvalidTransition.into());
                };
                let verify = if let Some(verify) = &self.verify {
                    verify
                } else {
                    return Err(ConnectionError::InvalidTransition.into());
                };

                const KEY_SIZE: usize = 16;

                let incoming_verify = self.key.decrypt(enc.encrypted_verifier());
                if incoming_verify.len() < verify.len() {
                    return Err(ConnectionError::InvalidVerifier.into());
                }

                if enc.encrypted_shared_secret().len() < KEY_SIZE {
                    return Err(ConnectionError::InvalidKey.into());
                }

                let padding = incoming_verify.len() - verify.len();
                let incoming_verify = &incoming_verify[padding..];

                if !crate::crypto::equals_constant_time(verify, incoming_verify) {
                    return Err(ConnectionError::InvalidVerifier.into());
                }

                trace!("{} verifier validated", self);
                let key = self.key.decrypt(enc.encrypted_shared_secret());
                let padding = key.len() - KEY_SIZE;
                let key = &key[padding..];

                trace!("{} key decrypted", self);
                let server_hash = mojang::hash(b"" as &[u8], &key, self.key.public_der());
                let player_info = mojang::api::player_join_session(&player_name, &server_hash).await?;
                trace!("{} player info retrieved for {} with uuid {}", self, player_info.player_name(), player_info.uuid());

                let aes_out = match AesCfb8::new_var(&key, &key) {
                    Ok(r) => r,
                    Err(_) => return Err(ConnectionError::InvalidKey.into())
                };
                let aes_in = match AesCfb8::new_var(&key, &key) {
                    Ok(r) => r,
                    Err(_) => return Err(ConnectionError::InvalidKey.into())
                };
                self.aes_in = Box::new(Some(aes_in));
                self.aes_out = Box::new(Some(aes_out));
                login::LoginSuccess::new(&player_info.uuid(), &player_info.player_name()).write(&mut self.writer, (*self.aes_out).as_mut()).await?;

                info!("{} player {} connected with uuid {}", self, player_info.player_name(), player_info.uuid());

                Err(ConnectionError::NotImplemented.into())
            },
            _ => {
                Err(ConnectionError::InvalidTransition.into())
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    Open,

    // Login
    AwaitingLogin,
    AwaitingEncryptionResponse,

    // Game
    RunningGame,
}
