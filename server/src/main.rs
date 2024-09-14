use concurrent_runtime::ConcurrentRuntime;
use native_tls::Identity;
use smart_stream::AsyncStream;

use std::net::{TcpStream, TcpListener};

fn main() {
    let mut runtime = ConcurrentRuntime::new(2);
    runtime.start();
    
    runtime.spawn(async move {
        let listener = TcpListener::bind("127.0.0.1:2525").unwrap();
        loop {
            let (stream, _) = listener.accept().unwrap();
            let mut async_stream = AsyncStream::new(stream).unwrap();
            async_stream.write(b"220 localhost ESMTP\r\n").await.unwrap();
            
            loop {
                if !async_stream.is_open() {
                    break;
                }

                let data = async_stream.read().await.unwrap();
                if data.starts_with("QUIT") {
                    async_stream.write(b"221 Bye\r\n").await.unwrap();
                    break;
                }
                else if data.starts_with("EHLO") {
                    async_stream.write(b"HELO\r\n").await.unwrap();
                }
                else if data.starts_with("STARTTLS") {
                    async_stream.write(b"220 Ready to start TLS\r\n").await.unwrap();
                    let identity = Identity::from_pkcs8(
                        include_bytes!("../certs/server.crt"),
                        include_bytes!("../certs/server.key"),
                    ).unwrap();

                    async_stream.accept_tls(identity).await.unwrap();
                } else {
                    async_stream.write(b"250 OK\r\n").await.unwrap();
                }
            }
        }
    });

    runtime.spawn(async move {
        let stream = TcpStream::connect("smtp.gmail.com:587").unwrap();
        let mut async_stream = AsyncStream::new(stream).unwrap();

        print!("{}", async_stream.read().await.unwrap());

        async_stream.write(b"EHLO smtp.gmail.com\r\n").await.unwrap();
        print!("{}", async_stream.read().await.unwrap());

        async_stream.write(b"STARTTLS\r\n").await.unwrap();
        print!("{}", async_stream.read().await.unwrap());

        async_stream.connect_tls().await.unwrap();
        async_stream.write(b"EHLO smtp.gmail.com\r\n").await.unwrap();
        print!("{}", async_stream.read().await.unwrap());
    });
}
