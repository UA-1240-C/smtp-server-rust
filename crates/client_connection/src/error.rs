use smart_stream::error::SmartStreamError;
use mail_database::MailError;

#[derive(Debug)]
pub enum ClientConnectionError {
    ClosedConnection,
    SmartStream(SmartStreamError),
    DataBase(MailError),
}

impl From<SmartStreamError> for ClientConnectionError {
    fn from(err: SmartStreamError) -> Self {
        Self::SmartStream(err)
    }
}

impl From<MailError> for ClientConnectionError {
    fn from(err: MailError) -> Self {
        Self::DataBase(err)
    }
}
