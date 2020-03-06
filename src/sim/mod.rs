mod client;
pub mod types;

pub use types::*;

use crate::connection::messages::*;
use acteur::{Actor, Assistant, Handle};
use async_std::sync::Sender;
use async_trait::async_trait;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Simulation {
    next_perm_id: u32,
    next_entity_id: u32,
    players: HashMap<String, u32>,
    config: Option<Configuration>,
}

impl Simulation {
    fn get_player_perm_id(&mut self, player_name: &str) -> u32 {
        match self.players.get(player_name) {
            Some(r) => *r,
            None => {
                let id = self.next_perm_id;
                self.next_perm_id += 1;
                self.players.insert(player_name.to_string(), id);
                id
            }
        }
    }
}

#[async_trait]
impl Actor for Simulation {
    type Id = u32;

    async fn activate(_: Self::Id) -> Self {
        Self {
            next_perm_id: 1,
            next_entity_id: 0,
            players: HashMap::new(),
            config: None,
        }
    }
}

#[derive(Debug)]
pub struct Configuration {
    game_mode: GameMode,
    seed: u64,
    render_distance: u8,
    reduce_debug: bool,
    enable_respawn_screen: bool,
}

impl Configuration {
    pub fn new(
        game_mode: GameMode,
        seed: u64,
        render_distance: u8,
        reduce_debug: bool,
        enable_respawn_screen: bool,
    ) -> Self {
        Self {
            game_mode,
            seed,
            render_distance,
            reduce_debug,
            enable_respawn_screen,
        }
    }
}

impl From<&crate::config::Config> for Configuration {
    fn from(config: &crate::config::Config) -> Self {
        Self {
            game_mode: config.game().game_mode(),
            seed: config.game().seed(),
            render_distance: config.game().view_distance(),
            reduce_debug: config.game().reduce_debug_info(),
            enable_respawn_screen: config.game().enable_respawn_screen(),
        }
    }
}

#[async_trait]
impl Handle<Configuration> for Simulation {
    async fn handle(&mut self, message: Configuration, _: Assistant) {
        self.config = Some(message);
    }
}

#[derive(Debug)]
pub struct PlayerConnected {
    name: String,
    sender: Sender<ClientMessages>,
}

impl PlayerConnected {
    pub fn new(name: String, sender: Sender<ClientMessages>) -> Self {
        Self {
            name,
            sender,
        }
    }
}

#[async_trait]
impl Handle<PlayerConnected> for Simulation {
    async fn handle(&mut self, message: PlayerConnected, assistant: Assistant) {
        let entity_id = self.next_entity_id;
        self.next_entity_id += 1;
        let perm_id = self.get_player_perm_id(&message.name);
        let config = self.config.as_ref().unwrap();

        assistant
            .send::<client::Player, _>(
                perm_id,
                client::Connected::new(
                    entity_id,
                    message.sender,
                    WorldInfo::new(
                        config.game_mode,
                        config.seed,
                        "racemus".to_string(),
                        config.render_distance,
                        config.reduce_debug,
                        config.enable_respawn_screen,
                    ),
                ),
            )
            .await;
    }
}

#[derive(Debug)]
pub struct PlayerDisconnected {
    name: String,
}

impl PlayerDisconnected {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

#[async_trait]
impl Handle<PlayerDisconnected> for Simulation {
    async fn handle(&mut self, message: PlayerDisconnected, assistant: Assistant) {
        let perm_id = self.get_player_perm_id(&message.name);
        assistant
            .send::<client::Player, _>(perm_id, client::Disconnected)
            .await;
    }
}
