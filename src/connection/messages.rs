use super::protocol::*;
use crate::sim;
use async_std::io::Write;
use async_std::sync::Sender;
use std::io::Error;
use std::marker::Unpin;

pub enum ClientMessages {
    JoinGame {
        entity_id: i32,
        game_mode: sim::GameMode,
        dimension: i32,
        hashed_seed: u64,
        level_type: String,
        view_distance: i32,
        reduce_debug: bool,
        enable_respawn_screen: bool,
    },
}

impl ClientMessages {
    pub async fn write<W: Write + Unpin>(&self, writer: &mut PacketWriter<W>) -> Result<(), Error> {
        match self {
            Self::JoinGame {
                entity_id,
                game_mode,
                dimension,
                hashed_seed,
                level_type,
                view_distance,
                reduce_debug,
                enable_respawn_screen,
            } => {
                login::write_join_game(
                    writer,
                    *entity_id,
                    *game_mode,
                    *dimension,
                    *hashed_seed,
                    level_type,
                    *view_distance,
                    *reduce_debug,
                    *enable_respawn_screen,
                )
                .await
            }
        }
    }
}

pub enum ServerMessages {
    Accept(Sender<ClientMessages>),
}
