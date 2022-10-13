///
mod util;
use async_backtrace::framed;

#[framed]
fn deadlockless() {
    util::model(|| util::run(outer()))
}

#[framed]
async fn outer() {
    let dump = std::thread::spawn(|| async_backtrace::taskdump(true))
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
    let dump = util::thread::spawn(|| async_backtrace::taskdump(true))
        .join()
        .unwrap();
    pretty_assertions::assert_str_eq!(
        util::strip(dump),
        "\
╼ deadlockless::outer at backtrace/tests/deadlockless.rs:LINE:COL
  └┈ [POLLING]"
    );
}
