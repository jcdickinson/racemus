use crate::{
    proto::packet_ids::open as packet_ids, writer::StructuredWriter, BinaryReader, BinaryWriter,
    Error, ErrorKind,
};
use async_std::io::{Read, Write};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestedState {
    Status,
    Login,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenRequest {
    Handshake {
        version: i32,
        address: Arc<str>,
        port: u16,
        next_state: RequestedState,
    },
    HttpGet {

    },
    Unknown {
        packet_id: i32,
    },
}

impl<R: Read + Unpin> BinaryReader<R> {
    pub async fn read_open(&mut self) -> Result<OpenRequest, Error> {
        // Effectively tests for the first bytes.
        if self.remaining().is_none() &&
            // HTTP Request
            self.data(1).await? == b"G" &&
            self.data(4).await? == b"GET " {
            
            return Ok(OpenRequest::HttpGet{})
        }

        let packet_id = self.packet_header().await?;
        match packet_id {
            packet_ids::HANDSHAKE => {
                let version = self.var_i32().await?;
                let address = self.arr_char(Some(255)).await?;
                let port = self.fix_u16().await?;
                let next_state = self.var_i32().await?;
                let next_state = match next_state {
                    0x01 => RequestedState::Status,
                    0x02 => RequestedState::Login,
                    _ => return Err(ErrorKind::InvalidState(next_state).into()),
                };
                Ok(OpenRequest::Handshake {
                    version,
                    address,
                    port,
                    next_state,
                })
            },
            _ => Ok(OpenRequest::Unknown { packet_id }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenResponse {
    HttpOK {}
}

impl<W: Write + Unpin> StructuredWriter<W, OpenResponse> for BinaryWriter<W> {
    fn structure(&mut self, val: &OpenResponse) -> Result<&mut Self, Error> {
        match val {
            OpenResponse:: HttpOK {} => self.raw_buffer(b"HTTP/1.1 200 OK\r\nConnection: close\r\n")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{OpenRequest::*, OpenResponse::*, *};
    use crate::tests::*;
    
    macro_rules! raw_write_tests {
        ($($name:ident, $expected:expr, $writer:ident => $expr:expr;)*) => {
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
        binary_writer_open_http_ok, "test-data/open-http-ok-1.in", w => w.structure(&HttpOK{})?;
    );

    macro_rules! raw_read_tests {
        ($($name:ident, $input:expr, $expected:expr;)*) => {
            $(
                #[test]
                pub fn $name() -> Result<(), Error> {
                    let mut reader = make_reader(include_bytes!($input) as &[u8]);
                    assert_eq!(block_on(reader.read_open())?, $expected);
                    Ok(())
                }
            )*
        }
    }

    raw_read_tests!(
        binary_reader_open_handshake, "test-data/open-handshake-1.in", Handshake {
            version: 21,
            address: "localhost".into(),
            port: 25565,
            next_state: RequestedState::Status,
        };
        binary_reader_open_http_get, "test-data/open-http-get-1.in", HttpGet {};
    );
}
