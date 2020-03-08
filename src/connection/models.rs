use crate::{connection::protocol::*, models::*};
use async_std::io::Write;
use std::{io::Error, marker::Unpin, sync::Arc};

#[derive(Debug)]
pub enum ClientMessage {
    JoinGame {
        entity_id: EntityId,
        game_mode: GameMode,
        dimension: i32,
        hashed_seed: u64,
        level_type: Arc<Box<str>>,
        view_distance: u8,
        reduce_debug: bool,
        enable_respawn_screen: bool,
    },
    PluginBrand {
        brand: &'static str,
    },
    ServerDifficulty {
        difficulty: Difficulty,
        difficulty_locked: bool,
    },
    HeldItemChange {
        slot: u8,
    },
    DeclareRecipes,
    DeclareTags,
    PlayerPositionAndLook {
        position: vek::Vec3<f64>,
        look: vek::Vec2<f32>,
        flags: u8,
        teleport_id: i32,
    },
    ChunkData {
        position: vek::Vec2<i32>
    }
}

impl ClientMessage {
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
                play::write_join_game(
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
            Self::PluginBrand { brand } => play::write_plugin_brand(writer, brand).await,
            Self::HeldItemChange { slot } => play::write_held_item_change(writer, *slot).await,
            Self::DeclareRecipes => play::write_declare_recipes(writer).await,
            Self::DeclareTags => play::write_declare_tags(writer).await,
            Self::ServerDifficulty {
                difficulty,
                difficulty_locked,
            } => play::write_server_difficulty(writer, *difficulty, *difficulty_locked).await,
            Self::PlayerPositionAndLook {
                position,
                look,
                flags,
                teleport_id,
            } => {
                play::write_player_position_and_look(writer, position, *look, *flags, *teleport_id)
                    .await
            },
            Self::ChunkData {
                position
            } => {
                play::write_chunk_data(writer, *position).await
            }
        }
    }
}
