use hyper_tls::HttpsConnector;
use log::trace;
use std::error::Error;

const HAS_JOINED: &str = "https://sessionserver.mojang.com/session/minecraft/hasJoined";
const GET_USER: &str = "https://api.mojang.com/users/profiles/minecraft";

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
    player_name: String,
    uuid: String,
}

impl PlayerInfo {
    pub fn player_name(&self) -> &String {
        &self.player_name
    }
    pub fn uuid(&self) -> &String {
        &self.uuid
    }
}

async fn read_player_info(
    resp: http::response::Response<hyper::Body>,
) -> Result<PlayerInfo, Box<dyn Error>> {
    let resp = hyper::body::to_bytes(resp).await?;
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

            Ok(PlayerInfo { player_name, uuid })
        }
        _ => Err(Box::new(ApiError::InvalidData)),
    }
}

pub async fn get_player_info(player_name: &str) -> Result<PlayerInfo, Box<dyn Error>> {
    let mut url = url::Url::parse(GET_USER).unwrap();
    if let Ok(mut path) = url.path_segments_mut() {
        path.push(player_name);
    }

    let url = url.to_string().parse().unwrap();

    trace!("sending player request: {}", url);
    let connector = HttpsConnector::new();
    let client = hyper::Client::builder().build::<_, hyper::Body>(connector);
    let resp = client.get(url).await?;

    match resp.status().as_u16() {
        200 => read_player_info(resp).await,
        _ => Err(Box::new(ApiError::LoginFail)),
    }
}

pub async fn player_join_session(
    player_name: &str,
    server_hash: &str,
) -> Result<PlayerInfo, Box<dyn Error>> {
    let mut url = url::Url::parse(HAS_JOINED).unwrap();
    url.query_pairs_mut()
        .append_pair("username", &player_name)
        .append_pair("serverId", &server_hash);
    let url = url.to_string().parse().unwrap();

    trace!("sending login request: {}", url);
    let connector = HttpsConnector::new();
    let client = hyper::Client::builder().build::<_, hyper::Body>(connector);
    let resp = client.get(url).await?;

    match resp.status().as_u16() {
        200 => read_player_info(resp).await,
        _ => Err(Box::new(ApiError::LoginFail)),
    }
}
