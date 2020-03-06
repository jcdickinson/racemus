#![allow(clippy::too_many_arguments)]

use super::{PacketWriter};
use async_std::io::{Write};
use std::io::{Error};
use std::marker::Unpin;

#[derive(Debug, PartialEq, Eq)]
pub enum Packet {
   
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
    x: f64,
    y: f64,
    z: f64,
    yaw: f32,
    pitch: f32,
    flags: u8,
    teleport_id: i32
) -> Result<(), Error> {
    writer
        .packet_id(0x36)
        .fix_f64(x)
        .fix_f64(y)
        .fix_f64(z)
        .fix_f32(yaw)
        .fix_f32(pitch)
        .fix_u8(flags)
        .var_i32(teleport_id)
        .flush()
        .await
}