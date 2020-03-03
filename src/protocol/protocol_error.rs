use nom::error::{ErrorKind, ParseError};
use std::error::Error;

#[derive(Debug, PartialEq, Eq)]
pub enum ProtocolErrorKind<I> {
    ParserError(I, ErrorKind),
    StringInvalid(I, std::str::Utf8Error),
    VarIntTooLarge(I),
    StringTooLarge(I),
    NegativeLengthPacket(I),
    UnknownPacketType(I, i32),
    UnknownStatusType(I, i32),
}

impl<I> ParseError<I> for ProtocolErrorKind<I> {
    fn from_error_kind(input: I, kind: ErrorKind) -> Self {
        ProtocolErrorKind::ParserError(input, kind)
    }

    fn append(input: I, kind: ErrorKind, _other: Self) -> Self {
        ProtocolErrorKind::ParserError(input, kind)
    }

    fn from_char(input: I, _: char) -> Self {
        Self::from_error_kind(input, ErrorKind::Char)
    }

    fn or(self, other: Self) -> Self {
        other
    }

    fn add_context(_input: I, _ctx: &'static str, other: Self) -> Self {
        other
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProtocolError {
    ParserError(ErrorKind),
    StringInvalid(std::str::Utf8Error),
    VarIntTooLarge,
    StringTooLarge,
    NegativeLengthPacket,
    UnknownPacketType(i32),
    UnknownStatusType(i32),
}

impl<I> Into<ProtocolError> for ProtocolErrorKind<I> {
    fn into(self) -> ProtocolError {
        match self {
            Self::ParserError(_, a) => ProtocolError::ParserError(a),
            Self::StringInvalid(_, a) => ProtocolError::StringInvalid(a),
            Self::UnknownPacketType(_, a) => ProtocolError::UnknownPacketType(a),
            Self::UnknownStatusType(_, a) => ProtocolError::UnknownStatusType(a),
            Self::VarIntTooLarge(_) => ProtocolError::VarIntTooLarge,
            Self::StringTooLarge(_) => ProtocolError::StringTooLarge,
            Self::NegativeLengthPacket(_) => ProtocolError::NegativeLengthPacket,
        }
    }
}

impl<I> Into<Box<dyn Error>> for ProtocolErrorKind<I> {
    fn into(self) -> Box<dyn Error> {
        Box::<ProtocolError>::new(self.into())
    }
}

impl Error for ProtocolError {}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Self::ParserError(e) => write!(f, "ParserError({:?})", e),
            Self::StringInvalid(e) => write!(f, "StringInvalid({})", e),
            Self::UnknownPacketType(e) => write!(f, "UnknownPacketType({})", e),
            Self::UnknownStatusType(e) => write!(f, "UnknownStatusType({})", e),
            Self::VarIntTooLarge => write!(f, "VarIntTooLarge"),
            Self::StringTooLarge => write!(f, "StringTooLarge"),
            Self::NegativeLengthPacket => write!(f, "NegativeLengthPacket"),
        }
    }
}
