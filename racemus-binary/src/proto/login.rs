use crate::{writer::StructuredWriter, BinaryReader, BinaryWriter, Error};
use async_std::io::{Read, Write};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginRequest {
    Start {
        player_name: Arc<str>,
    },
    EncryptionResponse {
        encrypted_shared_secret: Arc<[u8]>,
        encrypted_verifier: Arc<[u8]>,
    },
    Unknown {
        packet_id: i32,
    },
}

impl<R: Read + Unpin> BinaryReader<R> {
    pub async fn read_login(&mut self) -> Result<LoginRequest, Error> {
        let packet_id = self.packet_header().await?;
        match packet_id {
            0x00 => {
                let player_name = self.arr_char(Some(16)).await?;
                Ok(LoginRequest::Start { player_name })
            }
            0x01 => {
                let encrypted_shared_secret = self.arr_u8(Some(128)).await?;
                let encrypted_verifier = self.arr_u8(Some(128)).await?;
                Ok(LoginRequest::EncryptionResponse {
                    encrypted_shared_secret,
                    encrypted_verifier,
                })
            }
            _ => Ok(LoginRequest::Unknown { packet_id }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginResponse<'a> {
    EncryptionRequest {
        public_key: &'a [u8],
        verify_token: &'a [u8],
    },
    Success {
        player_uuid: &'a str,
        player_name: &'a str,
    },
    Disconnect {
        reason: &'a str,
    },
}

impl<'a, W: Write + Unpin> StructuredWriter<W, LoginResponse<'a>> for BinaryWriter<W> {
    fn structure(&mut self, val: &LoginResponse<'a>) -> Result<&mut Self, Error> {
        let packet = self.start_packet();
        match val {
            LoginResponse::EncryptionRequest {
                public_key,
                verify_token,
            } => self
                .var_i32(0x01)?
                .var_i32(0)? // Server ID (obsolete)
                .arr_u8(public_key)?
                .arr_u8(verify_token)?,
            LoginResponse::Success {
                player_uuid,
                player_name,
            } => self
                .var_i32(0x02)?
                .arr_char(player_uuid)?
                .arr_char(player_name)?,
            LoginResponse::Disconnect { reason } => self.var_i32(0x00)?.arr_char(reason)?,
        }
        .complete_packet(packet)
    }
}

#[cfg(test)]
mod tests {
    use super::{LoginRequest::*, LoginResponse::*, *};
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
        binary_writer_login_encryption_request, "test-data/login-encryption-request-1.in", w => w.structure(&EncryptionRequest{
            public_key: b"1234",
            verify_token: b"5678",
        })?;
        binary_writer_login_success, "test-data/login-success-1.in", w => w.structure(&Success{
            player_uuid: "1234",
            player_name: "5678"
        })?;
        binary_writer_login_disconnect, "test-data/login-disconnect-1.in", w => w.structure(&Disconnect{
            reason: "bad player"
        })?;
    );

    macro_rules! raw_read_tests {
        ($($name:ident, $input:expr, $expected:expr;)*) => {
            $(
                #[test]
                pub fn $name() -> Result<(), Error> {
                    let mut reader = make_reader(include_bytes!($input) as &[u8]);
                    assert_eq!(block_on(reader.read_login())?, $expected);
                    Ok(())
                }
            )*
        }
    }

    raw_read_tests!(
        binary_reader_login_start, "test-data/login-start-1.in", Start {
            player_name: "test".into()
        };
        binary_reader_login_encryption_response, "test-data/login-encryption-response-1.in", EncryptionResponse {
            encrypted_shared_secret: (b"1234" as &[u8]).into(),
            encrypted_verifier: (b"56789" as &[u8]).into()
        };
    );
}
