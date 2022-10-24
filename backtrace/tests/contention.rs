/// A test that taskdump is well-behaved under contention.
///
/// In this test, two threads are spawned:
/// 1. Thread 1 executes a `framed` future, which requests a blocking taskdump
/// three times in different ways (immediately, in a sub-frame, and upon drop).
/// 2. Thread 2 requests a blocking taskdump.
mod util;
use taskdump::framed;

#[test]
fn contention() {
    util::model(|| {
        let handle_a = util::thread::spawn(|| util::run(outer()));
        let handle_b = util::thread::spawn(|| taskdump::taskdump_tree(true));
        handle_a.join().unwrap();
        handle_b.join().unwrap();
    });
}

#[framed]
pub async fn outer() {}
