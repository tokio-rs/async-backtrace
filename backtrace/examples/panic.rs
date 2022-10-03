#![feature(future_join)]

use async_backtrace::backtrace;
use std::future::join;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let _ = tokio::spawn(panic());

    tokio::spawn(dump()).await;
}

#[backtrace]
async fn panic() {
    panic!("oops");
}

#[backtrace]
async fn dump() {
    async_backtrace::dump();
}
