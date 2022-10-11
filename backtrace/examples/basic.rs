use async_backtrace::framed;

#[tokio::main]
async fn main() {
    foo().await;
}

#[framed]
async fn foo() {
    bar().await;
}

#[framed]
async fn bar() {
    baz().await;
}

#[framed]
async fn baz() {
    let a = fiz();
    let b = buz();
    tokio::join!(Box::pin(fiz()), Box::pin(buz()));
}

#[framed]
async fn fiz() {
    tokio::task::yield_now().await;
}

#[framed]
async fn buz() {
    dump().await;
}

#[framed]
async fn dump() {
    println!("{}", async_backtrace::tasks());
    tokio::task::yield_now().await;
    println!("{}", async_backtrace::tasks());
    tokio::task::yield_now().await;
    println!("{}", async_backtrace::tasks());
}
