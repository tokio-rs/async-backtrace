use async_backtrace::backtrace;

#[tokio::main]
async fn main() {
    foo().await;
}

#[backtrace]
async fn foo() {
    bar().await;
}

#[backtrace]
async fn bar() {
    baz().await;
}

#[backtrace]
async fn baz() {
    tokio::join!(fiz(), buz());
}

#[backtrace]
async fn fiz() {
    tokio::task::yield_now().await;
}

#[backtrace]
async fn buz() {
    dump().await;
}

#[backtrace]
async fn dump() {
    async_backtrace::dump()
}
