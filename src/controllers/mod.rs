pub mod player;
pub mod server;

use async_std::{
    sync::{channel, Sender}
};

#[derive(Debug, Clone)]
pub struct Controllers {
    config: crate::config::Config,
    server: Sender<server::Message>,
    player: Sender<player::Message>,
}

impl Controllers {
    pub fn new(config: &crate::config::Config, cap: usize) -> Controllers {
        let (server_tx, server_rx) = channel(cap);
        let (player_tx, player_rx) = channel(cap);
        let controllers = Controllers {
            config: config.clone(),
            server: server_tx,
            player: player_tx
        };
        player::Controller::start(controllers.clone(), player_rx);
        server::Controller::start(controllers.clone(), server_rx);
        controllers
    }

    pub fn config(&self) -> &crate::config::Config {
        &self.config
    }

    pub async fn send_server(&self, message: server::Message) {
        self.server.send(message).await
    }
    
    pub async fn send_player(&self, message: player::Message) {
        self.player.send(message).await
    }
}