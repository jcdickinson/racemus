use log::trace;
use std::{
    error::Error,
    sync::Arc
};

const HAS_JOINED: &str = "https://sessionserver.mojang.com/session/minecraft/hasJoined";

#[derive(Debug)]
pub enum ApiError {
    LoginFail,
    InvalidData,
    MissingProperty(&'static str),
    InvalidProperty(&'static str),
}

impl Error for ApiError {}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Self::LoginFail => write!(f, "LoginFail"),
            Self::InvalidData => write!(f, "InvalidData"),
            Self::MissingProperty(p) => write!(f, "MissingProperty({})", p),
            Self::InvalidProperty(p) => write!(f, "InvalidProperty({})", p),
        }
    }
}

pub struct PlayerInfo {
    player_name: Arc<Box<str>>,
    uuid: Arc<Box<str>>,
}

impl PlayerInfo {
    pub fn player_name(&self) -> &Arc<Box<str>> {
        &self.player_name
    }
    pub fn uuid(&self) -> &Arc<Box<str>> {
        &self.uuid
    }
}

pub async fn player_join_session(
    player_name: &str,
    server_hash: &str,
) -> Result<PlayerInfo, Box<dyn Error + Send + Sync + 'static>> {
    let mut url = url::Url::parse(HAS_JOINED).unwrap();
    url.query_pairs_mut()
        .append_pair("username", &player_name)
        .append_pair("serverId", &server_hash);
    let url = url.to_string();

    trace!("sending login request: {}", url);
    let mut resp = surf::get(url).await?;

    if resp.status().as_u16() != 200 {
        return Err(Box::new(ApiError::LoginFail));
    }

    let resp = resp.body_bytes().await?;
    let resp = serde_json::from_slice(&resp)?;

    match resp {
        serde_json::Value::Object(o) => {
            let id = match o.get("id") {
                Some(r) => r,
                None => return Err(Box::new(ApiError::MissingProperty("id"))),
            };

            let mut uuid = match id {
                serde_json::Value::String(r) => r.clone(),
                _ => return Err(Box::new(ApiError::InvalidProperty("id"))),
            };

            let player_name = match o.get("name") {
                Some(r) => r,
                None => return Err(Box::new(ApiError::MissingProperty("name"))),
            };

            let player_name = match player_name {
                serde_json::Value::String(r) => r.clone(),
                _ => return Err(Box::new(ApiError::InvalidProperty("name"))),
            };

            uuid.insert(20, '-');
            uuid.insert(16, '-');
            uuid.insert(12, '-');
            uuid.insert(8, '-');

            let player_name = Arc::new(player_name.into());
            let uuid = Arc::new(uuid.into());

            Ok(PlayerInfo { player_name, uuid })
        }
        _ => Err(Box::new(ApiError::InvalidData)),
    }
}
