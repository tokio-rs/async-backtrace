mod test;

/// A panic in the body of a frame.
use async_backtrace::backtrace;

#[test]
fn main() {
    std::panic::set_hook(Box::new(|_| {
        println!("{}", async_backtrace::tasks());
    }));

    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let _ = disco().await;
    });
}

#[backtrace]
async fn disco() {
    panic!();
}