macro_rules! build_packet_parser {
    ($input:ident: $($id:literal => $handle:expr),*) => {
        pub fn take_packet<'a>(
            i: &'a [u8],
        ) -> nom::IResult<&'a [u8], Packet<'a>, ProtocolErrorKind<&'a [u8]>> {
            let (i, len) = take_var_i32(i)?;
            if len <= 0 {
                return Err(nom::Err::Error(ProtocolErrorKind::NegativeLengthPacket(i)));
            }
            let len = len as usize;
            if i.len() < len {
                return Err(nom::Err::Incomplete(nom::Needed::Size(len)));
            }
            let ($input, typ) = take_var_i32(i)?;
            match typ {
                $(
                    $id => $handle,
                )*
                _ => Err(nom::Err::Error(ProtocolErrorKind::UnknownPacketType(
                    $input, typ,
                ))),
            }
        }
    };
}

pub mod login;
pub mod open;

use crate::protocol::writers::{AesCfb8, PacketWriter};
use tokio::io::AsyncWrite;

pub struct Disconnect<'a> {
    login: bool,
    reason: &'a str,
}

impl<'a> Disconnect<'a> {
    pub fn login(reason: &'a str) -> Self {
        Self {
            login: true,
            reason,
        }
    }

    pub fn play(reason: &'a str) -> Self {
        Self {
            login: false,
            reason,
        }
    }

    pub async fn write<W: AsyncWrite + Unpin>(
        &self,
        stream: &mut W,
        crypt: Option<&mut AesCfb8>,
    ) -> Result<(), std::io::Error> {
        let mut writer = PacketWriter::new(if self.login { 0x00 } else { 0x1b });
        writer.var_utf8(self.reason);
        writer.flush(stream, crypt).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;
    use std::io::Cursor;

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
        write_disconnect_login: Disconnect::login("bad!"), b"\x06\x00\x04bad!" as &[u8],
        write_disconnect_play: Disconnect::play("bad?"), b"\x06\x1b\x04bad?" as &[u8]
    }
}
