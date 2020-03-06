use super::protocol::*;
use crate::sim;
use async_std::io::Write;
use async_std::sync::Sender;
use std::io::Error;
use std::marker::Unpin;

#[derive(Debug)]
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
    HeldItemChange {
        slot: u8,
    },
    DeclareRecipes,
    DeclareTags,
    PlayerPositionAndLook {
        x: f64,
        y: f64,
        z: f64,
        yaw: f32,
        pitch: f32,
        flags: u8,
        teleport_id: i32,
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
            Self::HeldItemChange { slot } => play::write_held_item_change(writer, *slot).await,
            Self::DeclareRecipes => play::declare_recipes(writer).await,
            Self::DeclareTags => play::declare_tags(writer).await,
            Self::PlayerPositionAndLook {
                x,
                y,
                z,
                yaw,
                pitch,
                flags,
                teleport_id,
            } => {
                play::player_position_and_look(
                    writer,
                    *x,
                    *y,
                    *z,
                    *yaw,
                    *pitch,
                    *flags,
                    *teleport_id,
                )
                .await
            }
        }
    }
}

pub enum ServerMessages {
    Accept(Sender<ClientMessages>),
}
