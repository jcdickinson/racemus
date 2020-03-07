#![warn(rust_2018_idioms)]

pub mod config;
pub mod connection;
pub mod mojang;
pub mod models;

use async_std::net::TcpListener;
use async_std::prelude::*;
use connection::Connection;
use log::{error, info, warn};

const ENV_LOG: &str = "RACEMUS_LOG";

#[async_std::main]
async fn main() {
    pretty_env_logger::init_custom_env(ENV_LOG);
    let config_data = match config::Config::read("server.toml").await {
        Ok(r) => r,
        Err(e) => {
            error!("invalid configuration file: {}", e);
            return;
        }
    };

    let addr = config_data.network().addr();

    let keys = match read_keys(
        config_data.security().private_key(),
        config_data.security().public_key(),
    )
    .await
    {
        Ok(r) => r,
        Err(_) => return,
    };

    let listener = match TcpListener::bind(&addr).await {
        Ok(listener) => {
            info!("listening on: {}", addr);
            listener
        }
        Err(error) => {
            error!("failed to start listener: {}", error);
            return;
        }
    };

    loop {
        match listener.accept().await {
            Ok((socket, cli)) => {
                info!("({}) client connected", cli);
                if let Err(error) = socket.set_nodelay(true) {
                    warn!("({}) failed to set no_delay: {}", cli, error);
                }

                let connection = Connection::new(
                    socket.clone(),
                    socket,
                    cli,
                    keys.clone(),
                    config_data.network().motd(),
                );
                connection.execute();
            }
            Err(error) => {
                error!("failed to accept client: {}", error);
                return;
            }
        };
    }
}

async fn read_keys(
    private_key_path: &str,
    public_key_path: &str,
) -> Result<connection::InsecurePrivateKey, ()> {
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
    let key = match connection::InsecurePrivateKey::from_der(&private_key, &public_key) {
        Ok(r) => r,
        Err(_) => {
            error!("failed to extract private key from: {}", private_key_path);
            return Err(());
        }
    };
    Ok(key)
}

async fn read_file(file_name: &str) -> Result<Vec<u8>, ()> {
    let mut file = match async_std::fs::File::open(file_name).await {
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
