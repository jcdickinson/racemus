use async_std::prelude::*;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde_derive::Deserialize;
use std::convert::TryFrom;
use std::error::Error;

#[derive(Debug)]
pub enum ConfigError {
    InvalidValue(String),
}

impl Error for ConfigError {}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Self::InvalidValue(path) => {
                write!(f, "The value {} in the configuration file is invalid", path)
            }
        }
    }
}

#[derive(Deserialize)]
struct RawConfig {
    #[serde(rename = "network", default = "network_default")]
    network: RawNetworkConfig,
    #[serde(rename = "security", default = "security_default")]
    security: RawSecurityConfig,
    #[serde(rename = "game", default = "game_default")]
    game: RawGameConfig,
}

impl RawConfig {
    pub async fn read(file_name: &str) -> Result<Self, Box<dyn Error>> {
        let mut file = async_std::fs::File::open(file_name).await?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).await?;
        match toml::from_slice::<Self>(&contents) {
            Ok(r) => Ok(r),
            Err(e) => Err(e.into()),
        }
    }
}

#[derive(Deserialize)]
struct RawNetworkConfig {
    #[serde(rename = "ip", default = "ip_default")]
    ip: String,
    #[serde(rename = "port", default = "port_default")]
    port: u16,
    #[serde(rename = "motd", default = "motd_default")]
    motd: String,
}

fn network_default() -> RawNetworkConfig {
    RawNetworkConfig {
        ip: ip_default(),
        port: port_default(),
        motd: motd_default(),
    }
}

fn ip_default() -> String {
    "0.0.0.0".to_string()
}

fn port_default() -> u16 {
    25565
}

fn motd_default() -> String {
    "A Minecraft Server".to_string()
}

#[derive(Deserialize)]
struct RawSecurityConfig {
    #[serde(rename = "private-key", default = "private_key_default")]
    private_key: String,
    #[serde(rename = "public-key", default = "public_key_default")]
    public_key: String,
}

fn security_default() -> RawSecurityConfig {
    RawSecurityConfig {
        private_key: private_key_default(),
        public_key: public_key_default(),
    }
}

fn private_key_default() -> String {
    "server_rsa".to_string()
}

fn public_key_default() -> String {
    "server_rsa.pub".to_string()
}

#[derive(Deserialize)]
struct RawGameConfig {
    #[serde(rename = "seed", default = "seed_default")]
    seed: String,
    #[serde(rename = "game-mode", default = "game_mode_default")]
    game_mode: u8,
    #[serde(rename = "difficulty", default = "difficulty_default")]
    difficulty: u8,
    #[serde(rename = "hardcore", default = "hardcore_default")]
    hardcore: bool,
    #[serde(rename = "view-distance", default = "view_distance_default")]
    view_distance: u8,
    #[serde(rename = "max-players", default = "max_players_default")]
    max_players: u16,
    #[serde(rename = "reduce-debug-info", default = "reduce_debug_info_default")]
    reduce_debug_info: bool,
    #[serde(rename = "enable-respawn-screen", default = "enable_respawn_screen_default")]
    enable_respawn_screen: bool,
}

fn game_default() -> RawGameConfig {
    RawGameConfig {
        seed: seed_default(),
        game_mode: game_mode_default(),
        difficulty: difficulty_default(),
        hardcore: hardcore_default(),
        view_distance: view_distance_default(),
        max_players: max_players_default(),
        reduce_debug_info: reduce_debug_info_default(),
        enable_respawn_screen: reduce_debug_info_default()
    }
}

fn seed_default() -> String {
    //thread_rng().sa
    thread_rng().sample_iter(&Alphanumeric).take(20).collect()
}

fn game_mode_default() -> u8 {
    0
}

fn difficulty_default() -> u8 {
    1
}
fn hardcore_default() -> bool {
    false
}
fn view_distance_default() -> u8 {
    10
}
fn max_players_default() -> u16 {
    20
}
fn reduce_debug_info_default() -> bool {
    false
}
fn enable_respawn_screen_default() -> bool {
    true
}

pub struct Config {
    network: NetworkConfig,
    security: SecurityConfig,
    game: GameConfig,
}

impl<'a> Config {
    pub async fn read(file_name: &str) -> Result<Self, Box<dyn Error>> {
        let raw = RawConfig::read(file_name).await?;
        Config::try_from(raw)
    }

    pub fn network(&'a self) -> &'a NetworkConfig {
        &self.network
    }
    pub fn security(&'a self) -> &'a SecurityConfig {
        &self.security
    }
    pub fn game(&'a self) -> &'a GameConfig {
        &self.game
    }
}

impl TryFrom<RawConfig> for Config {
    type Error = Box<dyn Error>;

    fn try_from(value: RawConfig) -> Result<Self, Self::Error> {
        let network = match NetworkConfig::try_from(value.network) {
            Ok(r) => r,
            Err(e) => return Err(e),
        };
        let security = match SecurityConfig::try_from(value.security) {
            Ok(r) => r,
            Err(e) => return Err(e),
        };
        let game = match GameConfig::try_from(value.game) {
            Ok(r) => r,
            Err(e) => return Err(e),
        };

        Ok(Self {
            network,
            security,
            game,
        })
    }
}

pub struct NetworkConfig {
    addr: std::net::SocketAddr,
    motd: String,
}

impl<'a> NetworkConfig {
    pub fn addr(&self) -> &std::net::SocketAddr {
        &self.addr
    }
    pub fn motd(&self) -> &str {
        &self.motd
    }
}

impl TryFrom<RawNetworkConfig> for NetworkConfig {
    type Error = Box<dyn std::error::Error>;

    fn try_from(value: RawNetworkConfig) -> Result<Self, Self::Error> {
        let addr: std::net::IpAddr = match value.ip.parse() {
            Ok(r) => r,
            Err(e) => return Err(e.into()),
        };
        let addr = std::net::SocketAddr::new(addr, value.port);
        let motd = value.motd;
        Ok(Self { addr, motd })
    }
}

pub struct SecurityConfig {
    private_key: String,
    public_key: String,
}

impl TryFrom<RawSecurityConfig> for SecurityConfig {
    type Error = Box<dyn Error>;

    fn try_from(value: RawSecurityConfig) -> Result<Self, Self::Error> {
        let private_key = value.private_key;
        let public_key = value.public_key;
        Ok(Self {
            private_key,
            public_key,
        })
    }
}

impl SecurityConfig {
    pub fn private_key(&self) -> &str {
        &self.private_key
    }
    pub fn public_key(&self) -> &str {
        &self.public_key
    }
}

pub struct GameConfig {
    seed: u64,
    game_mode: crate::sim::GameMode,
    difficulty: crate::sim::Difficulty,
    view_distance: u8,
    max_players: u16,
    reduce_debug_info: bool,
    enable_respawn_screen: bool,
}

impl GameConfig {
    pub fn seed(&self) -> u64 {
        self.seed
    }
    pub fn game_mode(&self) -> crate::sim::GameMode {
        self.game_mode
    }
    pub fn difficulty(&self) -> crate::sim::Difficulty {
        self.difficulty
    }
    pub fn view_distance(&self) -> u8 {
        self.view_distance
    }
    pub fn max_players(&self) -> u16 {
        self.max_players
    }
    pub fn reduce_debug_info(&self) -> bool {
        self.reduce_debug_info
    }
    pub fn enable_respawn_screen(&self) -> bool {
        self.enable_respawn_screen
    }
}

impl TryFrom<RawGameConfig> for GameConfig {
    type Error = Box<dyn Error>;

    fn try_from(value: RawGameConfig) -> Result<Self, Self::Error> {
        let mut ctx = ring::digest::Context::new(&ring::digest::SHA1_FOR_LEGACY_USE_ONLY);
        ctx.update(value.seed.as_bytes());
        let digest = ctx.finish();
        let mut copy = [0u8; 8];
        copy.copy_from_slice(&digest.as_ref()[0..8]);
        let seed = u64::from_ne_bytes(copy);

        let kind = match value.game_mode {
            0 => crate::sim::GameModeKind::Survival,
            1 => crate::sim::GameModeKind::Creative,
            2 => crate::sim::GameModeKind::Adventure,
            3 => crate::sim::GameModeKind::Spectator,
            _ => return Err(ConfigError::InvalidValue("game.game-mode".to_string()).into()),
        };

        let game_mode = if value.hardcore {
            crate::sim::GameMode::Hardcore(kind)
        } else {
            crate::sim::GameMode::Softcore(kind)
        };

        let difficulty = match value.difficulty {
            0 => crate::sim::Difficulty::Peaceful,
            1 => crate::sim::Difficulty::Easy,
            2 => crate::sim::Difficulty::Medium,
            3 => crate::sim::Difficulty::Hard,
            _ => return Err(ConfigError::InvalidValue("game.difficulty".to_string()).into()),
        };

        let view_distance = value.view_distance;
        let max_players = value.max_players;
        let reduce_debug_info = value.reduce_debug_info;
        let enable_respawn_screen = value.enable_respawn_screen;

        Ok(Self {
            seed,
            game_mode,
            difficulty,
            view_distance,
            max_players,
            reduce_debug_info,
            enable_respawn_screen
        })
    }
}
