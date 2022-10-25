/// A test that a non-blocking taskdump will not deadlock, even if requested
/// from insided a framed future that spawns a scoped thread that requests the
/// task dump.
mod util;
use async_backtrace::framed;

#[framed]
fn deadlockless() {
    util::model(|| util::run(outer()))
}

#[framed]
async fn outer() {
    let dump = std::thread::spawn(|| async_backtrace::taskdump_tree(true))
        .join()
        .unwrap();
    pretty_assertions::assert_str_eq!(
        util::strip(dump),
        "\
╼ deadlockless::outer at backtrace/tests/deadlockless.rs:LINE:COL
  └┈ [POLLING]"
    );
    inner().await;
}

#[framed]
async fn inner() {
    let dump = util::thread::spawn(|| async_backtrace::taskdump_tree(true))
        .join()
        .unwrap();
    pretty_assertions::assert_str_eq!(
        util::strip(dump),
        "\
╼ deadlockless::outer at backtrace/tests/deadlockless.rs:LINE:COL
  └┈ [POLLING]"
    );
}
