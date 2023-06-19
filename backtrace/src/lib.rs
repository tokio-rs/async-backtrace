//! Efficient, logical 'stack' traces of async functions.
//!
//! ## Usage
//! To use, annotate your async functions with `#[async_backtrace::framed]`,
//! like so:
//!
//! ```rust
//! #[tokio::main]
//! async fn main() {
//!     tokio::select! {
//!         _ = tokio::spawn(async_backtrace::frame!(pending())) => {}
//!         _ = foo() => {}
//!     };
//! }
//!
//! #[async_backtrace::framed]
//! async fn pending() {
//!     std::future::pending::<()>().await
//! }
//!
//! #[async_backtrace::framed]
//! async fn foo() {
//!     bar().await;
//! }
//!
//! #[async_backtrace::framed]
//! async fn bar() {
//!     futures::join!(fiz(), buz());
//! }
//!
//! #[async_backtrace::framed]
//! async fn fiz() {
//!     tokio::task::yield_now().await;
//! }
//!
//! #[async_backtrace::framed]
//! async fn buz() {
//!     println!("{}", baz().await);
//! }
//!
//! #[async_backtrace::framed]
//! async fn baz() -> String {
//!     async_backtrace::taskdump_tree(true)
//! }
//! ```
//!
//! This example program will print out something along the lines of:
//!
//! ```text
//! ╼ taskdump::foo::{{closure}} at backtrace/examples/taskdump.rs:20:1
//!   └╼ taskdump::bar::{{closure}} at backtrace/examples/taskdump.rs:25:1
//!      ├╼ taskdump::buz::{{closure}} at backtrace/examples/taskdump.rs:35:1
//!      │  └╼ taskdump::baz::{{closure}} at backtrace/examples/taskdump.rs:40:1
//!      └╼ taskdump::fiz::{{closure}} at backtrace/examples/taskdump.rs:30:1
//! ╼ taskdump::pending::{{closure}} at backtrace/examples/taskdump.rs:15:1
//! ```
//!
//! ## Minimizing Overhead
//! To minimize overhead, ensure that futures you spawn with your async runtime
//! are marked with `#[framed]`.
//!
//! In other words, avoid doing this:
//! ```rust
//! # #[tokio::main] async fn main() {
//! tokio::spawn(async {
//!     foo().await;
//!     bar().await;
//! }).await;
//! # }
//!
//! #[async_backtrace::framed] async fn foo() {}
//! #[async_backtrace::framed] async fn bar() {}
//! ```
//! ...and prefer doing this:
//! ```rust
//! # #[tokio::main] async fn main() {
//! tokio::spawn(async_backtrace::location!().frame(async {
//!     foo().await;
//!     bar().await;
//! })).await;
//! # }
//!
//! #[async_backtrace::framed] async fn foo() {}
//! #[async_backtrace::framed] async fn bar() {}
//! ```
//!
//! ## Estimating Overhead
//! To estimate the overhead of adopting `#[framed]` in your application, refer
//! to the benchmarks and interpretive guidance in
//! `./backtrace/benches/frame_overhead.rs`. You can run these benchmarks with
//! `cargo bench`.

pub(crate) mod frame;
pub(crate) mod framed;
pub(crate) mod linked_list;
pub(crate) mod location;
pub(crate) mod tasks;

pub(crate) use frame::Frame;
pub(crate) use framed::Framed;
pub use location::Location;
pub use tasks::{tasks, Task};

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
///     async_backtrace::frame!(async move {
///         bar().await;
///         baz().await;
///     }).await;
/// }
/// ```
pub use async_backtrace_attributes::framed;

/// Include the annotated async expression in backtraces and taskdumps.
///
/// This, for instance:
/// ```
/// # #[tokio::main] async fn main() {
/// # async fn foo() {}
/// # async fn bar() {}
/// tokio::spawn(async_backtrace::frame!(async {
///     foo().await;
///     bar().await;
/// })).await;
/// # }
/// ```
/// ...expands, roughly, to:
/// ```
/// # #[tokio::main] async fn main() {
/// # async fn foo() {}
/// # async fn bar() {}
/// tokio::spawn(async_backtrace::location!().frame(async {
///     foo().await;
///     bar().await;
/// })).await;
/// # }
/// ```
#[macro_export]
macro_rules! frame {
    ($async_expr:expr) => {
        $crate::location!().frame($async_expr)
    };
}

/// Produces a human-readable tree of task states.
///
/// If `wait_for_running_tasks` is `false`, this routine will display only the
/// top-level location of currently-running tasks and a note that they are
/// "POLLING". Otherwise, this routine will wait for currently-running tasks to
/// become idle.
///
/// # Safety
/// If `wait_for_running_tasks` is `true`, this routine may deadlock if any
/// non-async lock is held which may also be held by a Framed task.
pub fn taskdump_tree(wait_for_running_tasks: bool) -> String {
    itertools::join(
        tasks().map(|task| task.pretty_tree(wait_for_running_tasks)),
        "\n",
    )
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
/// #   macro_rules! assert_eq { ($l:expr, $r:expr) => { debug_assert_eq!($l.len(), $r.len());} }
///     assert_eq!(&async_backtrace::backtrace().unwrap().iter().map(|l| l.to_string()).collect::<Vec<_>>()[..], &[
///         "rust_out::baz::{{closure}} at src/lib.rs:19:1",
///         "rust_out::bar::{{closure}} at src/lib.rs:14:1",
///         "rust_out::foo::{{closure}} at src/lib.rs:9:1",
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

#[doc(hidden)]
/** NOT STABLE! DO NOT USE! */
pub mod ඞ {
    //  ^ kudos to Daniel Henry-Mantilla
    pub use crate::frame::Frame;
}
