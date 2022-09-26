use async_backtrace::backtrace;

#[tokio::main]
async fn main() {
    foo().await;
}

#[inline(never)]
#[backtrace]
async fn foo() {
    bar().await;
}

#[inline(never)]
#[backtrace]
async fn bar() {
    baz().await;
}

#[inline(never)]
#[backtrace]
async fn baz() {
    tokio::join!(fiz(), buz());
}

#[inline(never)]
#[backtrace]
async fn fiz() {
    tokio::task::yield_now().await;
}

#[inline(never)]
#[backtrace]
async fn buz() {
    dump().await;
}

#[inline(never)]
#[backtrace]
async fn dump() {
    async_backtrace::dump();
}
