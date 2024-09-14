use std::num::ParseFloatError;

#[derive(Debug, PartialEq)]
pub enum JsonError {
    ParseError,
    BrokenTree,
}

impl From<ParseFloatError> for JsonError {
    fn from(_err: ParseFloatError) -> Self {
        JsonError::ParseError
    }
}
