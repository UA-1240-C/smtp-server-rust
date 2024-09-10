use async_io::Async;
use futures::io::{AsyncReadExt, AsyncWriteExt};
use std::net::{TcpListener, TcpStream};
use concurrent_runtime::ConcurrentRuntime;

pub struct SmtpServer<'a> {
    runtime: &'a mut ConcurrentRuntime,
}

impl<'a> SmtpServer<'a> {
    pub fn new(runtime: &'a mut ConcurrentRuntime) -> Self {
        SmtpServer { runtime }
    }

    pub fn run(&mut self) {
        self.accep_new_connection();
    }

    fn accep_new_connection(&mut self) {
        let listener = TcpListener::bind("127.0.0.1:2525").unwrap();
        loop {
            let (socket, _) = listener.accept().unwrap();
            let str =  Async::new(socket).unwrap();
            self.runtime.spawn(async move {
                Self::handle_client(str).await;
            });
        }
    }

    pub async fn handle_client(mut stream: Async<TcpStream>) {
        let mut buffer = [0; 1024];
        loop {
            let bytes_read = stream.read(&mut buffer).await.unwrap();
            if bytes_read == 0 {
                break;
            }

            let request = String::from_utf8_lossy(&buffer[..bytes_read]);
            println!("Received: {}", request);

            let response = match request.trim() {
                "HELO" => "250 Hello\r\n".to_string(),
                "MAIL FROM" => "250 OK\r\n".to_string(),
                "RCPT TO" => "250 OK\r\n".to_string(),
                "DATA" => "354 End data with <CR><LF>.<CR><LF>\r\n".to_string(),
                "QUIT" => {
                    println!("Client disconnected");
                    break;
                }
                _ => "500 Command not recognized\r\n".to_string(),
            };

            stream.write_all(response.as_bytes()).await.unwrap();
        }
    }
}
