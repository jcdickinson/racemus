use super::{PacketReader};
use std::io::{Error, ErrorKind};
use std::marker::Unpin;
use tokio::io::{AsyncRead};

#[derive(Debug, PartialEq, Eq)]
pub enum Packet {
    Handshake(Handshake),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Handshake {
    version: i32,
    addr: String,
    port: u16,
    next_state: RequestedState,
}

impl Handshake {
    pub fn version(&self) -> i32 {
        self.version
    }
    pub fn addr(&self) -> &str {
        &self.addr
    }
    pub fn port(&self) -> u16 {
        self.port
    }
    pub fn next_state(&self) -> RequestedState {
        self.next_state
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestedState {
    Status,
    Login,
}

pub async fn read_packet<R: AsyncRead + Unpin>(
    reader: &mut PacketReader<R>,
) -> Result<Packet, Error> {
    match reader.packet_header().await? {
        0x00 => {
            let version = reader.var_i32().await?;
            let addr = reader.arr_char(Some(255)).await?;
            let port = reader.fix_u16().await?;
            let next_state = reader.var_i32().await?;
            let next_state = match next_state {
                0x01 => RequestedState::Status,
                0x02 => RequestedState::Login,
                _ => return Err(ErrorKind::InvalidData.into())
            };
            Ok(Packet::Handshake(Handshake {
                version,
                addr,
                port,
                next_state
            }))
        }
        _ => Err(ErrorKind::InvalidData.into()),
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;
    use std::io::Cursor;

    macro_rules! sync {
        ($e:expr) => {
            block_on($e).unwrap()
        };
    }
    
    macro_rules! sync_err {
        ($e:expr) => {
            match block_on($e) {
                Ok(_) => None,
                Err(e) => Some(e.kind())
            }
        };
    }

    macro_rules! read_tests {
        ($($name:ident, $input:literal: $method:ident, $expected:expr),*) => {
            $(
                #[test]
                pub fn $name() {
                    let input = $input as &[u8];
                    let target = Cursor::new(input);
                    let mut reader = PacketReader::new(target);
                    assert_eq!(sync!($method(&mut reader)), $expected);
                }
            )*
        }
    }

    macro_rules! read_error_tests {
        ($($name:ident, $input:literal: $method:ident, $expected:expr),*) => {
            $(
                #[test]
                pub fn $name() {
                    let input = $input as &[u8];
                    let target = Cursor::new(input);
                    let mut reader = PacketReader::new(target);
                    assert_eq!(sync_err!($method(&mut reader)), $expected);
                }
            )*
        }
    }

    read_tests! {
        read_status_handshake, b"\x0b\x00\x02\x05abcde\xaa\xbb\x01": read_packet, Packet::Handshake(Handshake {
            version: 0x02,
            addr: "abcde".to_string(),
            port: 0xaabb,
            next_state: RequestedState::Status
        }),
        read_login_handshake, b"\x0b\x00\x02\x05abcde\xaa\xbb\x02": read_packet, Packet::Handshake(Handshake {
            version: 0x02,
            addr: "abcde".to_string(),
            port: 0xaabb,
            next_state: RequestedState::Login
        })
    }

    read_error_tests! {
        read_unknown_handshake, b"\x07\xff\xff\xff\xff\x07": read_packet, Some(ErrorKind::InvalidData),
        read_unknown_handshake_state, b"\x0b\x00\x02\x05abcde\xaa\xbb\x03": read_packet, Some(ErrorKind::InvalidData)
    }
}
