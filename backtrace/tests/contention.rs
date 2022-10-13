// futures::executor::block_on

// idea: create a tree one-level deep, block on poll, and then request a task
// dump in another thread. idea: use executor_blockon inside the drop of an
// async task

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
