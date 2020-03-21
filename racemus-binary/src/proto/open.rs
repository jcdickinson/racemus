use crate::{writer::StructuredWriter, BinaryReader, BinaryWriter, Error, ErrorKind};
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
    Unknown {
        packet_id: i32,
    },
}

impl<R: Read + Unpin> BinaryReader<R> {
    pub async fn read_open(&mut self) -> Result<OpenRequest, Error> {
        let packet_id = self.packet_header().await?;
        match packet_id {
            0x00 => {
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
                    next_state
                })
            }
            _ => Ok(OpenRequest::Unknown { packet_id }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenResponse {}

impl<W: Write + Unpin> StructuredWriter<W, OpenResponse> for BinaryWriter<W> {
    fn structure(&mut self, _: &OpenResponse) -> Result<&mut Self, Error> {
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{OpenRequest::*, *};
    use crate::tests::*;

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
    );
}
