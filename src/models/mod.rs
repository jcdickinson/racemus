#[derive(Debug, Clone, Copy)]
pub enum GameModeKind {
    Survival,
    Creative,
    Adventure,
    Spectator,
}

#[derive(Debug, Clone, Copy)]
pub enum Difficulty {
    Peaceful,
    Easy,
    Medium,
    Hard,
}

#[derive(Debug, Clone, Copy)]
pub enum GameMode {
    Softcore(GameModeKind),
    Hardcore(GameModeKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntityId(u32);

impl Default for EntityId {
    fn default() -> Self {
        Self(0)
    }
}

impl From<u32> for EntityId {
    fn from(v: u32) -> Self {
        Self(v)
    }
}

impl From<EntityId> for u32 {
    fn from(v: EntityId) -> Self {
        v.0
    }
}
