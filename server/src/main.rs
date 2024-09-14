use async_native_tls::TlsAcceptor;
use native_tls::Identity;
use native_tls::TlsAcceptor as NativeTlsAcceptor;
use smart_stream::AsyncStream;
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:2525").unwrap();
    let identity = Identity::from_pkcs8(
        include_bytes!("../certs/server.crt"),
        include_bytes!("../certs/server.key"),
    )
    .unwrap();

    let native_acceptor = NativeTlsAcceptor::builder(identity).build().unwrap();
    let acceptor = Arc::new(TlsAcceptor::from(native_acceptor));

    // Connect to the server after 1 second
    tokio::spawn(async move {
        std::thread::sleep(std::time::Duration::from_secs(1));
        let stream = TcpStream::connect("localhost:2525").unwrap();
        let mut async_stream = AsyncStream::new(stream).unwrap();

        print!("{}", async_stream.read().await.unwrap());

        async_stream
            .write(b"EHLO smtp.gmail.com\r\n")
            .await
            .unwrap();
        print!("{}", async_stream.read().await.unwrap());

        async_stream.write(b"STARTTLS\r\n").await.unwrap();
        print!("{}", async_stream.read().await.unwrap());

        async_stream.connect_tls().await.unwrap();
        async_stream
            .write(b"EHLO smtp.gmail.com\r\n")
            .await
            .unwrap();
        print!("{}", async_stream.read().await.unwrap());
    });

    // Actual server. Blocks the main thread, listens for incoming connections and spawns a new
    // async coroutine for each connection.
    loop {
        let (stream, _) = listener.accept().unwrap();
        let mut async_stream = AsyncStream::new(stream).unwrap();
        async_stream
            .write(b"220 localhost ESMTP\r\n")
            .await
            .unwrap();
        
        let acceptor_clone = acceptor.clone();
        tokio::spawn(async move {
            loop {
                if !async_stream.is_open() {
                    break;
                }

                let data = async_stream.read().await.unwrap();

                if data.starts_with("QUIT") {
                    async_stream.write(b"221 Bye\r\n").await.unwrap();
                    break;
                } else if data.starts_with("EHLO") {
                    async_stream.write(b"HELO\r\n").await.unwrap();
                } else if data.starts_with("STARTTLS") {
                    async_stream
                        .write(b"220 Ready to start TLS\r\n")
                        .await
                        .unwrap();

                    async_stream.accept_tls(&acceptor_clone).await.unwrap();
                } else {
                    async_stream.write(b"250 OK\r\n").await.unwrap();
                }
            }
        });
    }
}
