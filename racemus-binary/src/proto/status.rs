use crate::{writer::StructuredWriter, BinaryReader, BinaryWriter, Error};
use async_std::io::{Read, Write};
use serde_json::json;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusRequest {
    InfoRequest,
    Ping { timestamp: u64 },
    Unknown { packet_id: i32 },
}

impl<R: Read + Unpin> BinaryReader<R> {
    pub async fn read_status(&mut self) -> Result<StatusRequest, Error> {
        let packet_id = self.packet_header().await?;
        match packet_id {
            0x00 => Ok(StatusRequest::InfoRequest),
            0x01 => {
                let timestamp = self.fix_u64().await?;
                Ok(StatusRequest::Ping { timestamp })
            }
            _ => Ok(StatusRequest::Unknown { packet_id }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusResponse<'a> {
    InfoResponse {
        max_players: u16,
        current_players: u16,
        description: &'a str,
    },
    Pong {
        timestamp: u64,
    },
}

impl<'a, W: Write + Unpin> StructuredWriter<W, StatusResponse<'a>> for BinaryWriter<W> {
    fn structure(&mut self, val: &StatusResponse) -> Result<&mut Self, Error> {
        let insertion = self.create_insertion();
        match val {
            StatusResponse::Pong { timestamp } => self.var_i32(0x01)?.fix_u64(*timestamp)?,
            StatusResponse::InfoResponse {
                max_players,
                current_players,
                description,
            } => {
                let response = json!({
                    "version": {
                        "name": crate::SERVER_VERSION,
                        "protocol": crate::SERVER_VERSION_NUMBER
                    },
                    "players": {
                        "max": max_players,
                        "online": current_players
                    },
                    "description": {
                        "text": description
                    }
                });
                let response = serde_json::to_string(&response).unwrap();
                self.var_i32(0x01)?.arr_char(&response)?
            }
        }
        .insert_len_var_i32(insertion)
    }
}

#[cfg(test)]
mod tests {
    use super::{StatusRequest::*, StatusResponse::*, *};
    use crate::tests::*;

    macro_rules! raw_write_tests {
        ($($name:ident, $expected:expr, $writer:ident => $expr:expr),*) => {
            $(
                #[test]
                fn $name() -> Result<(), Error> {
                    let mut $writer = make_writer();
                    $expr;
                    let buf = make_buffer($writer);
                    assert_eq!(buf, include_bytes!($expected) as &[u8]);
                    Ok(())
                }
            )*
        }
    }

    raw_write_tests!(
        binary_writer_status_pong, "test-data/status-pong-1.in", w => w.structure(&Pong{
            timestamp: 0x1526_3749_5015_2637
        })?,
        binary_writer_status_info_response, "test-data/status-info-response-1.in", w => w.structure(&InfoResponse {
            max_players: 50,
            current_players: 21,
            description: "Welcome!"
        })?
    );

    macro_rules! raw_read_tests {
        ($($name:ident, $input:expr, $expected:expr;)*) => {
            $(
                #[test]
                pub fn $name() -> Result<(), Error> {
                    let mut reader = make_reader(include_bytes!($input) as &[u8]);
                    assert_eq!(block_on(reader.read_status())?, $expected);
                    Ok(())
                }
            )*
        }
    }

    raw_read_tests!(
        binary_reader_status_info_request, "test-data/status-info-request-1.in", InfoRequest;
        binary_reader_status_ping, "test-data/status-ping-1.in", Ping {
            timestamp: 0x1526_3749_5015_2637
        };
    );
}
