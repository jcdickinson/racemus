use crate::{
    connection::protocol::*,
    models::*
};
use async_std::io::Write;
use std::{
    io::Error,
    marker::Unpin,
    sync::Arc
};

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
            Self::HeldItemChange { slot } => play::write_held_item_change(writer, *slot).await,
            Self::DeclareRecipes => play::declare_recipes(writer).await,
            Self::DeclareTags => play::declare_tags(writer).await,
            Self::PlayerPositionAndLook {
                position,
                look,
                flags,
                teleport_id,
            } => {
                play::player_position_and_look(
                    writer,
                    position,
                    *look,
                    *flags,
                    *teleport_id,
                )
                .await
            }
        }
    }
}

