use async_backtrace::backtrace;

#[tokio::main]
async fn main() {
    tokio::join!(foo(), bar());
}

#[inline(never)]
#[backtrace]
async fn foo() {
    tokio::task::yield_now().await;
}

#[inline(never)]
#[backtrace]
async fn bar() {
    let _ = tokio::task::spawn_blocking(|| async_backtrace::dump()).await;
}
