pub(crate) mod frame;
pub(crate) mod linked_list;
pub(crate) mod location;
pub(crate) mod task;
pub(crate) mod framed;

pub use frame::Frame;
pub use framed::Framed;
pub use location::Location;
pub use task::Tasks;

pub use async_backtrace_macros::framed;

pub const fn tasks() -> task::Tasks {
    task::Tasks::new()
}

pub(crate) mod sync {
    #[cfg(loom)]
    pub(crate) use loom::sync::Mutex;

    #[cfg(not(loom))]
    pub(crate) use std::sync::Mutex;
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
