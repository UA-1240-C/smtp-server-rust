use concurrent_runtime::ConcurrentRuntime;
use smart_stream::AsyncStream;
use std::sync::Arc;

use async_native_tls::TlsAcceptor;
use native_tls::{Identity, TlsAcceptor as NativeTlsAcceptor};

use std::net::{TcpListener, TcpStream};

fn main() {
    let mut runtime = ConcurrentRuntime::new(1);
    runtime.start();

    // Simple smtp client
    runtime.spawn(async move {
        let stream = TcpStream::connect("smtp.gmail.com:587").unwrap();
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

    // Setting up tls identity
    let listener = TcpListener::bind("127.0.0.1:2525").unwrap();
    let native_tls_acceptor = NativeTlsAcceptor::new(
        Identity::from_pkcs8(
            include_bytes!("../certs/server.crt"),
            include_bytes!("../certs/server.key"),
        ).unwrap(),
    ).unwrap();
    let acceptor = Arc::new(TlsAcceptor::from(native_tls_acceptor));

    // Simple smtp server
    loop {
        let (stream, _) = listener.accept().unwrap();
        let mut async_stream = AsyncStream::new(stream).unwrap();
        let acceptor = acceptor.clone();

        runtime.spawn(async move {
            async_stream
                .write(b"220 localhost ESMTP\r\n")
                .await
                .unwrap();

            loop {
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

                    async_stream.accept_tls(&acceptor).await.unwrap();
                } else {
                    async_stream.write(b"250 OK\r\n").await.unwrap();
                }
            }
        });
    }
}
