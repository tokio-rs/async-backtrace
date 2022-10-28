//! Run this example to see how `select!` turns taskdumps into trees.

#[tokio::main]
async fn main() {
    selecting().await;
}

#[async_backtrace::framed]
async fn selecting() {
    tokio::select! {
        biased;
        _ = yielding() => {}
        _ = ready() => {}
    };
}

#[async_backtrace::framed]
async fn yielding() {
    tokio::task::yield_now().await;
}

#[async_backtrace::framed]
async fn ready() {
    println!("{}", async_backtrace::taskdump_tree(true));
}
