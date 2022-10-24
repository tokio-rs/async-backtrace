/// A test that taskdump is well-behaved when frames are await'ed inside
/// a drop guard.
mod util;
use taskdump::framed;

#[test]
fn poll_in_drop() {
    util::model(|| {
        let on_drop = util::defer(|| util::run(inner()));
        util::run(outer(on_drop));
    });

    #[allow(drop_bounds)]
    #[framed]
    async fn outer(defer: impl Drop) {
        let _defer = defer;
    }

    #[framed]
    async fn inner() {
        let dump = taskdump::taskdump_tree(true);
        pretty_assertions::assert_str_eq!(util::strip(dump), "\
╼ poll_in_drop::poll_in_drop::outer<poll_in_drop::util::Defer<poll_in_drop::poll_in_drop::{{closure}}::{{closure}}, ()>>::{{closure}} at backtrace/tests/poll-in-drop.rs:LINE:COL
  └╼ poll_in_drop::poll_in_drop::inner::{{closure}} at backtrace/tests/poll-in-drop.rs:LINE:COL");
    }
}
