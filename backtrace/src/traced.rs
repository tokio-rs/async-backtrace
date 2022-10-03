use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::marker::PhantomPinned;

use crate::frame::Frame;
use crate::location::Location;

use pin_project_lite::pin_project;

pin_project! {
    /// Includes the wrapped future `F` in taskdumps.
    pub struct Traced<F> {
        #[pin]
        future: F,
        // #[pin]
        frame: Frame,
        polled: bool,
        _pinned: PhantomPinned,
    }
}

unsafe impl<F: Send> Send for Traced<F> {}
unsafe impl<F: Sync> Sync for Traced<F> {}

impl<F> Traced<F> {
    /// Include the given `future` in taskdumps with the given `location`.
    pub fn new(future: F, location: Location) -> Self {
        Self {
            future,
            frame: Frame::new(location),
            polled: false,
            _pinned: PhantomPinned,
        }
    }
}

impl<F> Future for Traced<F>
where
    F: Future,
{
    type Output = <F as Future>::Output;

    #[track_caller]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<<Self as Future>::Output> {
        // Upon the first invocation of `poll`, initialize `frame`.
        if !self.polled {
            unsafe {
                // SAFETY: `Frame::initialize` must only be called once.
                // This is enforced by checking `!self.polled`.
                Frame::initialize(self.as_mut().project().frame);
            }
            *self.as_mut().project().polled = true;
        }

        let _frame_guard = self.as_ref().frame.with_frame();
        self.as_mut().project().future.poll(cx)
    }
}
