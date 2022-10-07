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
    let a = fiz();
    let b = buz();
    tokio::join!(Box::pin(fiz()), Box::pin(buz()));
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
    println!("{}", async_backtrace::tasks());
    tokio::task::yield_now().await;
    println!("{}", async_backtrace::tasks());
    tokio::task::yield_now().await;
    println!("{}", async_backtrace::tasks());
}
