mod messages;
pub use messages::*;

use tokio::sync::mpsc::{ channel, Receiver, Sender };

pub fn create_client(backlog: usize) -> (SimMessages, Receiver<ClientMessages>) {
    let (tx, rx) = channel::<ClientMessages>(backlog);
    (SimMessages::Accept(tx), rx)
}

pub struct Simulation {
    sender: Sender<SimMessages>,
    receiver: Receiver<SimMessages>,
    cli: Vec<Sender<ClientMessages>>
}

impl Simulation {
    pub fn new(backlog: usize) -> Self {
        let (sender, receiver) = channel::<SimMessages>(backlog);
        Self {
            sender,
            receiver,
            cli: Vec::new()
        }
    }

    pub fn execute(mut self) -> Sender<SimMessages> {
        let sender = self.sender.clone();
        tokio::spawn(async move {
            loop {
                if let Some(message) = self.receiver.recv().await {
                    match message {
                        SimMessages::Accept(mut cli) => {
                            let _ = cli.send(ClientMessages::JoinGame(JoinGame {
                                entity_id: 5,
                                game_mode: GameMode::Softcore(GameModeKind::Survival),
                                dimension: 0,
                                hashed_seed: 1,
                                level_type: "default".to_string(),
                                view_distance: 5,
                                reduce_debug: false,
                                enable_respawn_screen: true
                            }));
                            self.cli.push(cli);
                        }
                    }
                }
            };
        });
        sender
    }
}

