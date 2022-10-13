//! Efficient, logical 'stack' traces of async functions.
//!
//! To use, annotate your async functions with `#[async_backtrace::framed]`.
//!
//! # Example
//! ```rust
//! #[tokio::main]
//! async fn main() {
//!     foo().await;
//! }
//!
//! #[async_backtrace::framed]
//! async fn foo() {
//!     bar().await;
//! }
//!
//! #[async_backtrace::framed]
//! async fn bar() {
//!     tokio::join!(Box::pin(fiz()), Box::pin(buz()));
//! }
//!
//! #[async_backtrace::framed]
//! async fn fiz() {
//!     tokio::task::yield_now().await;
//! }
//!
//! #[async_backtrace::framed]
//! async fn buz() {
//!     println!("{}", tokio::spawn(baz()).await.unwrap());
//! }
//!
//! #[async_backtrace::framed]
//! async fn baz() -> String {
//!     async_backtrace::taskdump(true)
//! }
//! ```
//! The above program, when run, prints something like:
//! ```text
//! ╼ taskdump::foo::{{closure}} at backtrace/examples/taskdump.rs:6:1
//!   └╼ taskdump::bar::{{closure}} at backtrace/examples/taskdump.rs:11:1
//!   └╼ taskdump::buz::{{closure}} at backtrace/examples/taskdump.rs:21:1
//! ╼ taskdump::baz::{{closure}} at backtrace/examples/taskdump.rs:26:1
//! ```

pub(crate) mod frame;
pub(crate) mod framed;
pub(crate) mod linked_list;
pub(crate) mod location;
pub(crate) mod tasks;

pub(crate) use frame::Frame;
pub(crate) use framed::Framed;
pub use location::Location;
pub(crate) use tasks::tasks;

/// Include the annotated async function in backtraces and taskdumps.
///
/// This, for instance:
/// ```
/// # async fn bar() {}
/// # async fn baz() {}
/// #[async_backtrace::framed]
/// async fn foo() {
///     bar().await;
///     baz().await;
/// }
/// ```
/// ...expands, roughly, to:
/// ```
/// # async fn bar() {}
/// # async fn baz() {}
/// async fn foo() {
///     async_backtrace::location!().frame(async move {
///         bar().await;
///         baz().await;
///     }).await
/// }
pub use async_backtrace_macros::framed;

/// Produces a human-readable tree of task states.
///
/// If `wait_for_running_tasks` is `true`, this routine will display only the
/// top-level location of currently-running tasks and a note that they are
/// "POLLING". Otherwise, this routine will wait for currently-running tasks to
/// become idle.
pub fn taskdump(wait_for_running_tasks: bool) -> String {
    tasks()
        .map(|task| task.dump_tree(wait_for_running_tasks))
        .collect()
}

/// Produces a backtrace starting at the currently-active frame (if any).
///
/// ## Example
/// ```
/// use async_backtrace::{framed, backtrace, Location};
///
/// #[tokio::main]
/// async fn main() {
///     foo().await;
/// }
///
/// #[async_backtrace::framed]
/// async fn foo() {
///     bar().await;
/// }
///
/// #[async_backtrace::framed]
/// async fn bar() {
///     baz().await;
/// }
///
/// #[async_backtrace::framed]
/// async fn baz() {
///     assert_eq!(&async_backtrace::backtrace().unwrap()[..], &[
///         Location { fn_name: "rust_out::baz::{{closure}}", file_name: "src/lib.rs", line_no: 20, col_no: 1 },
///         Location { fn_name: "rust_out::bar::{{closure}}", file_name: "src/lib.rs", line_no: 15, col_no: 1 },
///         Location { fn_name: "rust_out::foo::{{closure}}", file_name: "src/lib.rs", line_no: 10, col_no: 1 },
///     ]);
/// }
/// ```
pub fn backtrace() -> Option<Box<[Location]>> {
    Frame::with_active(|maybe_frame| maybe_frame.map(Frame::backtrace_locations))
}

pub(crate) mod sync {
    #[cfg(loom)]
    pub(crate) use loom::sync::Mutex;

    #[cfg(not(loom))]
    pub(crate) use std::sync::Mutex;

    pub(crate) use std::sync::TryLockError;
}

pub(crate) mod cell {
    #[cfg(loom)]
    pub(crate) use loom::cell::{Cell, UnsafeCell};

    #[cfg(not(loom))]
    pub(crate) use std::cell::Cell;

    #[cfg(not(loom))]
    #[derive(Debug)]
    #[repr(transparent)]
    pub(crate) struct UnsafeCell<T>(std::cell::UnsafeCell<T>);

    #[cfg(not(loom))]
    impl<T> UnsafeCell<T> {
        pub(crate) fn new(data: T) -> UnsafeCell<T> {
            UnsafeCell(std::cell::UnsafeCell::new(data))
        }

        pub(crate) fn with<R>(&self, f: impl FnOnce(*const T) -> R) -> R {
            f(self.0.get())
        }

        pub(crate) fn with_mut<R>(&self, f: impl FnOnce(*mut T) -> R) -> R {
            f(self.0.get())
        }
    }
}

pub(crate) fn defer<F: FnOnce() -> R, R>(f: F) -> impl Drop {
    struct Defer<F: FnOnce() -> R, R>(Option<F>);

    impl<F: FnOnce() -> R, R> Drop for Defer<F, R> {
        fn drop(&mut self) {
            self.0.take().unwrap()();
        }
    }

    Defer(Some(f))
}
