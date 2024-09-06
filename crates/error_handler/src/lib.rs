use std::fmt::Display;
use std::num::ParseFloatError;

#[derive(Debug, PartialEq)]
pub enum JsonErrorType {
    ParseError,
    UnreachableChild,
}

#[derive(Debug)]
pub enum Error {
    JsonError(JsonErrorType),
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Error::JsonError(a), Error::JsonError(b)) => { a == b },
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
