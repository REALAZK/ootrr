use std::{fmt, io};

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Format(&'static str),
    BadPath(String),
    Zstd(String),
    Msg(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "io: {e}"),
            Error::Format(m) => write!(f, "format: {m}"),
            Error::BadPath(p) => write!(f, "bad path: {p}"),
            Error::Zstd(m) => write!(f, "zstd: {m}"),
            Error::Msg(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}

pub type Res<T> = Result<T, Error>;
