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

impl From<ErrorKind> for Error {
    fn from(value: ErrorKind) -> Self {
        Self { kind: value }
    }
}

impl<T: std::error::Error + 'static> From<Box<T>> for Error {
    fn from(value: Box<T>) -> Self {
        Self {
            kind: ErrorKind::Error(value),
        }
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    Error(Box<dyn std::error::Error>),
    PendingInsertion,
    InvalidLengthPrefix,
    LengthTooLarge,
    ReadPastPacket,
    EndOfData,
    InvalidVarint
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Self::Error(e) => write!(f, "{}", e),
            Self::PendingInsertion => write!(f, "an insertion was not completed"),
            Self::InvalidLengthPrefix => write!(f, "invalid length prefix"),
            Self::LengthTooLarge => write!(f, "length would be too large"),
            Self::ReadPastPacket => write!(f, "read past the end of a packet"),
            Self::EndOfData => write!(f, "end of data"),
            Self::InvalidVarint => write!(f, "invalid varint"),
        }
    }
}
