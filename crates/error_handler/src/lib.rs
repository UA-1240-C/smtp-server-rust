use std::fmt::Display;

#[derive(Debug, PartialEq)]
pub enum AtomicCounterErrorType {
    DecrementError,
    IncrementError,
}

#[derive(Debug)]
pub enum Error {
    AtomicCounterError(AtomicCounterErrorType),
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Error::AtomicCounterError(a), Error::AtomicCounterError(b)) => a == b,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            writeln!(f, "{:?}", self)
    }
}
