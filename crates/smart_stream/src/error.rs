use std::{fmt::Display, net::AddrParseError, string::FromUtf8Error};

use async_std::future::TimeoutError;

#[derive(Debug)]
pub enum TlsError {
    NativeTls(native_tls::Error),
    StreamAlreadyEncrypted,
}

#[derive(Debug)]
pub enum SmartStreamError {
    Io(std::io::Error),
    Tls(TlsError),
    AddrParse(std::net::AddrParseError),
    Timeout(TimeoutError),
    CharsetConversion(FromUtf8Error),
    ClosedConnection(String),
    RuntimeError(String),
}

impl std::error::Error for SmartStreamError {}

impl Display for SmartStreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            writeln!(f, "{:?}", self)
    }
}

impl From<std::io::Error> for SmartStreamError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<native_tls::Error> for SmartStreamError {
    fn from(err: native_tls::Error) -> Self {
        Self::Tls(TlsError::NativeTls(err))
    }
}

impl From<AddrParseError> for SmartStreamError {
    fn from(err: AddrParseError) -> Self {
        Self::AddrParse(err)
    }
}

impl From<TimeoutError> for SmartStreamError {
    fn from(err: TimeoutError) -> Self {
        Self::Timeout(err)
    }
}

impl From<FromUtf8Error> for SmartStreamError {
    fn from(err: FromUtf8Error) -> Self {
        Self::CharsetConversion(err)
    }
}
