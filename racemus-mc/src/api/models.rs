#[derive(Debug)]
pub struct PlayerInfo {
    name: String,
    uuid: String,
}

impl PlayerInfo {
    pub fn new(name: String, uuid: String) -> Self {
        Self { name, uuid }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn uuid(&self) -> &str {
        &self.uuid
    }
}
