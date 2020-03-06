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

#[derive(Debug)]
pub struct WorldInfo {
    game_mode: GameMode,
    hashed_seed: u64,
    level_type: String,
    view_distance: u8,
    reduce_debug: bool,
    enable_respawn_screen: bool,
}

impl WorldInfo {
    pub fn new(
        game_mode: GameMode,
        hashed_seed: u64,
        level_type: String,
        view_distance: u8,
        reduce_debug: bool,
        enable_respawn_screen: bool,
    ) -> Self {
        Self {
            game_mode,
            hashed_seed,
            level_type,
            view_distance,
            reduce_debug,
            enable_respawn_screen,
        }
    }
    pub fn game_mode(&self) -> GameMode {
        self.game_mode
    }
    pub fn hashed_seed(&self) -> u64 {
        self.hashed_seed
    }
    pub fn level_type(&self) -> &str {
        &self.level_type
    }
    pub fn view_distance(&self) -> u8 {
        self.view_distance
    }
    pub fn reduce_debug(&self) -> bool {
        self.reduce_debug
    }
    pub fn enable_respawn_screen(&self) -> bool {
        self.enable_respawn_screen
    }
}
