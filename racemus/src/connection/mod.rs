mod models;
pub use models::*;

use racemus_binary::{proto::*, *};
use racemus_mc::{api::session::has_joined, chat};
use racemus_tools::crypto::insecure::InsecurePrivateKey;

use crate::controllers::{player, Controllers};
use async_std::{
    io::{Read, Write},
    sync::Receiver,
};
use log::{error, info, trace};
use rand::{self, RngCore};
use std::{error::Error, net::SocketAddr, sync::Arc};

#[derive(Debug)]
pub enum ConnectionError {
    NotImplemented,
    InvalidTransition,
    InvalidVerifier,
    InvalidKey,
    ServerClosing,
    UnsupportedVersion,
    AuthenticationFailed,
    UnknownPacketType(i32),
}

impl Error for ConnectionError {}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Self::NotImplemented => write!(f, "This feature is not supported by this server."),
            Self::InvalidTransition => write!(f, "invalid transition"),
            Self::InvalidVerifier => write!(f, "invalid verifier"),
            Self::InvalidKey => write!(f, "invalid key"),
            Self::ServerClosing => write!(f, "server closing"),
            Self::UnsupportedVersion => write!(f, "client not supported"),
            Self::AuthenticationFailed => write!(f, "authentication failed"),
            Self::UnknownPacketType(packet_id) => write!(f, "unknown packet type: {}", packet_id),
        }
    }
}

pub struct Connection<R: Read + Unpin + Send + 'static, W: Write + Unpin + Send + 'static> {
    key: Box<InsecurePrivateKey>,
    state: ConnectionState,
    addr: SocketAddr,
    player_uuid: Option<Arc<str>>,
    player_name: Option<Arc<str>>,
    verify: Option<Vec<u8>>,
    reader: BinaryReader<R>,
    writer: BinaryWriter<W>,
    recv: Option<Receiver<ClientMessage>>,
    version: Option<i32>,
    controllers: Controllers,
}

impl<R: Read + Unpin + Send + 'static, W: Write + Unpin + Send + 'static> std::fmt::Display
    for Connection<R, W>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        let player_id: &str = if let Some(player_name) = &self.player_name {
            player_name.as_ref()
        } else if let Some(player_uuid) = &self.player_uuid {
            player_uuid.as_ref()
        } else {
            "*"
        };

        match &self.state {
            ConnectionState::Open => write!(f, "({}-{} new)", self.addr, player_id),
            ConnectionState::AwaitingStatusRequest => {
                write!(f, "({}-{} state)", self.addr, player_id)
            }
            ConnectionState::AwaitingLogin => write!(f, "({}-{} login)", self.addr, player_id),
            ConnectionState::AwaitingEncryptionResponse => {
                write!(f, "({}-{} encrypt)", self.addr, player_id)
            }
            ConnectionState::RunningGame => write!(f, "({}-{} running)", self.addr, player_id),
            ConnectionState::Terminate => write!(f, "({}-{} terminating)", self.addr, player_id),
        }
    }
}

impl<R: Read + Unpin + Send + 'static, W: Write + Unpin + Send + 'static> Connection<R, W> {
    pub fn new(
        reader: R,
        writer: W,
        addr: SocketAddr,
        key: InsecurePrivateKey,
        controllers: Controllers,
    ) -> Self {
        let writer = BinaryWriter::new(writer);
        let reader = BinaryReader::new(reader);
        Self {
            addr,
            reader,
            writer,
            controllers,
            state: ConnectionState::Open,
            key: Box::new(key),
            player_uuid: None,
            player_name: None,
            verify: None,
            recv: None,
            version: None,
        }
    }

    pub fn execute(mut self) {
        async_std::task::spawn(async move {
            let e = loop {
                let result = match self.state {
                    ConnectionState::Open => self.execute_open().await,
                    ConnectionState::AwaitingStatusRequest => self.execute_status_request().await,
                    ConnectionState::AwaitingLogin => self.execute_login().await,
                    ConnectionState::AwaitingEncryptionResponse => {
                        self.execute_encryption_response().await
                    }
                    ConnectionState::RunningGame => self.execute_game().await,
                    ConnectionState::Terminate => return,
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
        if let Ok(reason) = chat::trivial(&reason) {
            let reason = &reason;
            let _ = match self.state {
                ConnectionState::RunningGame => {
                    if let Some(player_uuid) = std::mem::replace(&mut self.player_uuid, None) {
                        self.controllers
                            .send_player(player::Message::ConnectionClosed { player_uuid })
                            .await;
                    }
                    let _ = self.writer.structure(&PlayResponse::Disconnect { reason });
                    self.writer.flush().await
                }
                _ => {
                    let _ = self.writer.structure(&LoginResponse::Disconnect { reason });
                    self.writer.flush().await
                }
            };
        }
    }

    async fn execute_open(&mut self) -> Result<(), Box<dyn Error>> {
        match self.reader.read_open().await? {
            OpenRequest::Handshake {
                address: _,
                port: _,
                version,
                next_state,
            } => match next_state {
                RequestedState::Login => {
                    trace!("{} request to transition to login state", self);
                    self.version = Some(version);
                    self.state = ConnectionState::AwaitingLogin;
                    Ok(())
                }
                RequestedState::Status => {
                    trace!("{} request to transition to status state", self);
                    self.state = ConnectionState::AwaitingStatusRequest;
                    Ok(())
                }
            },
            OpenRequest::HttpGet {} => {
                trace!("{} responding to HTTP probe", self);
                self.writer.structure(&OpenResponse::HttpOK {})?;
                self.writer.flush().await?;
                self.state = ConnectionState::Terminate;
                Ok(())
            }
            OpenRequest::Unknown { packet_id } => {
                Err(ConnectionError::UnknownPacketType(packet_id).into())
            }
        }
    }
    async fn execute_status_request(&mut self) -> Result<(), Box<dyn Error>> {
        match self.reader.read_status().await? {
            StatusRequest::InfoRequest => {
                trace!("{} request for server status", self);
                self.writer.structure(&StatusResponse::InfoResponse {
                    max_players: self.controllers.config().game().max_players(),
                    current_players: 0,
                    description: &self.controllers.config().network().motd(),
                })?;
                self.writer.flush().await?;
                Ok(())
            }
            StatusRequest::Ping { timestamp } => {
                trace!("{} request for ping", self);
                self.writer.structure(&StatusResponse::Pong { timestamp })?;
                self.writer.flush().await?;
                Ok(())
            }
            StatusRequest::Unknown { packet_id } => {
                Err(ConnectionError::UnknownPacketType(packet_id).into())
            }
        }
    }

    async fn execute_login(&mut self) -> Result<(), Box<dyn Error>> {
        match self.reader.read_login().await? {
            LoginRequest::Start { player_name } => {
                trace!("{} request to login as: {}", self, player_name);
                match self.version {
                    Some(racemus_binary::SERVER_VERSION_NUMBER) => {}
                    Some(_) => return Err(ConnectionError::UnsupportedVersion.into()),
                    None => return Err(ConnectionError::InvalidTransition.into()),
                };
                let mut verify = vec![0u8; 16];
                rand::thread_rng().fill_bytes(&mut verify);
                self.writer.structure(&LoginResponse::EncryptionRequest {
                    public_key: self.key.public_der(),
                    verify_token: &verify,
                })?;
                self.writer.flush().await?;
                self.player_uuid = Some(player_name);
                self.verify = Some(verify);
                self.state = ConnectionState::AwaitingEncryptionResponse;

                Ok(())
            }
            LoginRequest::Unknown { packet_id } => {
                Err(ConnectionError::UnknownPacketType(packet_id).into())
            }
            _ => Err(ConnectionError::InvalidTransition.into()),
        }
    }

    async fn execute_encryption_response(&mut self) -> Result<(), Box<dyn Error>> {
        match self.reader.read_login().await? {
            LoginRequest::EncryptionResponse {
                encrypted_shared_secret,
                encrypted_verifier,
            } => {
                trace!("{} encryption response received", self);
                let player_name = if let Some(player_name) = &self.player_uuid {
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

                let incoming_verify = self.key.decrypt(&encrypted_verifier);
                if incoming_verify.len() < verify.len() {
                    return Err(ConnectionError::InvalidVerifier.into());
                }

                if encrypted_shared_secret.len() < KEY_SIZE {
                    return Err(ConnectionError::InvalidKey.into());
                }

                let padding = incoming_verify.len() - verify.len();
                let incoming_verify = &incoming_verify[padding..];

                if !racemus_tools::crypto::equals_constant_time(verify, incoming_verify) {
                    return Err(ConnectionError::InvalidVerifier.into());
                }

                trace!("{} verifier validated", self);
                let key = self.key.decrypt(&encrypted_shared_secret);
                let padding = key.len() - KEY_SIZE;
                let key = &key[padding..];

                trace!("{} key decrypted", self);
                let player_info = match has_joined(
                    player_name,
                    b"" as &[u8],
                    &key,
                    self.key.public_der(),
                )
                .await
                {
                    Ok(r) => r,
                    Err(_) => return Err(ConnectionError::AuthenticationFailed.into()),
                };

                trace!(
                    "{} player info retrieved for {} with uuid {}",
                    self,
                    player_info.name(),
                    player_info.uuid()
                );

                let aes_out = racemus_binary::create_aes_cfb8(&key, &key)?;
                let aes_in = racemus_binary::create_aes_cfb8(&key, &key)?;

                self.writer.encrypt(aes_out);
                self.reader.decrypt(aes_in);

                if let Some(compression_threshold) =
                    self.controllers.config().network().compression_threshold()
                {
                    self.writer.structure(&LoginResponse::SetCompression {
                        compression_threshold,
                    })?;
                    self.writer.flush().await?;
                    self.reader.allow_compression();
                    self.writer
                        .allow_compression(compression_threshold as usize);
                }

                self.writer.structure(&LoginResponse::Success {
                    player_uuid: &player_info.uuid(),
                    player_name: &player_info.name(),
                })?;
                self.writer.flush().await?;

                info!(
                    "{} player {} connected with uuid {}",
                    self,
                    player_info.name(),
                    player_info.uuid()
                );

                let player_uuid: Arc<str> = player_info.uuid().into();
                let player_name: Arc<str> = player_info.name().into();

                self.player_uuid = Some(player_uuid.clone());
                self.player_name = Some(player_name.clone());
                self.state = ConnectionState::RunningGame;

                let (sender, rx) = async_std::sync::channel(10);
                self.recv = Some(rx);

                self.controllers
                    .send_player(player::Message::ConnectionOpened {
                        player_uuid,
                        player_name,
                        sender,
                    })
                    .await;

                Ok(())
            }
            LoginRequest::Unknown { packet_id } => {
                Err(ConnectionError::UnknownPacketType(packet_id).into())
            }
            _ => Err(ConnectionError::InvalidTransition.into()),
        }
    }
    async fn execute_game(&mut self) -> Result<(), Box<dyn Error>> {
        let recv = match &mut self.recv {
            None => return Err(ConnectionError::InvalidTransition.into()),
            Some(m) => m,
        };

        while self.state == ConnectionState::RunningGame {
            let message = match recv.recv().await {
                None => return Err(ConnectionError::ServerClosing.into()),
                Some(m) => m,
            };

            message.write(&mut self.writer).await?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    Open,

    // Status
    AwaitingStatusRequest,

    // Login
    AwaitingLogin,
    AwaitingEncryptionResponse,

    // Game
    RunningGame,

    // Close without responding
    Terminate,
}
