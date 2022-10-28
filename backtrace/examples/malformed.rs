//! This example showcases what NOT to do. Avoid spawning tasks that are NOT
//! annotated with `#[async_backtrace::framed]`. If such tasks are spawned, and
//! they include invocations of functions that ARE annotated with
//! `#[async_backtrace::framed]`, these sub-routines will appear as distinct
//! tasks. This is both misleading, and more computationally expensive.
//!
//! Uncomment the attribute on `selecting()` to make this example behave well.

#[tokio::main]
async fn main() {
    selecting().await;
}

/* #[async_backtrace::framed] */
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
