#![warn(rust_2018_idioms)]

pub mod chat;
pub mod connection;
pub mod crypto;
pub mod mojang;
pub mod protocol;
pub mod sim;

use connection::Connection;
use log::{error, info, warn};
use std::env;
use tokio::net::TcpListener;
use tokio::prelude::*;

const ENV_LOG: &str = "RACEMUS_LOG";
const ENV_ENDPOINT: &str = "RACEMUS_ENDPOINT";
const ENV_PRIVATE: &str = "RACEMUS_PRIVATE_KEY";
const ENV_PUBLIC: &str = "RACEMUS_PUBLIC_KEY";
const DEFAULT_LISTEN: &str = "0.0.0.0:25565";
const DEFAULT_PRIVATE: &str = "server_rsa";
const DEFAULT_PUBLIC: &str = "server_rsa.pub";

#[tokio::main]
async fn main() {
    pretty_env_logger::init_custom_env(ENV_LOG);
    let addr = env::var(ENV_ENDPOINT).unwrap_or_else(|_| DEFAULT_LISTEN.to_string());

    let keys = match read_keys().await {
        Ok(r) => r,
        Err(_) => return,
    };

    let mut listener = match TcpListener::bind(&addr).await {
        Ok(listener) => {
            info!("listening on: {}", addr);
            listener
        }
        Err(error) => {
            error!("failed to start listener: {}", error);
            return;
        }
    };
    let simulation = sim::Simulation::new(10);
    let simulation = simulation.execute();

    loop {
        match listener.accept().await {
            Ok((socket, cli)) => {
                info!("({}) client connected", cli);
                if let Err(error) = socket.set_nodelay(true) {
                    warn!("({}) failed to set no_delay: {}", cli, error);
                }

                let (read, write) = tokio::io::split(socket);
                let send = simulation.clone();
                let connection = Connection::new(read, write, send, cli, keys.clone());
                connection.execute();
            }
            Err(error) => {
                error!("failed to accept client: {}", error);
                return;
            }
        };
    }
}

async fn read_keys() -> Result<crypto::insecure::InsecurePrivateKey, ()> {
    let private_key_path = env::var(ENV_PRIVATE).unwrap_or_else(|_| DEFAULT_PRIVATE.to_string());
    let public_key_path = env::var(ENV_PUBLIC).unwrap_or_else(|_| DEFAULT_PUBLIC.to_string());

    let private_key = match read_file(&private_key_path).await {
        Ok(r) => r,
        Err(_) => {
            error!("failed to read public key from: {}", private_key_path);
            return Err(());
        }
    };
    let public_key = match read_file(&public_key_path).await {
        Ok(r) => r,
        Err(_) => {
            error!("failed to read public key from: {}", public_key_path);
            return Err(());
        }
    };
    let key = match crypto::insecure::InsecurePrivateKey::from_der(&private_key, &public_key) {
        Ok(r) => r,
        Err(_) => {
            error!("failed to extract private key from: {}", private_key_path);
            return Err(());
        }
    };
    Ok(key)
}

async fn read_file(file_name: &str) -> Result<Vec<u8>, ()> {
    let mut file = match tokio::fs::File::open(file_name).await {
        Ok(file) => file,
        Err(error) => match error.kind() {
            std::io::ErrorKind::NotFound => {
                error!("could not find {} file.", file_name);
                return Err(());
            }
            e => {
                error!("could not open {} file: {:?}", file_name, e);
                return Err(());
            }
        },
    };

    let mut contents = vec![];
    if let Err(e) = file.read_to_end(&mut contents).await {
        error!("could not read {} file: {:?}", file_name, e);
        return Err(());
    }

    Ok(contents)
}
