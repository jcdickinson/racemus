use std::str::Utf8Error;

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
}

impl Error {
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", self.kind)
    }
}

impl From<std::io::Error> for Error {
    fn from(val: std::io::Error) -> Self {
        Self {
            kind: ErrorKind::IOError(val),
        }
    }
}

impl From<ErrorKind> for Error {
    fn from(value: ErrorKind) -> Self {
        Self { kind: value }
    }
}

impl From<cesu8::Cesu8DecodingError> for Error {
    fn from(value: cesu8::Cesu8DecodingError) -> Self {
        Self {
            kind: ErrorKind::InvalidCesu8String(value),
        }
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    PendingInsertion,
    InvalidLengthPrefix,
    LengthTooLarge,
    ReadPastPacket,
    EndOfData,
    InvalidVarint,
    InvalidKey,
    InvalidOperation,
    CompressedDataTooLarge,
    InvalidNbt,
    InvalidState(i32),
    IOError(std::io::Error),
    InvalidString(Utf8Error),
    InvalidCesu8String(cesu8::Cesu8DecodingError),
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Self::PendingInsertion => write!(f, "an insertion was not completed"),
            Self::InvalidLengthPrefix => write!(f, "invalid length prefix"),
            Self::LengthTooLarge => write!(f, "length prefix too large"),
            Self::ReadPastPacket => write!(f, "read past the end of a packet"),
            Self::EndOfData => write!(f, "end of data"),
            Self::InvalidVarint => write!(f, "invalid varint"),
            Self::InvalidKey => write!(f, "invalid encryption key"),
            Self::InvalidOperation => write!(f, "invalid operation"),
            Self::CompressedDataTooLarge => write!(f, "compressed data too large"),
            Self::InvalidNbt => write!(f, "invalid NBT"),
            Self::InvalidState(s) => write!(f, "invalid state: {}", s),
            Self::IOError(e) => write!(f, "I/O error: {}", e),
            Self::InvalidString(e) => write!(f, "invalid string: {}", e),
            Self::InvalidCesu8String(e) => write!(f, "invalid CESU8 string: {}", e),
        }
    }
}
