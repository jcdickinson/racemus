#![allow(clippy::too_many_arguments)]

use super::{PacketReader, PacketWriter};
use std::io::{Error, ErrorKind};
use std::marker::Unpin;
use tokio::io::{AsyncRead, AsyncWrite};
use crate::sim;

#[derive(Debug, PartialEq, Eq)]
pub enum Packet {
    LoginStart(LoginStart),
    EncryptionResponse(EncryptionResponse),
}

#[derive(Debug, PartialEq, Eq)]
pub struct LoginStart {
    player_name: String,
}

impl LoginStart {
    pub fn player_name(&self) -> &str {
        &self.player_name
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestedState {
    Status,
    Login,
}

#[derive(Debug, PartialEq, Eq)]
pub struct EncryptionResponse {
    encrypted_shared_secret: Vec<u8>,
    encrypted_verifier: Vec<u8>,
}

impl EncryptionResponse {
    pub fn encrypted_shared_secret(&self) -> &[u8] {
        &self.encrypted_shared_secret
    }
    pub fn encrypted_verifier(&self) -> &[u8] {
        &self.encrypted_verifier
    }
}

pub async fn read_packet<R: AsyncRead + Unpin>(
    reader: &mut PacketReader<R>,
) -> Result<Packet, Error> {
    match reader.packet_header().await? {
        0x00 => {
            let player_name = reader.arr_char(Some(16)).await?;
            Ok(Packet::LoginStart(LoginStart { player_name }))
        }
        0x01 => {
            let encrypted_shared_secret = reader.arr_u8(Some(128)).await?;
            let encrypted_verifier = reader.arr_u8(Some(128)).await?;
            Ok(Packet::EncryptionResponse(EncryptionResponse {
                encrypted_shared_secret,
                encrypted_verifier,
            }))
        }
        _ => Err(ErrorKind::InvalidData.into()),
    }
}

pub async fn write_encryption_request<W: AsyncWrite + Unpin>(
    writer: &mut PacketWriter<W>,
    public_key: &[u8],
    verify_token: &[u8],
) -> Result<(), Error> {
    writer.packet_id(0x01)
        .var_i32(0) // Server ID
        .arr_u8(public_key)
        .arr_u8(verify_token)
        .flush().await
}

pub async fn write_login_success<W: AsyncWrite + Unpin>(
    writer: &mut PacketWriter<W>,
    uuid: &str,
    player_name: &str,
) -> Result<(), Error> {
    writer.packet_id(0x02)
        .arr_char(uuid)
        .arr_char(player_name)
        .flush().await
}

pub async fn write_join_game<W: AsyncWrite + Unpin>(
    writer: &mut PacketWriter<W>,
    entity_id: i32,
    game_mode: sim::GameMode,
    dimension: i32,
    hashed_seed: u64,
    level_type: &str,
    view_distance: i32,
    reduce_debug: bool,
    enable_respawn_screen: bool
) -> Result<(), Error> {
    let (bit, mode) = match game_mode {
        sim::GameMode::Hardcore(m) => (0x8u8, m),
        sim::GameMode::Softcore(m) => (0x0, m)
    };

    let mode = match mode {
        sim::GameModeKind::Survival => bit,
        sim::GameModeKind::Creative => 0x1 | bit,
        sim::GameModeKind::Adventure => 0x2 | bit,
        sim::GameModeKind::Spectator => 0x3 | bit
    };

    writer.packet_id(0x26)
        .fix_i32(entity_id)
        .fix_u8(mode)
        .fix_i32(dimension)
        .fix_u64(hashed_seed)
        .arr_char(level_type)
        .var_i32(view_distance)
        .fix_bool(reduce_debug)
        .fix_bool(enable_respawn_screen)
        .flush().await
}