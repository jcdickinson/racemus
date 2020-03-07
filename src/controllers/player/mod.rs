use async_std::sync::Receiver;

pub enum Message {
    ConnectionOpened {
        player_uuid: String,
        player_name: String,
    }
}

pub struct Controller {
    recv: Receiver<Message>
}
