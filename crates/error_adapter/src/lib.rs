use std::fmt::Display;
use std::num::ParseFloatError;
use std::{fmt::Display, net::AddrParseError};

#[derive(Debug, PartialEq)]
pub enum JsonErrorType {
    ParseError,
    UnreachableChild,
}

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Tls(native_tls::Error),
    TlsUpgrade(String),
    AddrParseError(std::net::AddrParseError),
    ClosedConnection(String),
    RuntimeError(String),
    JsonError(JsonErrorType),
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Error::JsonError(a), Error::JsonError(b)) => { a == b },
            (Error::Io(_), Error::Io(_)) => false,
            (Error::Tls(_), Error::Tls(_)) => false,
            (Error::TlsUpgrade(a), Error::TlsUpgrade(b)) => a == b,
            (Error::AddrParseError(a), Error::AddrParseError(b)) => a == b,
            (Error::ClosedConnection(a), Error::ClosedConnection(b)) => a == b,
            _ => false,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            writeln!(f, "{:?}", self)
    }
}

impl From<ParseFloatError> for Error {
    fn from(_err: ParseFloatError) -> Self {
        Error::JsonError(JsonErrorType::ParseError)
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<native_tls::Error> for Error {
    fn from(err: native_tls::Error) -> Self {
        Error::Tls(err)
    }
}

impl From<AddrParseError> for Error {
    fn from(err: AddrParseError) -> Self {
        Error::AddrParseError(err)
    }
}
