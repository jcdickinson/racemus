use crate::models::*;
use async_std::io::Write;
use racemus_binary::{proto::*, BinaryWriter, *};
use std::{marker::Unpin, sync::Arc};

#[derive(Debug)]
pub enum ClientMessage {
    JoinGame {
        entity_id: EntityId,
        game_mode: crate::models::GameMode,
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
        difficulty: crate::models::Difficulty,
        difficulty_locked: bool,
    },
    HeldItemChange {
        slot: u8,
    },
    PlayerPositionAndLook {
        position: vek::Vec3<f64>,
        look: vek::Vec2<f32>,
        flags: u8,
        teleport_id: i32,
    },
    ChunkData {
        position: vek::Vec2<i32>,
    },
}

impl ClientMessage {
    pub async fn write<W: Write + Unpin>(&self, writer: &mut BinaryWriter<W>) -> Result<(), Error> {
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
                writer.structure(&PlayResponse::JoinGame {
                    entity_id: (*entity_id).into(),
                    game_mode: (*game_mode).into(),
                    dimension: *dimension,
                    hashed_seed: *hashed_seed,
                    level_type,
                    view_distance: *view_distance,
                    reduce_debug: *reduce_debug,
                    enable_respawn_screen: *enable_respawn_screen,
                })?;
                writer.flush().await
            }
            Self::PluginBrand { brand } => {
                writer.structure(&PlayResponse::Plugin {
                    channel: "brand",
                    data: brand.as_bytes(),
                })?;
                writer.flush().await
            }
            Self::HeldItemChange { slot } => {
                writer.structure(&PlayResponse::HeldItemChange { slot: *slot })?;
                writer.flush().await
            }
            Self::ServerDifficulty {
                difficulty,
                difficulty_locked,
            } => {
                writer.structure(&PlayResponse::ServerDifficulty {
                    difficulty: (*difficulty).into(),
                    difficulty_locked: *difficulty_locked,
                })?;
                writer.flush().await
            }
            Self::PlayerPositionAndLook {
                position,
                look,
                flags,
                teleport_id,
            } => {
                writer.structure(&PlayResponse::PlayerPositionAndLook {
                    position: [position.x, position.y, position.z],
                    look: [look.x, look.y],
                    flags: *flags,
                    teleport_id: *teleport_id,
                })?;
                writer.flush().await
            }
            Self::ChunkData { position: _ } => Ok(()),
        }
    }
}
