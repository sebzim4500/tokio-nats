use std::num::ParseIntError;
use std::str::Utf8Error;

#[derive(Debug)]
pub enum Error {
    ProtocolError,
    ClientClosed,
    SendBufferFull,
    IOError(std::io::Error),
    EncodingError(Utf8Error),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IOError(err)
    }
}

impl From<Utf8Error> for Error {
    fn from(err: Utf8Error) -> Self {
        Error::EncodingError(err)
    }
}

impl From<ParseIntError> for Error {
    fn from(_: ParseIntError) -> Self {
        Error::ProtocolError
    }
}

impl From<serde_json::error::Error> for Error {
    fn from(_: serde_json::error::Error) -> Error {
        Error::ProtocolError
    }
}
