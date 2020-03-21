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
