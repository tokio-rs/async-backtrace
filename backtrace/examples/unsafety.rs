#![feature(future_join)]

use async_backtrace::backtrace;
use std::future::join;

#[tokio::main]
async fn main() {
    tokio::join!(tokio::task::yield_now(), dump());
}

#[backtrace]
async fn dump() {
    tokio::task::spawn_blocking(async_backtrace::dump)
        .await
        .unwrap();
}
