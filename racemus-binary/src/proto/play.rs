use crate::{writer::StructuredWriter, BinaryWriter};
use async_std::io::Write;

#[derive(Debug, Clone, Copy)]
pub enum GameModeKind {
    Survival,
    Creative,
    Adventure,
    Spectator,
}

impl From<GameModeKind> for u8 {
    fn from(value: GameModeKind) -> Self {
        match value {
            GameModeKind::Survival => 0x0,
            GameModeKind::Creative => 0x1,
            GameModeKind::Adventure => 0x2,
            GameModeKind::Spectator => 0x3,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum GameMode {
    Softcore(GameModeKind),
    Hardcore(GameModeKind),
}

impl From<GameMode> for u8 {
    fn from(value: GameMode) -> Self {
        match value {
            GameMode::Hardcore(m) => {
                let sub: u8 = m.into();
                0x8 | sub
            }
            GameMode::Softcore(m) => m.into(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Difficulty {
    Peaceful,
    Easy,
    Medium,
    Hard,
}

impl From<Difficulty> for u8 {
    fn from(value: Difficulty) -> Self {
        match value {
            Difficulty::Peaceful => 0x0,
            Difficulty::Easy => 0x1,
            Difficulty::Medium => 0x2,
            Difficulty::Hard => 0x3,
        }
    }
}

pub enum PlayResponse<'a> {
    ServerDifficulty {
        // 0x0e
        difficulty: Difficulty,
        difficulty_locked: bool,
    },
    Plugin {
        // 0x19
        channel: &'a str,
        data: &'a [u8],
    },
    Disconnect {
        // 0x1b
        reason: &'a str,
    },
    JoinGame {
        // 0x26
        entity_id: u32,
        game_mode: GameMode,
        dimension: i32,
        hashed_seed: u64,
        level_type: &'a str,
        view_distance: u8,
        reduce_debug: bool,
        enable_respawn_screen: bool,
    },
    PlayerPositionAndLook {
        // 0x36
        position: [f64; 3],
        look: [f32; 2],
        flags: u8,
        teleport_id: i32,
    },
    HeldItemChange {
        // 0x40
        slot: u8,
    },
}

impl<'a, W: Write + Unpin> StructuredWriter<W, PlayResponse<'a>> for BinaryWriter<W> {
    fn structure(&mut self, val: &PlayResponse<'a>) -> Result<&mut Self, crate::Error> {
        let insertion = self.create_insertion();
        match val {
            PlayResponse::ServerDifficulty {
                difficulty,
                difficulty_locked,
            } => self
                .var_i32(0x0e)?
                .fix_u8((*difficulty).into())?
                .fix_bool(*difficulty_locked)?,
            PlayResponse::Plugin { channel, data } => {
                self.var_i32(0x19)?.arr_char(channel)?.arr_u8(data)?
            }
            PlayResponse::Disconnect { reason } => self.var_i32(0x1b)?.arr_char(reason)?,
            PlayResponse::JoinGame {
                entity_id,
                game_mode,
                dimension,
                hashed_seed,
                level_type,
                view_distance,
                reduce_debug,
                enable_respawn_screen,
            } => self
                .var_i32(0x26)?
                .fix_i32(*entity_id as i32)?
                .fix_u8((*game_mode).into())?
                .fix_i32(*dimension)?
                .fix_u64(*hashed_seed)?
                .fix_u8(0)? // Max players, no longer supported
                .arr_char(level_type)?
                .var_i32(*view_distance as i32)?
                .fix_bool(*reduce_debug)?
                .fix_bool(*enable_respawn_screen)?,
            PlayResponse::PlayerPositionAndLook {
                position,
                look,
                flags,
                teleport_id,
            } => self
                .var_i32(0x36)?
                .fix_f64(position[0])?
                .fix_f64(position[1])?
                .fix_f64(position[2])?
                .fix_f32(look[0])?
                .fix_f32(look[1])?
                .fix_u8(*flags)?
                .var_i32(*teleport_id)?,
            PlayResponse::HeldItemChange { slot } => self.var_i32(0x40)?.fix_u8(*slot)?,
        }
        .insert_len_var_i32(insertion)
    }
}

#[cfg(test)]
mod tests {
    use super::{PlayResponse::*, *};
    use crate::{tests::*, Error};

    macro_rules! raw_write_tests {
        ($($name:ident: $writer:ident => $expr:expr, $expected:expr),*) => {
            $(
                #[test]
                fn $name() -> Result<(), Error> {
                    let mut $writer = make_writer();
                    $expr;
                    let buf = make_buffer($writer);
                    assert_eq!(buf, $expected);
                    Ok(())
                }
            )*
        }
    }

    raw_write_tests!(
        test_write_play_join_game: w => w.structure(&JoinGame{
            entity_id: 0x1526_3749,
            game_mode: GameMode::Hardcore(GameModeKind::Adventure),
            dimension: -1,
            hashed_seed: 0x1526_3749_5015_2637,
            level_type: "default",
            view_distance: 28,
            reduce_debug: true,
            enable_respawn_screen: false,
        })?, b"\x1e\x26\x15\x26\x37\x49\x0a\xff\xff\xff\xff\x15\x26\x37\x49\x50\x15\x26\x37\x00\x07default\x1c\x01\x00",
        test_write_held_item_change: w => w.structure(&HeldItemChange{
            slot: 0x48
        })?, b"\x02\x40\x48",
        test_write_plugin: w => w.structure(&Plugin{
            channel: "brand",
            data: b"1234"
        })?, b"\x0c\x19\x05brand\x041234",
        test_write_server_difficulty: w => w.structure(&ServerDifficulty{
            difficulty: Difficulty::Medium,
            difficulty_locked: true
        })?, b"\x03\x0e\x02\x01",
        test_write_disconnect: w => w.structure(&Disconnect{
            reason: "kicked"
        })?, b"\x08\x1b\x06kicked"
    );
}
