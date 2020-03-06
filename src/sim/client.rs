use crate::connection::messages::*;
use acteur::{Actor, Assistant, Handle};
use async_std::sync::Sender;
use async_trait::async_trait;

#[derive(Debug)]
pub struct Player {
    perm_id: u32,
    dimension: i32,
    held_item_slot: u8,
    connection: Option<PlayerConnection>,
    x: f64,
    y: f64,
    z: f64,
    yaw: f32,
    pitch: f32
}

#[derive(Debug)]
struct PlayerConnection {
    entity_id: u32,
    sender: Sender<ClientMessages>,
}

#[async_trait]
impl Actor for Player {
    type Id = u32;

    async fn activate(perm_id: Self::Id) -> Self {
        Self {
            perm_id,
            dimension: 0,
            held_item_slot: 0,
            connection: None,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            yaw: 0.0,
            pitch: 0.0
        }
    }
}

#[derive(Debug)]
pub struct Connected {
    entity_id: u32,
    sender: Sender<ClientMessages>,
    world: super::WorldInfo,
}

impl Connected {
    pub fn new(entity_id: u32, sender: Sender<ClientMessages>, world: super::WorldInfo) -> Self {
        Self {
            entity_id,
            sender,
            world,
        }
    }
}

#[async_trait]
impl Handle<Connected> for Player {
    async fn handle(&mut self, message: Connected, assistant: Assistant) {
        self.connection = Some(PlayerConnection {
            entity_id: message.entity_id,
            sender: message.sender,
        });

        assistant
            .send::<Player, _>(
                message.entity_id,
                ClientMessages::JoinGame {
                    entity_id: message.entity_id as i32,
                    game_mode: message.world.game_mode(),
                    dimension: self.dimension,
                    hashed_seed: message.world.hashed_seed(),
                    level_type: message.world.level_type().to_string(),
                    view_distance: message.world.view_distance() as i32,
                    reduce_debug: message.world.reduce_debug(),
                    enable_respawn_screen: message.world.enable_respawn_screen(),
                },
            )
            .await;
        assistant
            .send::<Player, _>(
                message.entity_id,
                ClientMessages::HeldItemChange {
                    slot: self.held_item_slot
                }
            )
            .await;
        assistant
            .send::<Player, _>(
                message.entity_id,
                ClientMessages::PlayerPositionAndLook {
                    x: self.x,
                    y: self.y,
                    z: self.z,
                    yaw: self.yaw,
                    pitch: self.pitch,
                    flags: 0,
                    teleport_id: 0
                }
            )
            .await;
    }
}

#[async_trait]
impl Handle<ClientMessages> for Player {
    async fn handle(&mut self, message: ClientMessages, _: Assistant) {
        if let Some(connection) = self.connection.as_mut() {
            connection.sender.send(message).await;
        }
    }
}

#[derive(Debug)]
pub struct Disconnected;

#[async_trait]
impl Handle<Disconnected> for Player {
    async fn handle(&mut self, _: Disconnected, _: Assistant) {
        self.connection = None;
    }
}
