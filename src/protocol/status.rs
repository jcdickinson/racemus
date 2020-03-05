#![allow(clippy::too_many_arguments)]

use super::{PacketReader, PacketWriter};
use std::io::{Error, ErrorKind};
use std::marker::Unpin;
use async_std::io::{Read, Write};
use serde_json::json;

#[derive(Debug, PartialEq, Eq)]
pub enum Packet {
    Request,
    Ping(Ping),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Ping {
    timestamp: u64
}

impl Ping {
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }
}

pub async fn read_packet<R: Read + Unpin>(
    reader: &mut PacketReader<R>,
) -> Result<Packet, Error> {
    match reader.packet_header().await? {
        0x00 => {
            Ok(Packet::Request)
        }
        0x01 => {
            let timestamp = reader.fix_u64().await?;
            Ok(Packet::Ping(Ping {
                timestamp
            }))
        }
        _ => Err(ErrorKind::InvalidData.into()),
    }
}

pub async fn write_response<W: Write + Unpin>(
    writer: &mut PacketWriter<W>
) -> Result<(), Error> {
    let response = json!({
        "version": {
            "name": crate::protocol::SERVER_VERSION,
            "protocol": crate::protocol::SERVER_VERSION_NUMBER
        },
        "players": {
            "max": 100,
            "online": 0
        }
    });
    let s = serde_json::to_string(&response).unwrap();
    writer.packet_id(0x00)
        .arr_char(&s)
        .flush().await
}

pub async fn write_pong<W: Write + Unpin>(
    writer: &mut PacketWriter<W>,
    timestamp: u64
) -> Result<(), Error> {
    writer.packet_id(0x01)
        .fix_u64(timestamp)
        .flush().await
}
