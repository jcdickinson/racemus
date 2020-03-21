use crate::models::*;
use async_std::{
    sync::{Receiver, Sender},
    task,
};

pub enum Message {
    AllocateEntity(Sender<EntityId>),
}

pub struct Controller {
    //controllers: super::Controllers,
    receiver: Receiver<Message>,
    entity_id: u32,
}

impl Controller {
    pub fn start(_controllers: super::Controllers, receiver: Receiver<Message>) {
        let mut controller = Controller {
            receiver,
            entity_id: 0,
        };
        task::spawn(async move {
            controller.execute().await;
        });
    }

    async fn execute(&mut self) {
        loop {
            match self.receiver.recv().await {
                None => {
                    self.close().await;
                    return;
                }
                Some(Message::AllocateEntity(sender)) => {
                    let eid = self.entity_id;
                    self.entity_id += 1;
                    sender.send(eid.into()).await;
                }
            }
        }
    }

    async fn close(&self) {}
}
