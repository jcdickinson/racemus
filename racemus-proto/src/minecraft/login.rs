#![allow(clippy::too_many_arguments)]

use crate::{PacketReader, PacketWriter};
use async_std::io::{Read, Write};
use std::{
    io::{Error, ErrorKind},
    marker::Unpin,
    sync::Arc,
};

#[derive(Debug, PartialEq, Eq)]
pub enum Packet {
    LoginStart(LoginStart),
    EncryptionResponse(EncryptionResponse),
}

#[derive(Debug, PartialEq, Eq)]
pub struct LoginStart {
    player_name: Arc<Box<str>>,
}

impl LoginStart {
    pub fn player_name(&self) -> &Arc<Box<str>> {
        &self.player_name
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct EncryptionResponse {
    encrypted_shared_secret: Arc<Box<[u8]>>,
    encrypted_verifier: Arc<Box<[u8]>>,
}

impl EncryptionResponse {
    pub fn encrypted_shared_secret(&self) -> &[u8] {
        self.encrypted_shared_secret.as_ref()
    }
    pub fn encrypted_verifier(&self) -> &[u8] {
        self.encrypted_verifier.as_ref()
    }
}

pub async fn read_packet<R: Read + Unpin>(reader: &mut PacketReader<R>) -> Result<Packet, Error> {
    match reader.packet_header().await? {
        0x00 => {
            let player_name = reader.var_arr_char(Some(16)).await?;
            Ok(Packet::LoginStart(LoginStart { player_name }))
        }
        0x01 => {
            let encrypted_shared_secret = reader.var_arr_u8(Some(128)).await?;
            let encrypted_verifier = reader.var_arr_u8(Some(128)).await?;
            Ok(Packet::EncryptionResponse(EncryptionResponse {
                encrypted_shared_secret,
                encrypted_verifier,
            }))
        }
        _ => Err(ErrorKind::InvalidData.into()),
    }
}

pub async fn write_encryption_request<W: Write + Unpin>(
    writer: &mut PacketWriter<W>,
    public_key: &[u8],
    verify_token: &[u8],
) -> Result<(), Error> {
    writer
        .packet_id(0x01)
        .var_i32(0) // Server ID
        .var_arr_u8(public_key)
        .var_arr_u8(verify_token)
        .flush_length_prefixed()
        .await
}

pub async fn write_login_success<W: Write + Unpin>(
    writer: &mut PacketWriter<W>,
    uuid: &str,
    player_name: &str,
) -> Result<(), Error> {
    writer
        .packet_id(0x02)
        .var_arr_char(uuid)
        .var_arr_char(player_name)
        .flush_length_prefixed()
        .await
}

pub async fn write_disconnect<W: Write + Unpin>(
    writer: &mut PacketWriter<W>,
    reason: &str,
) -> Result<(), std::io::Error> {
    writer.packet_id(0x00).var_arr_char(reason);
    writer.flush_length_prefixed().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_std::io::Cursor;
    use async_std::task::block_on;

    macro_rules! write_tests {
        ($($name:ident: $fn:ident( $($param:expr),* ), $expected:expr),*) => {
            $(
                #[test]
                fn $name() {
                    let target = Cursor::new(Vec::<u8>::new());
                    let mut writer = PacketWriter::new(target);
                    block_on(
                        $fn(
                            &mut writer,
                           $(
                            $param
                           ),*
                        )
                    ).unwrap();
                    assert_eq!(
                        writer.into_inner().into_inner(),
                        $expected as &[u8]
                    );
                }
            )*
        }
    }

    write_tests! {
        write_disconnect_test: write_disconnect("bad!"), b"\x06\x00\x04bad!" as &[u8]
    }
}
