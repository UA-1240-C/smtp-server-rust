use concurrent_runtime::ConcurrentRuntime;

use logger_proc_macro::*;
use logger::{set_logger_level, set_logger_target, LogLevel, ConsoleLogTarget};
use logger::info;

fn main() {
    set_logger_level(LogLevel::Trace);
    set_logger_target(Box::new(ConsoleLogTarget));


    let mut custom_runtime = ConcurrentRuntime::new(1);
    custom_runtime.start();
    

    // call the fib function
    let result = fib(2);
    info!("Fib result: {}", result);

    // call the add function
    custom_runtime.spawn( async {
        let result = add(2, 3).await;
        info!("Add result: {}", result);
    });

}

#[log(trace)]
pub fn fib(n: u64) -> u64 {
    if n == 0 {
        return 0;
    } else if n == 1 {
        return 1;
    } else {
        return fib(n - 1) + fib(n - 2);
    }
}

#[log(debug)]
async fn add(a: i32, b: i32) -> i32 {
    if a == 0 {
        return b;
    } 
    if b == 0 {
        return a;
    }
    return a + b;
}
