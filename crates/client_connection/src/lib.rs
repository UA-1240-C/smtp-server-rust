use std::sync::LazyLock;

use smart_stream::AsyncStream;
use request_parser::RequestType;
use error_adapter::Error;

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

#[derive(Default)]
pub struct ConnectionData {
    // logged_user: String,
    pub mail_from: String,
    pub rcpt_to: Vec<String>,
    pub data: String,
}

pub struct ClientConnection {
    current_state: ClientState,
    connection: Option<AsyncStream>,
    connection_data: ConnectionData,
}

impl ClientConnection {
    pub async fn new(connection: AsyncStream) -> Self {
        Self {
            current_state: ClientState::Connected,
            connection: Some(connection),
            connection_data: ConnectionData::default(),
        }
    }

    async fn handle_new_request(&mut self) -> Result<(), Error> {
        let connection = self.connection.as_mut().ok_or(Error::ClosedConnection("Connection is closed".into()))?;
        let raw_request = connection.read().await; // вичитує
        let request = RequestType::parse(&raw_request.unwrap()); // парсить
        println!("{:?}", request);


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
                let _ = connection.write([b"500 Error\r\n", err.as_bytes()].concat().as_ref()).await;
            }
        }
        Ok(())
        
    }



    pub async fn run(&mut self) -> Result<(), Error> {
        let connection = self.connection.as_mut().ok_or(Error::ClosedConnection("Connection is closed".into()))?;
        connection.write(b"220 SMTP server ready\r\n").await?;
        loop {
            if self.connection.is_none() {
                return Ok(());
            }
            self.handle_new_request().await?;
        }
    }

    async fn handle_following_connected(&mut self, request: &RequestType) -> Result<(), Error> {
        let connection = self.connection.as_mut().ok_or(Error::ClosedConnection("Connection is closed".into()))?;
        match request {
            _ => {
                let _ = connection.write(b"500 Error\r\n").await;
            }
        }
        Ok(())
    }

    async fn handle_following_ehlo(&mut self, request: &RequestType) -> Result<(), Error> {
        let connection = self.connection.as_mut().ok_or(Error::ClosedConnection("Connection is closed".into()))?;
        match request {
            RequestType::STARTTLS => {
                let _ = connection.write(b"220 Ready to start TLS\r\n").await;
                self.current_state = ClientState::StartTLS;

                connection.accept_tls(IDENTITY.clone()).await.unwrap();
            },
            _ => {
                let _ = connection.write(b"500 Error\r\n").await;
            }
        }
        Ok(())
    }

    async fn handle_following_starttls(&mut self, request: &RequestType) -> Result<(), Error> {
        let connection = self.connection.as_mut().ok_or(Error::ClosedConnection("Connection is closed".into()))?;
        match request {
            // TODO: check in database if user exists
            RequestType::AUTH_PLAIN(_) => {
                self.current_state = ClientState::Auth;
                let _ = connection.write(b"235 OK\r\n").await;
            },
            RequestType::REGISTER(_) => {
                self.current_state = ClientState::Auth;
                let _ = connection.write(b"235 OK\r\n").await;
            },
            _ => {
                let _ = connection.write(b"500 Error\r\n").await;  
            }
        }
        Ok(())
    }

    async fn handle_following_auth(&mut self, request: &RequestType) -> Result<(), Error> {
        let connection = self.connection.as_mut().ok_or(Error::ClosedConnection("Connection is closed".into()))?;
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

    async fn handle_following_mail_from(&mut self, request: &RequestType) -> Result<(), Error> {
        let connection = self.connection.as_mut().ok_or(Error::ClosedConnection("Connection is closed".into()))?;
        match request {
            RequestType::RCPT_TO(rcpt_to) => {
                self.connection_data.rcpt_to.push(rcpt_to.clone());
                self.current_state = ClientState::RcptTo;
                let _ = connection.write(b"250 OK\r\n").await;
            },
            _ => {
                let _ = connection.write(b"500 Error\r\n").await;
            }
        }
        Ok(())
    }

    async fn handle_following_rcpt_to(&mut self, request: &RequestType) -> Result<(), Error> {
        let connection = self.connection.as_mut().ok_or(Error::ClosedConnection("Connection is closed".into()))?;
        match request {
            RequestType::RCPT_TO(rcpt_to) => {
                self.connection_data.rcpt_to.push(rcpt_to.clone());
                self.current_state = ClientState::RcptTo;
                let _ = connection.write(b"250 OK\r\n").await;
            },
            RequestType::DATA => {
                let _ = connection.write(b"354 End data with <CR><LF>.<CR><LF>\r\n").await;    
                let result = Self::read_data_until_dot(connection).await;

                match result {
                    Ok(data) => {
                        self.connection_data.data = data;
                        self.current_state = ClientState::Data;
                        let _ = connection.write(b"250 OK\r\n").await;
                        // TODO: save email to database
                    },
                    Err(err) => {
                        let _ = connection.write([b"500 Error\r\n", err.as_bytes()].concat().as_ref()).await;
                    }
                } 
            },
            _ => {
                let _ = connection.write(b"500 Error\r\n").await;
            }
        }
        Ok(())
    }

    async fn handle_following_data(&mut self, request: &RequestType) -> Result<(), Error> {
        let connection = self.connection.as_mut().ok_or(Error::ClosedConnection("Connection is closed".into()))?;
        match request {
            RequestType::MAIL_FROM(mail_from) => {
                self.current_state = ClientState::MailFrom;
                self.connection_data = ConnectionData::default();
                self.connection_data.rcpt_to.push(mail_from.clone());
                let _ = connection.write(b"250 OK\r\n").await;
            },
            _ => {
                let _ = connection.write(b"500 Error\r\n").await;
            }
        }
        Ok(())
    }

    async fn read_data_until_dot(stream: &mut AsyncStream) -> Result<String, String> {
        const MAX_SIZE: usize = 1024 * 1024 * 2;
        let mut data = String::new();
        loop {
            let line = stream.read().await;
            
            if let Ok(line) = line {
                if line.ends_with("\r\n.\r\n") {
                    println!("Data read");
                    data.push_str(&line[..line.len() - 5]);
                    break;
                }
                data.push_str(&line);
            }
            if data.len() > MAX_SIZE {
                return Err("Data size is too big".into());
            }
        }
        return Ok(data);
    }

    async fn handle_following_quit(&mut self, request: &RequestType) -> Result<(), Error> {
        match request {
            _ => {
                unreachable!("Should not accept any commands after QUIT");
            }
        }
    }

    async fn handle_if_loose(&mut self, request: &RequestType) -> Result<bool, Error> {
        let connection = self.connection.as_mut().ok_or(Error::ClosedConnection("Connection is closed".into()))?;
        match request {
            RequestType::EHLO(_) => {
                self.current_state = ClientState::Ehlo;
                self.connection_data = ConnectionData::default();
                connection.write(b"250 OK\r\n").await?;
            },
            RequestType::QUIT => {
                self.current_state = ClientState::Quit;
                connection.write(b"221 OK\r\n").await?;
                self.connection.take();
            },
            RequestType::HELP => {
                connection.write(b"214 OK\r\n").await?;
            },
            RequestType::NOOP => {
                connection.write(b"250 OK\r\n").await?;
            },
            RequestType::RSET => {  
                self.current_state = ClientState::Connected;
                self.connection_data = ConnectionData::default();
                connection.write(b"250 OK\r\n").await?;
            },
            _ => {
                return Ok(false);
            }
        }
        return Ok(true);
    }




}



static IDENTITY: LazyLock<native_tls::Identity> = LazyLock::new(|| native_tls::Identity::from_pkcs8(
    include_bytes!("../../../server/certs/server.crt"),
    include_bytes!("../../../server/certs/server.key"),
).unwrap());