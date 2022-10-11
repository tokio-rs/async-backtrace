#![allow(unused_imports, unused_variables, dead_code)]

use std::{future::Future, sync::Mutex, task::Poll};

static STDOUT: Mutex<String> = Mutex::new(String::new());

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

#[macro_export]
macro_rules! ui_test {
    () => {
        $crate::test::UITest {
            #[cfg(bless)]
            expected_path: concat!(env!("CARGO_MANIFEST_DIR"), "/../", file!(), ".stdout"),
            #[cfg(not(bless))]
            expected_output: include_str!(concat!("../../", file!(), ".stdout")),
        }
    };
}

#[macro_export]
macro_rules! io {
    () => {
        static STDOUT: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

        pub fn stdout() -> String {
            STDOUT.lock().unwrap().clone()
        }

        pub fn println(s: impl ToString) {
            let mut stdout = STDOUT.lock().unwrap();
            *stdout += &s.to_string();
            *stdout += "\n";
        }
    };
}

pub struct UITest {
    #[cfg(bless)]
    pub(crate) expected_path: &'static str,
    #[cfg(not(bless))]
    pub(crate) expected_output: &'static str,
}

pub struct DropGuard<'a> {
    test: &'a UITest,
}

impl<'a> Drop for DropGuard<'a> {
    fn drop(&mut self) {
        let actual = &*STDOUT.lock().unwrap();
        #[cfg(bless)]
        {
            std::fs::write(self.test.expected_path, actual).unwrap();
        }
        #[cfg(not(bless))]
        {
            let expected = self.test.expected_output;
            pretty_assertions::assert_str_eq!(expected, actual);
        }
    }
}

pub fn actual() -> String {
    STDOUT.lock().unwrap().clone()
}

pub fn println(s: impl ToString) {
    let mut stdout = STDOUT.lock().unwrap();
    *stdout += &s.to_string();
    *stdout += "\n";
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
    re.replace_all(str.as_ref().trim(), ":LINE:COL").to_string()
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
