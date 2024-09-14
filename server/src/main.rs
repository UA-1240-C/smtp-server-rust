use concurrent_runtime::ConcurrentRuntime;
use smart_stream::AsyncStream;
use std::sync::Arc;

use async_native_tls::TlsAcceptor;
use native_tls::{Identity, TlsAcceptor as NativeTlsAcceptor};

use std::net::TcpListener;

fn main() {
    let mut runtime = ConcurrentRuntime::new(1);
    runtime.start();
    

    let listener = TcpListener::bind("127.0.0.1:2525").unwrap();
    let native_tls_acceptor: NativeTlsAcceptor = NativeTlsAcceptor::new(
        Identity::from_pkcs8(
            include_bytes!("../certs/server.crt"),
            include_bytes!("../certs/server.key"),
        ).unwrap(),
    ).unwrap();

    let acceptor = Arc::new(TlsAcceptor::from(native_tls_acceptor));
    loop {
        let (stream, _) = listener.accept().unwrap();
        let async_stream = AsyncStream::new(stream).unwrap();
        let acceptor = acceptor.clone();

        runtime.spawn(async move {
            let mut connection = client_connection::ClientConnection::new(async_stream, &acceptor);
            match connection.run().await {
                Ok(_) => println!("Connection closed"),
                Err(e) => println!("Connection error: {:?}", e),
            }
        });

    }

}
