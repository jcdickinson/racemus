#![allow(clippy::too_many_arguments)]

use super::{PacketWriter};
use async_std::io::{Write};
use std::{
    io::{Error},
    marker::Unpin
};
use crate::models::*;

#[derive(Debug, PartialEq, Eq)]
pub enum Packet {
   
}

pub async fn write_join_game<W: Write + Unpin>(
    writer: &mut PacketWriter<W>,
    entity_id: EntityId,
    game_mode: GameMode,
    dimension: i32,
    hashed_seed: u64,
    level_type: &str,
    view_distance: u8,
    reduce_debug: bool,
    enable_respawn_screen: bool,
) -> Result<(), Error> {
    let (bit, mode) = match game_mode {
        GameMode::Hardcore(m) => (0x8u8, m),
        GameMode::Softcore(m) => (0x0, m),
    };

    let mode = match mode {
        GameModeKind::Survival => bit,
        GameModeKind::Creative => 0x1 | bit,
        GameModeKind::Adventure => 0x2 | bit,
        GameModeKind::Spectator => 0x3 | bit,
    };

    let eid: u32 = entity_id.into();
    writer
        .packet_id(0x26)
        .fix_i32(eid as i32)
        .fix_u8(mode)
        .fix_i32(dimension)
        .fix_u64(hashed_seed)
        .fix_u8(0) // Max players, ignored by MC client
        .arr_char(level_type)
        .var_i32(view_distance as i32)
        .fix_bool(reduce_debug)
        .fix_bool(enable_respawn_screen)
        .flush()
        .await
}


pub async fn write_held_item_change<W: Write + Unpin>(
    writer: &mut PacketWriter<W>,
    slot: u8
) -> Result<(), Error> {
    writer
        .packet_id(0x40)
        .fix_u8(slot)
        .flush()
        .await
}

pub async fn declare_recipes<W: Write + Unpin>(
    writer: &mut PacketWriter<W>
) -> Result<(), Error> {
    writer
        .packet_id(0x5B)
        .fix_i32(0)
        .flush()
        .await
}

pub async fn declare_tags<W: Write + Unpin>(
    writer: &mut PacketWriter<W>
) -> Result<(), Error> {
    writer
        .packet_id(0x5C)
        .fix_i32(0)
        .fix_i32(0)
        .fix_i32(0)
        .fix_i32(0)
        .flush()
        .await
}

pub async fn player_position_and_look<W: Write + Unpin>(
    writer: &mut PacketWriter<W>,
    position: &vek::Vec3<f64>,
    look: vek::Vec2<f32>,
    flags: u8,
    teleport_id: i32
) -> Result<(), Error> {
    writer
        .packet_id(0x36)
        .fix_f64(position.x)
        .fix_f64(position.y)
        .fix_f64(position.z)
        .fix_f32(look.x)
        .fix_f32(look.y)
        .fix_u8(flags)
        .var_i32(teleport_id)
        .flush()
        .await
}