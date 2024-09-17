use logger_proc_macro::log;
use smart_stream::AsyncStream;
use request_parser::RequestType;
use async_native_tls::TlsAcceptor;
use mail_database::{IMailDB, PgMailDB};
use base64::decode;

pub mod error;
use error::ClientSessionError;

#[derive(Debug)]
enum ClientState {
    Connected,
    Ehlo,
    StartTLS,
    Auth,
    MailFrom,
    RcptTo,
    Data,
    Quit,
}

#[derive(Default, Debug)]
pub struct SessionData {
    logged_user: String,
    pub mail_from: String,
    pub rcpt_to: Vec<String>,
    pub data: String,
}

pub struct ClientSession {
    current_state: ClientState,
    connection: Option<AsyncStream>,
    connection_data: SessionData,
    tls_acceptor: TlsAcceptor,
    db_connection: PgMailDB,
}

impl ClientSession {
    #[log(debug)]
    pub fn new(connection: AsyncStream, tls_acceptor: &TlsAcceptor, connection_string: &str)
    -> Result<Self, ClientSessionError> {
        let mut pg = PgMailDB::new("localhost".to_string());
        pg.connect(connection_string)?;
        
        Ok(Self {
            current_state: ClientState::Connected,
            connection: Some(connection),
            connection_data: SessionData::default(),
            tls_acceptor: tls_acceptor.clone(),
            db_connection: pg,
        })
    }

    #[log(trace)]
    async fn handle_new_request(&mut self) -> Result<(), ClientSessionError> {
        let connection = self.connection.as_mut().ok_or(ClientSessionError::ClosedConnection)?;
        let raw_request = connection.read().await?;
        let request = RequestType::parse(&raw_request);

        match request {
            Ok(request) => {
                // commands that can be executed in any state
                if self.handle_if_loose(&request).await? {
                    return Ok(());
                }
            
                match self.current_state {
                    ClientState::Connected => { self.handle_following_connected(&request).await?; },
                    ClientState::Ehlo => { self.handle_following_ehlo(&request).await?; },
                    ClientState::StartTLS => { self.handle_following_starttls(&request).await?; },
                    ClientState::Auth => { self.handle_following_auth(&request).await?; },
                    ClientState::MailFrom => { self.handle_following_mail_from(&request).await?; },
                    ClientState::RcptTo => { self.handle_following_rcpt_to(&request).await?; },
                    ClientState::Data => { self.handle_following_data(&request).await?; },
                    ClientState::Quit => { self.handle_following_quit(&request).await?; },
                }
            },
            Err(err) => {
                connection.write(format!("500 Error {}\r\n", err).as_bytes()).await?;
            }
        }
        Ok(())
    }
    
    #[log(trace)]
    pub async fn run(&mut self) -> Result<(), ClientSessionError> {
        let connection = self.connection.as_mut().ok_or(ClientSessionError::ClosedConnection)?;
        connection.write(b"220 SMTP server ready\r\n").await?;
        while let Some(connection) = &self.connection {
            if !connection.is_open() {
                break;
            }
            self.handle_new_request().await?;
        }
        Ok(())
    }

    #[log(trace)]
    async fn handle_following_connected(&mut self, _request: &RequestType) -> Result<(), ClientSessionError> {
        let connection = self.connection.as_mut().ok_or(ClientSessionError::ClosedConnection)?;
        connection.write(b"500 Error\r\n").await?;
        Ok(())
    }

    #[log(trace)]
    async fn handle_following_ehlo(&mut self, request: &RequestType) -> Result<(), ClientSessionError> {
        let connection = self.connection.as_mut().ok_or(ClientSessionError::ClosedConnection)?;
        match request {
            RequestType::STARTTLS => {
                connection.write(b"220 Ready to start TLS\r\n").await?;
                self.current_state = ClientState::StartTLS;

                connection.accept_tls(&self.tls_acceptor).await?;
            },
            _ => {
                connection.write(b"500 Error\r\n").await?;
            }
        }
        Ok(())
    }

    #[log(trace)]
    async fn handle_following_starttls(&mut self, request: &RequestType) -> Result<(), ClientSessionError> {
        let connection = self.connection.as_mut().ok_or(ClientSessionError::ClosedConnection)?;
        match request {
            RequestType::AUTH_PLAIN(cred_string) => {
                match decode(cred_string) {
                    Ok(cred) => {
                        let cred: Vec<&str> = cred.split("\0").collect();
                        let user = cred[1];
                        let pass = cred[2];
                        if self.db_connection.login(user, pass).is_ok() {
                            self.current_state = ClientState::Auth;
                            self.connection_data.logged_user = user.to_string();
                            connection.write(b"235 OK\r\n").await?;
                        } else {
                            connection.write(b"500 Error user not found\r\n").await?;
                        }
                    },
                    Err(_) => {
                        connection.write(b"500 Error could not decode credentials\r\n").await?;
                    }
                }
                self.current_state = ClientState::Auth;
            },
            RequestType::REGISTER(_) => {
                self.current_state = ClientState::Auth;
                connection.write(b"235 OK\r\n").await?;
            },
            _ => {
                connection.write(b"500 Error\r\n").await?; 
            }
        }
        Ok(())
    }

    #[log(trace)]
    async fn handle_following_auth(&mut self, request: &RequestType) -> Result<(), ClientSessionError> {
        let connection = self.connection.as_mut().ok_or(ClientSessionError::ClosedConnection)?;
        match request {
            RequestType::MAIL_FROM(_) => {
                self.current_state = ClientState::MailFrom;
                connection.write(b"250 OK\r\n").await?;
            },
            _ => {
                connection.write(b"500 Error\r\n").await?;
            },
            
        }
        Ok(())
    }

    #[log(trace)]
    async fn handle_following_mail_from(&mut self, request: &RequestType) -> Result<(), ClientSessionError> {
        let connection = self.connection.as_mut().ok_or(ClientSessionError::ClosedConnection)?;
        match request {
            RequestType::RCPT_TO(rcpt_to) => {
                self.connection_data.rcpt_to.push(rcpt_to.clone());
                self.current_state = ClientState::RcptTo;
                connection.write(b"250 OK\r\n").await?;
            },
            _ => {
                connection.write(b"500 Error\r\n").await?;
            }
        }
        Ok(())
    }

    #[log(trace)]
    async fn handle_following_rcpt_to(&mut self, request: &RequestType) -> Result<(), ClientSessionError> {
        let connection = self.connection.as_mut().ok_or(ClientSessionError::ClosedConnection)?;
        match request {
            RequestType::RCPT_TO(rcpt_to) => {
                self.connection_data.rcpt_to.push(rcpt_to.clone());
                self.current_state = ClientState::RcptTo;
                connection.write(b"250 OK\r\n").await?;
            },
            RequestType::DATA => {
                connection.write(b"354 End data with <CR><LF>.<CR><LF>\r\n").await?; 
                let result = Self::read_data_until_dot(connection).await;

                match result {
                    Ok(data) => {
                        self.connection_data.data = data;
                        self.current_state = ClientState::Data;
                        connection.write(b"250 OK\r\n").await?;
  
                        let subject = &self.connection_data.data.lines()
                                                .find(|x| x.starts_with("Subject: "))
                                                .unwrap_or("Subject: No Subject")[9..];

                        self.db_connection.insert_multiple_emails(
                                self.connection_data.rcpt_to.iter().map(|x| &x[..]).collect(), 
                                subject, 
                                &self.connection_data.data
                            )?;
                    },
                    Err(err) => {
                        connection.write([b"500 Error\r\n", err.as_bytes()].concat().as_ref()).await?;
                    }
                } 
            },
            _ => {
                connection.write(b"500 Error\r\n").await?;
            }
        }
        Ok(())
    }

    #[log(trace)]
    async fn handle_following_data(&mut self, request: &RequestType) -> Result<(), ClientSessionError> {
        let connection = self.connection.as_mut().ok_or(ClientSessionError::ClosedConnection)?;
        match request {
            RequestType::MAIL_FROM(mail_from) => {
                self.current_state = ClientState::MailFrom;
                self.connection_data = SessionData::default();
                self.connection_data.rcpt_to.push(mail_from.clone());
                connection.write(b"250 OK\r\n").await?;
            },
            _ => {
                connection.write(b"500 Error\r\n").await?;
            }
        }
        Ok(())
    }

    #[log(trace)]
    async fn handle_following_quit(&mut self, _request: &RequestType) -> Result<(), ClientSessionError> {
        unreachable!("Should not accept any commands after QUIT");
    }

    #[log(trace)]
    async fn handle_if_loose(&mut self, request: &RequestType) -> Result<bool, ClientSessionError> {
        let connection = self.connection.as_mut().ok_or(ClientSessionError::ClosedConnection)?;
        match request {
            RequestType::EHLO(_) => {
                self.current_state = ClientState::Ehlo;
                self.connection_data = SessionData::default();
                connection.write(b"250 OK\r\n").await?;
            },
            RequestType::QUIT => {
                self.current_state = ClientState::Quit;
                connection.write(b"221 OK\r\n").await?;
                self.connection.take();
                self.db_connection.disconnect();
            },
            RequestType::HELP => {
                connection.write(b"214 OK\r\n").await?;
            },
            RequestType::NOOP => {
                connection.write(b"250 OK\r\n").await?;
            },
            RequestType::RSET => {  
                self.current_state = ClientState::Connected;
                self.connection_data = SessionData::default();
                connection.write(b"250 OK\r\n").await?;
            },
            _ => {
                return Ok(false);
            }
        }
        Ok(true)
    }

    #[log(debug)]
    async fn read_data_until_dot(stream: &mut AsyncStream) -> Result<String, String> {
        const MAX_SIZE: usize = 1024 * 1024 * 2;
        let mut data = String::new();
        loop {
            let line = stream.read().await;
            
            if let Ok(line) = line {
                if line.ends_with("\r\n.\r\n") {
                    data.push_str(&line[..line.len() - 5]);
                    break;
                }
                data.push_str(&line);
            }
            if data.len() > MAX_SIZE {
                return Err("Data size is too big".into());
            }
        }
        Ok(data)
    }
}
