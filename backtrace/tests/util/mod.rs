#![allow(unused_imports, unused_variables, dead_code)]

use std::{future::Future, sync::Mutex, task::Poll};

pub(crate) fn model<F>(f: F)
where
    F: Fn() + Sync + Send + 'static,
{
    #[cfg(not(loom))]
    f();
    #[cfg(loom)]
    loom::model(f);
}

pub(crate) mod thread {
    #[cfg(not(loom))]
    pub(crate) use std::thread::{spawn, yield_now};

    #[cfg(loom)]
    pub(crate) use loom::thread::{spawn, yield_now};
}

pub fn run<F: Future>(f: F) -> <F as Future>::Output {
    use std::task::Context;
    let mut f = Box::pin(f);
    let waker = futures::task::noop_waker();
    let mut cx = Context::from_waker(&waker);
    loop {
        match f.as_mut().poll(&mut cx) {
            Poll::Ready(v) => return v,
            Poll::Pending => thread::yield_now(),
        }
    }
}

pub fn strip(str: impl AsRef<str>) -> String {
    let re = regex::Regex::new(r":\d+:\d+").unwrap();
    re.replace_all(str.as_ref(), ":LINE:COL").to_string()
}

pub fn defer<F: FnOnce() -> R, R>(f: F) -> impl Drop {
    Defer(Some(f))
}

struct Defer<F: FnOnce() -> R, R>(Option<F>);

impl<F: FnOnce() -> R, R> Drop for Defer<F, R> {
    fn drop(&mut self) {
        self.0.take().unwrap()();
    }
}
