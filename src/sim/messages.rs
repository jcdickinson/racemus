use crate::protocol::*;
use std::io::Error;
use std::marker::Unpin;
use async_std::io::Write;
use async_std::sync::{ Sender };

pub enum ClientMessages {
    JoinGame(JoinGame),
}

#[derive(Clone, Copy)]
pub enum GameModeKind {
    Survival,
    Creative,
    Adventure,
    Spectator,
}

#[derive(Clone, Copy)]
pub enum GameMode {
    Softcore(GameModeKind),
    Hardcore(GameModeKind),
}

pub struct JoinGame {
    pub(super) entity_id: i32,
    pub(super) game_mode: GameMode,
    pub(super) dimension: i32,
    pub(super) hashed_seed: u64,
    pub(super) level_type: String,
    pub(super) view_distance: i32,
    pub(super) reduce_debug: bool,
    pub(super) enable_respawn_screen: bool,
}

impl JoinGame {
    pub async fn write<W: Write + Unpin>(&self, writer: &mut PacketWriter<W>) -> Result<(), Error> {
        login::write_join_game(
            writer,
            self.entity_id,
            self.game_mode,
            self.dimension,
            self.hashed_seed,
            &self.level_type,
            self.view_distance,
            self.reduce_debug,
            self.enable_respawn_screen,
        )
        .await
    }
}

pub enum SimMessages {
    Accept(Sender<ClientMessages>),
}
