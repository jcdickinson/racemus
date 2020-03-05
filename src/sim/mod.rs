mod types;
pub use types::*;

use crate::connection::messages::*;
use async_std::sync::{channel, Receiver, Sender};

pub fn create_client(backlog: usize) -> (ServerMessages, Receiver<ClientMessages>) {
    let (tx, rx) = channel::<ClientMessages>(backlog);
    (ServerMessages::Accept(tx), rx)
}

pub struct Simulation {
    sender: Sender<ServerMessages>,
    receiver: Receiver<ServerMessages>,
    cli: Vec<Sender<ClientMessages>>,
}

impl Simulation {
    pub fn new(backlog: usize) -> Self {
        let (sender, receiver) = channel::<ServerMessages>(backlog);
        Self {
            sender,
            receiver,
            cli: Vec::new(),
        }
    }

    pub fn execute(mut self) -> Sender<ServerMessages> {
        let sender = self.sender.clone();
        async_std::task::spawn(async move {
            loop {
                if let Some(message) = self.receiver.recv().await {
                    match message {
                        ServerMessages::Accept(cli) => {
                            let _ = cli.send(ClientMessages::JoinGame {
                                entity_id: 5,
                                game_mode: GameMode::Softcore(GameModeKind::Survival),
                                dimension: 0,
                                hashed_seed: 1,
                                level_type: "default".to_string(),
                                view_distance: 5,
                                reduce_debug: false,
                                enable_respawn_screen: true,
                            });
                            self.cli.push(cli);
                        }
                    }
                }
            }
        });
        sender
    }
}
