use crate::crypto::insecure::InsecurePrivateKey;
use crate::mojang;
use crate::protocol::*;
use crate::sim;
use log::{error, info, trace};
use rand::{self, RngCore};
use std::error::Error;
use std::net::SocketAddr;
use stream_cipher::NewStreamCipher;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::{Receiver, Sender};

#[derive(Debug)]
pub enum ConnectionError {
    NotImplemented,
    InvalidTransition,
    InvalidVerifier,
    InvalidKey,
    ServerClosing,
    UnsupportedVersion
}

impl Error for ConnectionError {}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Self::NotImplemented => write!(f, "This feature is not supported by this server."),
            Self::InvalidTransition => write!(f, "The Minecraft client attempted to perform an invalid action."),
            Self::InvalidVerifier => write!(f, "Authentication failed."),
            Self::InvalidKey => write!(f, "Authentication failed."),
            Self::ServerClosing => write!(f, "The server is shutting down."),
            Self::UnsupportedVersion => write!(f, "Your client version is not supported."),
        }
    }
}

pub struct Connection<R: AsyncRead + Unpin + Send + 'static, W: AsyncWrite + Unpin + Send + 'static>
{
    key: Box<InsecurePrivateKey>,
    state: ConnectionState,
    addr: SocketAddr,
    player_name: Option<String>,
    verify: Option<Vec<u8>>,
    reader: PacketReader<R>,
    writer: PacketWriter<W>,
    recv: Option<Receiver<sim::ClientMessages>>,
    send: Sender<sim::SimMessages>,
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
            ConnectionState::AwaitingStatusRequest => write!(f, "({}-{} state)", self.addr, player_name),
            ConnectionState::AwaitingStatusPing => write!(f, "({}-{} ping)", self.addr, player_name),
            ConnectionState::AwaitingLogin => write!(f, "({}-{} login)", self.addr, player_name),
            ConnectionState::AwaitingEncryptionResponse => {
                write!(f, "({}-{} encrypt)", self.addr, player_name)
            }
            ConnectionState::RunningGame => write!(f, "({}-{} running)", self.addr, player_name),
        }
    }
}

impl<R: AsyncRead + Unpin + Send + 'static, W: AsyncWrite + Unpin + Send + 'static>
    Connection<R, W>
{
    pub fn new(
        reader: R,
        writer: W,
        send: Sender<sim::SimMessages>,
        addr: SocketAddr,
        key: InsecurePrivateKey,
    ) -> Self {
        let writer = PacketWriter::new(writer);
        let reader = PacketReader::new(reader);
        Self {
            addr,
            reader,
            writer,
            send,
            state: ConnectionState::Open,
            key: Box::new(key),
            player_name: None,
            verify: None,
            recv: None,
        }
    }

    pub fn execute(mut self) {
        tokio::spawn(async move {
            let e = loop {
                let result = match self.state {
                    ConnectionState::Open => self.execute_open().await,
                    ConnectionState::AwaitingStatusRequest => self.execute_status_request().await,
                    ConnectionState::AwaitingStatusPing => self.execute_status_ping().await,
                    ConnectionState::AwaitingLogin => self.execute_login().await,
                    ConnectionState::AwaitingEncryptionResponse => {
                        self.execute_encryption_response().await
                    }
                    ConnectionState::RunningGame => self.execute_game().await,
                };

                if let Err(error) = result {
                    error!("{} client encountered an error: {:?}", self, error);
                    let error = format!("{}", error);
                    break error;
                };
            };

            info!("{} disconnecting", self);
            self.disconnect_client(e).await;
        });
    }

    async fn disconnect_client(&mut self, reason: String) {
        if let Ok(chat) = crate::chat::trivial(&reason) {
            let _ = match self.state {
                ConnectionState::RunningGame => {
                    write_disconnect_play(&mut self.writer, &chat).await
                }
                _ => write_disconnect_login(&mut self.writer, &chat).await,
            };
        }
    }

    async fn execute_open(&mut self) -> Result<(), Box<dyn Error>> {
        match open::read_packet(&mut self.reader).await? {
            open::Packet::Handshake(handshake) => match handshake.next_state() {
                open::RequestedState::Login => {
                    trace!("{} request to transition to login state", self);
                    self.state = ConnectionState::AwaitingLogin;
                    Ok(())
                }
                open::RequestedState::Status => {
                    trace!("{} request to transition to status state", self);
                    self.state = ConnectionState::AwaitingStatusRequest;
                    Ok(())
                }
            },
        }
    }
    
    async fn execute_status_request(&mut self) -> Result<(), Box<dyn Error>> {
        match status::read_packet(&mut self.reader).await? {
            status::Packet::Request => {
                trace!("{} request for server status", self);
                status::write_response(&mut self.writer).await?;
                self.state = ConnectionState::AwaitingStatusPing;

                Ok(())
            }
            _ => Err(ConnectionError::InvalidTransition.into()),
        }
    }
    
    async fn execute_status_ping(&mut self) -> Result<(), Box<dyn Error>> {
        match status::read_packet(&mut self.reader).await? {
            status::Packet::Ping(ping) => {
                trace!("{} request for ping", self);
                status::write_pong(&mut self.writer, ping.timestamp()).await?;

                Ok(())
            }
            _ => Err(ConnectionError::InvalidTransition.into()),
        }
    }

    async fn execute_login(&mut self) -> Result<(), Box<dyn Error>> {
        match login::read_packet(&mut self.reader).await? {
            login::Packet::LoginStart(login) => {
                trace!("{} request to login as: {}", self, login.player_name());
                let mut verify = vec![0u8; 16];
                rand::thread_rng().fill_bytes(&mut verify);
                login::write_encryption_request(&mut self.writer, self.key.public_der(), &verify)
                    .await?;
                self.player_name = Some(login.player_name().to_string());
                self.verify = Some(verify);
                self.state = ConnectionState::AwaitingEncryptionResponse;

                Ok(())
            }
            _ => Err(ConnectionError::InvalidTransition.into()),
        }
    }

    async fn execute_encryption_response(&mut self) -> Result<(), Box<dyn Error>> {
        match login::read_packet(&mut self.reader).await? {
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
                let player_info =
                    mojang::api::player_join_session(&player_name, &server_hash).await?;
                trace!(
                    "{} player info retrieved for {} with uuid {}",
                    self,
                    player_info.player_name(),
                    player_info.uuid()
                );

                let aes_out = match AesCfb8::new_var(&key, &key) {
                    Ok(r) => r,
                    Err(_) => return Err(ConnectionError::InvalidKey.into()),
                };
                let aes_in = match AesCfb8::new_var(&key, &key) {
                    Ok(r) => r,
                    Err(_) => return Err(ConnectionError::InvalidKey.into()),
                };

                self.writer.encrypt(aes_out);
                self.reader.decrypt(aes_in);

                login::write_login_success(
                    &mut self.writer,
                    &player_info.uuid(),
                    &player_info.player_name(),
                )
                .await?;

                info!(
                    "{} player {} connected with uuid {}",
                    self,
                    player_info.player_name(),
                    player_info.uuid()
                );
                let (message, recv) = sim::create_client(10);
                self.recv = Some(recv);

                if self.send.send(message).await.is_err() {
                    return Err(ConnectionError::ServerClosing.into());
                }

                self.state = ConnectionState::RunningGame;
                Ok(())
            }
            _ => Err(ConnectionError::InvalidTransition.into()),
        }
    }
    async fn execute_game(&mut self) -> Result<(), Box<dyn Error>> {
        let recv = match &mut self.recv {
            None => return Err(ConnectionError::InvalidTransition.into()),
            Some(m) => m,
        };

        let message = match recv.recv().await {
            None => return Err(ConnectionError::ServerClosing.into()),
            Some(m) => m,
        };

        match message {
            sim::ClientMessages::JoinGame(j) => {
                j.write(&mut self.writer).await?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    Open,

    // Status
    AwaitingStatusRequest,
    AwaitingStatusPing,

    // Login
    AwaitingLogin,
    AwaitingEncryptionResponse,

    // Game
    RunningGame,
}
