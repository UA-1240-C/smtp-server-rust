use smart_stream::error::SmartStreamError;

#[derive(Debug)]
pub enum ClientConnectionError {
    ClosedConnection,
    SmartStream(SmartStreamError),
}

impl From<SmartStreamError> for ClientConnectionError {
    fn from(err: SmartStreamError) -> Self {
        Self::SmartStream(err)
    }
}
