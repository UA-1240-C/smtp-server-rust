use core::panic;
use std::sync::LazyLock;

use smart_stream::AsyncStream;
use request_parser::RequestType;

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
    logged_user: String,
    pub mail_from: String,
    pub rcpt_to: Vec<String>,
    pub data: String,
}

pub struct ClientConnection {
    current_state: ClientState,
    connection: AsyncStream,
    connection_data: ConnectionData,
}

impl ClientConnection {
    pub async fn new(connection: AsyncStream) -> Self {
        Self {
            current_state: ClientState::Connected,
            connection,
            connection_data: ConnectionData::default(),
        }
    }

    async fn handle_new_request(&mut self) {
        let raw_request = self.connection.read().await; // вичитує
        println!("{:?}", raw_request);
        let request = RequestType::parse(&raw_request.unwrap()); // парсить
        println!("{:?}", request);
        println!("{:?}", self.current_state);

        match request {
            Ok(request) => {
                // commands that can be executed in any state
                if self.handle_if_loose(&request).await{
                    return;
                }
            
                match self.current_state {
                    ClientState::Connected => { self.handle_following_connected(&request).await; },
                    ClientState::Ehlo => { self.handle_following_ehlo(&request).await; },
                    ClientState::StartTLS => { self.handle_following_starttls(&request).await; },
                    ClientState::Auth => { self.handle_following_auth(&request).await; },
                    ClientState::MailFrom => { self.handle_following_mail_from(&request).await; },
                    ClientState::RcptTo => { self.handle_following_rcpt_to(&request).await; },
                    ClientState::Data => { self.handle_following_data(&request).await; },
                    ClientState::Quit => { self.handle_following_quit(&request).await; },
                }
            },
            Err(err) => {
                let _ = self.connection.write([b"500 Error\r\n", err.as_bytes()].concat().as_ref()).await;
            }
        }
        
    }



    pub async fn run(mut self) {
        let _ = self.connection.write(b"220 SMTP server ready\r\n").await;
        loop {
            if !self.connection.is_open(){
                println!("Connection closed");
                break;
            }
            self.handle_new_request().await;
        }
    }

    async fn handle_following_connected(&mut self, request: &RequestType) {
        // following connected expects no additional commands except loose commands
        match request {
            _ => {
                let _ = self.connection.write(b"500 Error\r\n").await;
            }
        }
    }

    async fn handle_following_ehlo(&mut self, request: &RequestType) {
        match request {
            RequestType::STARTTLS => {
                let _ = self.connection.write(b"220 Ready to start TLS\r\n").await;
                self.current_state = ClientState::StartTLS;

                self.connection.accept_tls(IDENTITY.clone()).await.unwrap();
            },
            _ => {
                let _ = self.connection.write(b"500 Error\r\n").await;
            }
        }
    }

    async fn handle_following_starttls(&mut self, request: &RequestType) {
        match request {
            RequestType::AUTH_PLAIN(_) => {
                self.current_state = ClientState::Auth;
                let _ = self.connection.write(b"235 OK\r\n").await;
            },
            RequestType::REGISTER(_) => {
                self.current_state = ClientState::Auth;
                let _ = self.connection.write(b"235 OK\r\n").await;
            },
            _ => {
                let _ = self.connection.write(b"500 Error\r\n").await;  
            }
        }
    }

    async fn handle_following_auth(&mut self, request: &RequestType) {
        match request {
            RequestType::MAIL_FROM(_) => {
                self.current_state = ClientState::MailFrom;
                let _ = self.connection.write(b"250 OK\r\n").await;
            },
            _ => {
                let _ = self.connection.write(b"500 Error\r\n").await;
            },
            
        }
    }

    async fn handle_following_mail_from(&mut self, request: &RequestType) {
        match request {
            RequestType::RCPT_TO(rcpt_to) => {
                self.connection_data.rcpt_to.push(rcpt_to.clone());
                self.current_state = ClientState::RcptTo;
                let _ = self.connection.write(b"250 OK\r\n").await;
            },
            _ => {
                let _ = self.connection.write(b"500 Error\r\n").await;
            }
        }
    }

    async fn handle_following_rcpt_to(&mut self, request: &RequestType) {
        match request {
            RequestType::RCPT_TO(rcpt_to) => {
                self.connection_data.rcpt_to.push(rcpt_to.clone());
                self.current_state = ClientState::RcptTo;
                let _ = self.connection.write(b"250 OK\r\n").await;
            },
            RequestType::DATA => {
                self.current_state = ClientState::Data;
                let _ = self.connection.write(b"354 End data with <CR><LF>.<CR><LF>\r\n").await;    
                self.read_data_until_dot().await;
                // save data to db
            },
            _ => {
                let _ = self.connection.write(b"500 Error\r\n").await;
            }
        }
    }

    async fn handle_following_data(&mut self, request: &RequestType) {
        match request {
            RequestType::MAIL_FROM(mail_from) => {
                self.current_state = ClientState::MailFrom;
                self.connection_data = ConnectionData::default();
                self.connection_data.rcpt_to.push(mail_from.clone());
                let _ = self.connection.write(b"250 OK\r\n").await;
            },
            _ => {
                let _ = self.connection.write(b"500 Error\r\n").await;
            }
        }
    }

    async fn read_data_until_dot(&mut self) {
        let mut data = String::new();
        // TODO: enforce max data size
        loop {
            let line = self.connection.read().await;
            println!("{:?}", line);
            if let Ok(line) = line {
                if line.ends_with("\r\n.\r\n") {
                    break;
                }
                data.push_str(&line);
            }
        }
        self.connection_data.data = data;
        let _ = self.connection.write(b"250 OK\r\n").await;
    }

    async fn handle_following_quit(&mut self, request: &RequestType) {
        match request {
            _ => {
                panic!("Unreachable");
            }
        }
    }

    async fn handle_if_loose(&mut self, request: &RequestType) -> bool {
        match request {
            RequestType::EHLO(_) => {
                self.current_state = ClientState::Ehlo;
                self.connection_data = ConnectionData::default();
                let _ = self.connection.write(b"250 OK\r\n").await;
            },
            RequestType::QUIT => {
                self.current_state = ClientState::Quit;
                let _ = self.connection.write(b"221 OK\r\n").await;
                self.connection.close();
            },
            RequestType::HELP => {
                let _ = self.connection.write(b"214 OK\r\n").await;
            },
            RequestType::NOOP => {
                let _ = self.connection.write(b"250 OK\r\n").await;
            },
            RequestType::RSET => {  
                self.current_state = ClientState::Connected;
                self.connection_data = ConnectionData::default();
                let _ = self.connection.write(b"250 OK\r\n").await;
            },
            _ => {
                return false;
            }
        }
        return true;
    }




}



static IDENTITY: LazyLock<native_tls::Identity> = LazyLock::new(|| native_tls::Identity::from_pkcs8(
    include_bytes!("../../../server/certs/server.crt"),
    include_bytes!("../../../server/certs/server.key"),
).unwrap());