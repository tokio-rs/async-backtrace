//! Run this example to see how `join!` turns taskdumps into trees.

#[tokio::main]
async fn main() {
    joining().await;
}

#[async_backtrace::framed]
async fn joining() {
    let (_, _) = tokio::join!(yielding(), ready());
}

#[async_backtrace::framed]
async fn yielding() {
    tokio::task::yield_now().await;
}

#[async_backtrace::framed]
async fn ready() {
    println!("{}", async_backtrace::taskdump_tree(true));
}
