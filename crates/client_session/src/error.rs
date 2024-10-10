use smart_stream::error::SmartStreamError;
use mail_database::MailError;

#[derive(Debug)]
pub enum ClientSessionError {
    ClosedConnection,
    SmartStream(SmartStreamError),
    DataBase(MailError),
}

impl From<SmartStreamError> for ClientSessionError {
    fn from(err: SmartStreamError) -> Self {
        Self::SmartStream(err)
    }
}

impl From<MailError> for ClientSessionError {
    fn from(err: MailError) -> Self {
        Self::DataBase(err)
    }
}
