use async_backtrace::backtrace;

#[tokio::main]
async fn main() {
    tokio::join!(tokio::task::yield_now(), dump());
}

#[backtrace]
async fn dump() {
    tokio::task::spawn_blocking(async_backtrace::dump)
        .await
        .unwrap();
}
