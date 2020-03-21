mod chunk;
pub use chunk::*;
use racemus_proto::minecraft as proto;

#[derive(Debug, Clone, Copy)]
pub enum Difficulty {
    Peaceful,
    Easy,
    Medium,
    Hard,
}

impl From<proto::Difficulty> for Difficulty {
    fn from(val: proto::Difficulty) -> Self {
        match val {
            proto::Difficulty::Peaceful => Self::Peaceful,
            proto::Difficulty::Easy => Self::Easy,
            proto::Difficulty::Medium => Self::Medium,
            proto::Difficulty::Hard => Self::Hard,
        }
    }
}

impl From<Difficulty> for proto::Difficulty {
    fn from(val: Difficulty) -> Self {
        match val {
            Difficulty::Peaceful => Self::Peaceful,
            Difficulty::Easy => Self::Easy,
            Difficulty::Medium => Self::Medium,
            Difficulty::Hard => Self::Hard,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum GameModeKind {
    Survival,
    Creative,
    Adventure,
    Spectator,
}

impl From<proto::GameModeKind> for GameModeKind {
    fn from(val: proto::GameModeKind) -> Self {
        match val {
            proto::GameModeKind::Survival => Self::Survival,
            proto::GameModeKind::Creative => Self::Creative,
            proto::GameModeKind::Adventure => Self::Adventure,
            proto::GameModeKind::Spectator => Self::Spectator,
        }
    }
}

impl From<GameModeKind> for proto::GameModeKind {
    fn from(val: GameModeKind) -> Self {
        match val {
            GameModeKind::Survival => Self::Survival,
            GameModeKind::Creative => Self::Creative,
            GameModeKind::Adventure => Self::Adventure,
            GameModeKind::Spectator => Self::Spectator,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum GameMode {
    Softcore(GameModeKind),
    Hardcore(GameModeKind),
}

impl From<proto::GameMode> for GameMode {
    fn from(val: proto::GameMode) -> Self {
        match val {
            proto::GameMode::Softcore(kind) => Self::Softcore(kind.into()),
            proto::GameMode::Hardcore(kind) => Self::Hardcore(kind.into()),
        }
    }
}

impl From<GameMode> for proto::GameMode {
    fn from(val: GameMode) -> Self {
        match val {
            GameMode::Softcore(kind) => Self::Softcore(kind.into()),
            GameMode::Hardcore(kind) => Self::Hardcore(kind.into()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, PartialOrd, Ord)]
pub struct EntityId(u32);

impl From<u32> for EntityId {
    fn from(val: u32) -> Self {
        Self(val)
    }
}

impl From<EntityId> for u32 {
    fn from(val: EntityId) -> Self {
        val.0
    }
}
