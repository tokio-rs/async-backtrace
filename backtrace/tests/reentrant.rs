mod util;
use async_backtrace::framed;

#[test]
fn reentrant() {
    util::run(outer())
}

#[framed]
async fn outer() {
    let dump = async_backtrace::tasks().to_string();
    pretty_assertions::assert_str_eq!(
        util::strip(dump),
        "\
╼ reentrant::outer::{{closure}} at backtrace/tests/reentrant.rs:LINE:COL"
    );
    inner().await;
}

#[framed]
async fn inner() {
    let dump = async_backtrace::tasks().to_string();
    pretty_assertions::assert_str_eq!(
        util::strip(dump),
        "\
╼ reentrant::outer::{{closure}} at backtrace/tests/reentrant.rs:LINE:COL
  └╼ reentrant::inner::{{closure}} at backtrace/tests/reentrant.rs:LINE:COL"
    );
}
