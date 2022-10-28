//! Run this example to see how functions NOT annotated with
//! `#[async_backtrace::framed]` don't appear in taskdumps.

#[tokio::main]
async fn main() {
    foo().await;
}

#[async_backtrace::framed]
async fn foo() {
    bar().await;
}

/* #[async_backtrace::framed] */
async fn bar() {
    baz().await;
}

#[async_backtrace::framed]
async fn baz() {
    println!("{}", async_backtrace::taskdump_tree(true));
}
