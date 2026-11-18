use std::num::ParseIntError;
use std::str::Utf8Error;
use std::{fmt, io};

#[derive(Debug)]
pub enum Error {
    ProtocolError,
    ClientClosed,
    SendBufferFull,
    IOError(io::Error),
    EncodingError(Utf8Error),
    ConnectionTimeout,
    HostResolutionFailed,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProtocolError => f.write_str("ProtocolError"),
            Self::ClientClosed => f.write_str("ClientClosed"),
            Self::SendBufferFull => f.write_str("SendBufferFull"),
            Self::IOError(err) => fmt::Display::fmt(err, f),
            Self::EncodingError(err) => fmt::Display::fmt(err, f),
            Self::ConnectionTimeout => f.write_str("ConnectionTimeout"),
            Self::HostResolutionFailed => f.write_str("HostResolutionFailed"),
        }
    }
}

impl_more::impl_error_enum! {
    Error:
    IOError(err) => err,
    EncodingError(err) => err,
}

impl_more::impl_enum_from!(io::Error => Error::IOError);
impl_more::impl_enum_from!(Utf8Error => Error::EncodingError);

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
