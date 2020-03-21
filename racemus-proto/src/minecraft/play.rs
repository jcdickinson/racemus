#![allow(clippy::too_many_arguments)]

use crate::{
    minecraft::{Difficulty, GameMode, GameModeKind},
    PacketWriter,
};
use async_std::io::Write;
use std::{io::Error, marker::Unpin};

#[derive(Debug, PartialEq, Eq)]
pub enum Packet {}

pub async fn write_join_game<W: Write + Unpin>(
    writer: &mut PacketWriter<W>,
    entity_id: u32,
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

    writer
        .packet_id(0x26)
        .fix_i32(entity_id as i32)
        .fix_u8(mode)
        .fix_i32(dimension)
        .fix_u64(hashed_seed)
        .fix_u8(0) // Max players, ignored by MC client
        .var_arr_char(level_type)
        .var_i32(view_distance as i32)
        .fix_bool(reduce_debug)
        .fix_bool(enable_respawn_screen)
        .flush_length_prefixed()
        .await
}

pub async fn write_held_item_change<W: Write + Unpin>(
    writer: &mut PacketWriter<W>,
    slot: u8,
) -> Result<(), Error> {
    writer.packet_id(0x40).fix_u8(slot).flush_length_prefixed().await
}

pub async fn write_declare_recipes<W: Write + Unpin>(
    writer: &mut PacketWriter<W>,
) -> Result<(), Error> {
    writer.packet_id(0x5B).fix_i32(0).flush_length_prefixed().await
}

pub async fn write_declare_tags<W: Write + Unpin>(
    writer: &mut PacketWriter<W>,
) -> Result<(), Error> {
    writer
        .packet_id(0x5C)
        .fix_i32(0)
        .fix_i32(0)
        .fix_i32(0)
        .fix_i32(0)
        .flush_length_prefixed()
        .await
}

pub async fn write_player_position_and_look<W: Write + Unpin>(
    writer: &mut PacketWriter<W>,
    position: &[f64; 3],
    look: &[f32; 2],
    flags: u8,
    teleport_id: i32,
) -> Result<(), Error> {
    writer
        .packet_id(0x36)
        .fix_f64(position[0])
        .fix_f64(position[1])
        .fix_f64(position[2])
        .fix_f32(look[0])
        .fix_f32(look[1])
        .fix_u8(flags)
        .var_i32(teleport_id)
        .flush_length_prefixed()
        .await
}

pub async fn write_plugin_brand<W: Write + Unpin>(
    writer: &mut PacketWriter<W>,
    brand: &str,
) -> Result<(), Error> {
    writer
        .packet_id(0x19)
        .var_arr_char("brand")
        .var_arr_char(brand)
        .flush_length_prefixed()
        .await
}

pub async fn write_server_difficulty<W: Write + Unpin>(
    writer: &mut PacketWriter<W>,
    difficulty: Difficulty,
    difficulty_locked: bool,
) -> Result<(), Error> {
    let difficulty = match difficulty {
        Difficulty::Peaceful => 0,
        Difficulty::Easy => 1,
        Difficulty::Medium => 2,
        Difficulty::Hard => 3,
    };

    writer
        .packet_id(0x0E)
        .fix_u8(difficulty)
        .fix_bool(difficulty_locked)
        .flush_length_prefixed()
        .await
}

pub async fn write_disconnect<W: Write + Unpin>(
    writer: &mut PacketWriter<W>,
    reason: &str,
) -> Result<(), std::io::Error> {
    writer.packet_id(0x1b).var_arr_char(reason).flush_length_prefixed().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_std::io::Cursor;
    use async_std::task::block_on;

    macro_rules! write_tests {
        ($($name:ident: $fn:ident( $($param:expr),* ), $expected:expr),*) => {
            $(
                #[test]
                fn $name() {
                    let target = Cursor::new(Vec::<u8>::new());
                    let mut writer = PacketWriter::new(target);
                    block_on(
                        $fn(
                            &mut writer,
                           $(
                            $param
                           ),*
                        )
                    ).unwrap();
                    assert_eq!(
                        writer.into_inner().into_inner(),
                        $expected as &[u8]
                    );
                }
            )*
        }
    }

    write_tests! {
        write_disconnect_test: write_disconnect("bad?"), b"\x06\x1b\x04bad?" as &[u8]
    }
}
