use concurrent_runtime::ConcurrentRuntime;
use smart_stream::AsyncStream;

use std::net::TcpListener;

fn main() {
    let mut runtime = ConcurrentRuntime::new(1);
    runtime.start();
    

    let listener = TcpListener::bind("127.0.0.1:2525").unwrap();
    loop {
        let (stream, _) = listener.accept().unwrap();
        let async_stream = AsyncStream::new(stream).unwrap();
        runtime.spawn(async {
            let connection = client_connection::ClientConnection::new(async_stream).await;
            connection.run().await
        });
    }

}
