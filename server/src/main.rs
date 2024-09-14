use concurrent_runtime::ConcurrentRuntime;
use smtp_server::SmtpServer;

fn main() {
    ctrlc::set_handler(move || {
        println!("received Ctrl+C!");
        println!("Shutting down server...");
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    let mut custom_runtime = ConcurrentRuntime::new(1);
    custom_runtime.start();
    
    SmtpServer::new(&mut custom_runtime).run();
}
