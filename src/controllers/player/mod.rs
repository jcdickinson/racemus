use crate::{
    controllers::server,
    models::*,
    sync::wait,
    connection::ClientMessage
};
use async_std::{
    sync::{Receiver, Sender},
    task,
};
use std::{
    sync::Arc,
    collections::HashMap
};

pub enum Message {
    ConnectionOpened {
        player_uuid: Arc<Box<str>>,
        player_name: Arc<Box<str>>,
        sender: Sender<crate::connection::ClientMessage>,
    },
    ConnectionClosed {
        player_uuid: Arc<Box<str>>
    }
}

pub struct Controller {
    controllers: super::Controllers,
    receiver: Receiver<Message>,
    players: HashMap<Arc<Box<str>>, Player>
}

impl Controller {
    pub fn start(controllers: super::Controllers, receiver: Receiver<Message>) {
        let mut controller = Controller {
            controllers,
            receiver,
            players: HashMap::new()
        };
        task::spawn(async move {
            controller.execute().await;
        });
    }

    async fn execute(&mut self) {
        loop {
            match self.receiver.recv().await {
                None => {
                    self.disconnect().await;
                    return;
                }
                Some(Message::ConnectionOpened {
                    player_uuid,
                    player_name,
                    sender,
                }) => {
                    let player = Player::new(player_uuid, player_name, sender, self.controllers.config());
                    self.load_player(player).await;
                },
                Some(Message::ConnectionClosed {
                    player_uuid: _
                }) => {
                    
                }
            }
        }
    }

    async fn disconnect(&self) {}

    async fn load_player(&mut self, player: Player) {
        let eid = wait(|complete| {
            self.controllers
                .send_server(server::Message::AllocateEntity(complete))
        })
        .await;
        let mut player = player;

        if let Some(eid) = eid {
            player.entity_id = eid
        }

        player.sender.send(ClientMessage::JoinGame {
            entity_id: player.entity_id,
            game_mode: player.game_mode,
            dimension: player.dimension,
            hashed_seed: self.controllers.config().game().seed(),
            level_type: Arc::new("default".into()),
            view_distance: self.controllers.config().game().view_distance(),
            reduce_debug: self.controllers.config().game().reduce_debug_info(),
            enable_respawn_screen: self.controllers.config().game().enable_respawn_screen(),
        }).await;

        player.sender.send(ClientMessage::PluginBrand{
            brand: "racemus"
        }).await;

        player.sender.send(ClientMessage::ServerDifficulty{
            difficulty: self.controllers.config().game().difficulty(),
            difficulty_locked: true
        }).await;

        player.sender.send(ClientMessage::PlayerPositionAndLook {
            position: player.position,
            look: player.look,
            flags: 0,
            teleport_id: 0
        }).await;

        player.sender.send(ClientMessage::ChunkData {
            position: vek::Vec2::zero()
        }).await;
        
        self.players.insert(player.uuid.clone(), player);
    }
}

struct Player {
    uuid: Arc<Box<str>>,
    //name: Arc<Box<str>>,
    sender: Sender<crate::connection::ClientMessage>,
    entity_id: EntityId,

    game_mode: GameMode,
    dimension: i32,

    position: vek::Vec3<f64>,
    look: vek::Vec2<f32>
}

impl Player {
    pub fn new(
        uuid: Arc<Box<str>>,
        _name: Arc<Box<str>>,
        sender: Sender<crate::connection::ClientMessage>,
        config: &crate::config::Config,
    ) -> Self {
        Self {
            uuid,
            //name,
            sender,
            entity_id: EntityId::default(),
            game_mode: config.game().game_mode(),
            dimension: 0,
            position: vek::Vec3::new(0.0, 255.0, 0.0),
            look: vek::Vec2::zero()
        }
    }
}
