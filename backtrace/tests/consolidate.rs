/// A test that async-backtrace is well-behaved when frames are await'ed inside
/// a drop guard.
mod util;
use std::{future::pending, sync::Arc};

use async_backtrace::framed;
use futures::future::join3;
use itertools::Itertools;
use tokio::sync::Barrier;

#[test]
fn consolidate() {
    util::model(|| util::run(outer()));
}

#[framed]
async fn outer() {
    let barrier = Arc::new(Barrier::new(4));
    let barrier2 = barrier.clone();
    util::thread::spawn(|| util::run(lots(barrier2)));
    barrier.wait().await;
    let dump = async_backtrace::taskdump_tree(true);
    pretty_assertions::assert_str_eq!(
        itertools::join(util::strip(dump).lines().sorted(), "\n"),
        r"  └╼ 3x consolidate::inner::{{closure}} at backtrace/tests/consolidate.rs:LINE:COL
╼ consolidate::lots::{{closure}} at backtrace/tests/consolidate.rs:LINE:COL
╼ consolidate::outer::{{closure}} at backtrace/tests/consolidate.rs:LINE:COL"
    );
}

#[framed]
async fn lots(barrier: Arc<Barrier>) {
    join3(
        inner(barrier.clone()),
        inner(barrier.clone()),
        inner(barrier.clone()),
    )
    .await;
}

#[framed]
async fn inner(barrier: Arc<Barrier>) {
    barrier.wait().await;
    pending::<()>().await;
}
