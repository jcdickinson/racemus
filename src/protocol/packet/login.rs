use crate::protocol::extensions::{take_buffer, take_var_i32};
use crate::protocol::protocol_error::ProtocolErrorKind;
use crate::protocol::writers::{AesCfb8, PacketWriter};
use tokio::io::AsyncWrite;

#[derive(Debug, PartialEq, Eq)]
pub enum Packet<'a> {
    LoginStart(LoginStart<'a>),
    EncryptionResponse(EncryptionResponse<'a>),
}

#[derive(Debug, PartialEq, Eq)]
pub struct LoginStart<'a> {
    player_name: &'a str,
}

impl<'a> LoginStart<'a> {
    pub fn player_name(&'a self) -> &'a str {
        &self.player_name
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestedState {
    Status,
    Login,
}

#[derive(Debug, PartialEq, Eq)]
pub struct EncryptionResponse<'a> {
    encrypted_shared_secret: &'a [u8],
    encrypted_verifier: &'a [u8],
}

impl<'a> EncryptionResponse<'a> {
    pub fn encrypted_shared_secret(&'a self) -> &'a [u8] {
        &self.encrypted_shared_secret
    }
    pub fn encrypted_verifier(&'a self) -> &'a [u8] {
        &self.encrypted_verifier
    }
}

build_utf8!(take_player_name, 16);

build_packet_parser!(i:
    0x00 => {
        let (i, player_name) = take_player_name(i)?;
        Ok((
            i,
            Packet::LoginStart(LoginStart {
                player_name
            }),
        ))
    },
    0x01 => {
        let (i, encrypted_shared_secret) = take_buffer(i)?;
        let (i, encrypted_verifier) = take_buffer(i)?;
        Ok((
            i,
            Packet::EncryptionResponse(EncryptionResponse {
                encrypted_shared_secret,
                encrypted_verifier
            })
        ))
    }
);

pub struct EncryptionRequest<'a> {
    public_key: &'a [u8],
    verify_token: &'a [u8],
}

impl<'a> EncryptionRequest<'a> {
    pub fn new(public_key: &'a [u8], verify_token: &'a [u8]) -> EncryptionRequest<'a> {
        EncryptionRequest {
            public_key,
            verify_token,
        }
    }

    pub async fn write<W: AsyncWrite + Unpin>(
        &self,
        stream: &mut W,
        crypt: Option<&mut AesCfb8>,
    ) -> Result<(), std::io::Error> {
        let mut writer = PacketWriter::new(0x01);
        writer.var_i32(0); // Server ID String
        writer.var_buffer(self.public_key);
        writer.var_buffer(self.verify_token);
        writer.flush(stream, crypt).await
    }
}

pub struct LoginSuccess<'a> {
    uuid: &'a str,
    player_name: &'a str,
}

impl<'a> LoginSuccess<'a> {
    pub fn new(uuid: &'a str, player_name: &'a str) -> Self {
        Self { uuid, player_name }
    }

    pub async fn write<W: AsyncWrite + Unpin>(
        &self,
        stream: &mut W,
        crypt: Option<&mut AesCfb8>,
    ) -> Result<(), std::io::Error> {
        let mut writer = PacketWriter::new(0x02);
        writer.var_utf8(self.uuid);
        writer.var_utf8(self.player_name);
        writer.flush(stream, crypt).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;
    use std::io::Cursor;

    macro_rules! parse_tests {
        ($($name:ident, $take_fn:ident: $input:expr, $expected:expr),*) => {
            $(
                #[test]
                fn $name() {
                    assert_eq!(
                        $take_fn($input),
                        $expected
                    );
                }
            )*
        }
    }

    macro_rules! ok_tests {
        ($($name:ident, $take_fn:ident: $input:expr, $expected:expr, $remainder:expr),*) => {
            parse_tests! {
                $(
                    $name, $take_fn: $input, Ok(($remainder as &[u8], $expected))
                ),*
            }
        }
    }

    macro_rules! err_tests {
        ($($name:ident, $take_fn:ident: $input:expr, $expected:expr),*) => {
            parse_tests! {
                $(
                    $name, $take_fn: $input, Err(nom::Err::Error($expected))
                ),*
            }
        }
    }

    ok_tests! {
        take_packet_handshake_status, take_packet:
            b"\x07\x00\x05abcderemaining",
            Packet::LoginStart(LoginStart {
                player_name: "abcde"
            }),
            b"remaining"
    }

    err_tests! {
        take_packet_unknown_type, take_packet:
        b"\x07\xff\xff\xff\xff\x07remaining",
            ProtocolErrorKind::UnknownPacketType(b"remaining" as &[u8], 0x7fff_ffff)
    }

    macro_rules! write_tests {
        ($($name:ident: $input:expr, $expected:expr),*) => {
            $(
                #[test]
                fn $name() {
                    let mut target = Cursor::new(Vec::<u8>::new());
                    block_on(
                        $input.write(&mut target, None),
                    )
                    .unwrap();
                    assert_eq!(
                        target.into_inner(),
                        $expected as &[u8]
                    );
                }
            )*
        }
    }

    write_tests! {
        write_encryption_request: EncryptionRequest::new(b"test" as &[u8], b"value" as &[u8]), b"\x0d\x01\x00\x04test\x05value" as &[u8]
    }
    
    write_tests! {
        write_login_success: LoginSuccess::new("uuid", "player"), b"\x0d\x02\x04uuid\x06player" as &[u8]
    }
}
