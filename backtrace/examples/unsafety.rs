/// Demonstrates a correctness/safety issue, in which a future is polled, 
/// then forgotten and replaced with a future in the same memory location.
use async_backtrace::backtrace;

use futures::task;

use std::{
    future::Future,
    mem::{forget, replace},
    pin::Pin,
    task::{Context, Poll},
};

pub struct TestFuture;

impl Future for TestFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

fn main() {
    #[backtrace]
    async fn root() {
        #[repr(C)]
        enum Place<A, B> {
            A(A),
            B(B),
        }

        #[backtrace]
        async fn foo() {
            TestFuture.await
        }

        #[backtrace]
        async fn bar() {
            TestFuture.await
        }

        #[backtrace]
        async fn baz() {
            TestFuture.await
        }

        let waker = task::noop_waker();
        let mut cx = Context::from_waker(&waker);

        let place = &mut Place::A(foo());

        loop {
            match place {
                Place::A(f) => {
                    let pinned = unsafe { Pin::new_unchecked(f) };
                    let _ = pinned.poll(&mut cx);
                    forget(replace(place, Place::B(bar())));
                }
                Place::B(ref mut f) => {
                    let pinned = unsafe { Pin::new_unchecked(f) };
                    let _ = pinned.poll(&mut cx);
                    break;
                }
            }
        }

        core::mem::forget(place);
    }

    let waker = task::noop_waker();
    let mut cx = Context::from_waker(&waker);
    let mut future = root();
    let pinned = unsafe { Pin::new_unchecked(&mut future) };
    let _ = pinned.poll(&mut cx);
}
