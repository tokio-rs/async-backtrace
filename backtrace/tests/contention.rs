/// A test that async-backtrace is well-behaved under contention.
///
/// In this test, two threads are spawned:
/// 1. Thread 1 executes a `framed` future, which requests a blocking taskdump
/// three times in different ways (immediately, in a sub-frame, and upon drop).
/// 2. Thread 2 requests a blocking taskdump.
mod util;
use async_backtrace::framed;

#[test]
fn contention() {
    util::model(|| {
        let handle_a = util::thread::spawn(|| util::run(outer()));
        let handle_b = util::thread::spawn(|| async_backtrace::taskdump(true));
        handle_a.join().unwrap();
        handle_b.join().unwrap();
    });
}

#[framed]
pub async fn outer() {
    let _defer = util::defer(|| async_backtrace::taskdump(true));
    async_backtrace::taskdump(true);
    inner().await;
}

#[framed]
pub async fn inner() {
    async_backtrace::taskdump(true);
}
