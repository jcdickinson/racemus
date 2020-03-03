use crate::protocol::extensions::{take_fix_u16, take_var_i32};
use crate::protocol::protocol_error::ProtocolErrorKind;

#[derive(Debug, PartialEq, Eq)]
pub enum Packet<'a> {
    Handshake(Handshake<'a>),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Handshake<'a> {
    version: i32,
    addr: &'a str,
    port: u16,
    next_state: RequestedState,
}

impl<'a> Handshake<'a> {
    pub fn version(&'a self) -> i32 {
        self.version
    }
    pub fn addr(&'a self) -> &'a str {
        &self.addr
    }
    pub fn port(&'a self) -> u16 {
        self.port
    }
    pub fn next_state(&'a self) -> RequestedState {
        self.next_state
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestedState {
    Status,
    Login,
}

build_utf8!(take_server_address, 255);

build_packet_parser!(i:
    0x00 => {
        let (i, version) = take_var_i32(i)?;
        let (i, addr) = take_server_address(i)?;
        let (i, port) = take_fix_u16(i)?;
        let (i, next_state) = take_var_i32(i)?;
        let next_state = match next_state {
            0x01 => RequestedState::Status,
            0x02 => RequestedState::Login,
            _ => {
                return Err(nom::Err::Error(ProtocolErrorKind::UnknownStatusType(
                    i, next_state,
                )))
            }
        };
        Ok((
            i,
            Packet::Handshake(Handshake {
                version,
                addr,
                port,
                next_state,
            }),
        ))
    }
);

#[cfg(test)]
mod tests {
    use super::*;

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
            b"\x0b\x00\x02\x05abcde\xaa\xbb\x01remaining",
            Packet::Handshake(Handshake {
                version: 0x02,
                addr: "abcde",
                port: 0xaabb,
                next_state: RequestedState::Status
            }),
            b"remaining",
        take_packet_handshake_login, take_packet:
            b"\x0b\x00\x02\x05abcde\xaa\xbb\x02remaining",
            Packet::Handshake(Handshake {
                version: 0x02,
                addr: "abcde",
                port: 0xaabb,
                next_state: RequestedState::Login
            }),
            b"remaining"
    }

    err_tests! {
        take_packet_unknown_type, take_packet:
        b"\x07\xff\xff\xff\xff\x07remaining",
            ProtocolErrorKind::UnknownPacketType(b"remaining" as &[u8], 0x7fff_ffff),
        take_packet_handshake_invalid_state, take_packet:
            b"\x0b\x00\x02\x05abcde\xaa\xbb\x03remaining",
            ProtocolErrorKind::UnknownStatusType(b"remaining" as &[u8], 0x03)
    }
}
