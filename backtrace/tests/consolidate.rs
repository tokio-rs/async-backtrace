/// A test that async-backtrace is well-behaved when frames are await'ed inside
/// a drop guard.
mod util;

#[test]
fn consolidate() {
    util::model(|| util::run(selecting()));
}

#[async_backtrace::framed]
async fn selecting() {
    tokio::select! {
        biased;
        _ = yielding_outer() => {}
        _ = yielding_outer() => {}
        _ = ready() => {}
    };
}

#[async_backtrace::framed]
async fn yielding_outer() {
    yielding_inner().await;
}

#[async_backtrace::framed]
async fn yielding_inner() {
    tokio::task::yield_now().await;
}

#[async_backtrace::framed]
async fn ready() {
    let dump = async_backtrace::taskdump_tree(true);

    pretty_assertions::assert_str_eq!(
        util::strip(dump),
        "\
╼ consolidate::selecting::{{closure}} at backtrace/tests/consolidate.rs:LINE:COL
  ├╼ consolidate::ready::{{closure}} at backtrace/tests/consolidate.rs:LINE:COL
  └╼ 2x consolidate::yielding_outer::{{closure}} at backtrace/tests/consolidate.rs:LINE:COL
     └╼ consolidate::yielding_inner::{{closure}} at backtrace/tests/consolidate.rs:LINE:COL"
    );
}
