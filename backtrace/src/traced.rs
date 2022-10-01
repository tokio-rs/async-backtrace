use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::ptr::NonNull;

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
    }
}

impl<F> Traced<F> {
    /// Include the given `future` in taskdumps with the given `location`.
    pub fn new(future: F, location: Location) -> Self {
        Self {
            future,
            frame: Frame::new(location),
            polled: false,
        }
    }
}

impl<F> Future for Traced<F>
where
    F: Future,
{
    type Output = <F as Future>::Output;

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

        let frame = Some(NonNull::from(&self.frame));
        let future = self.as_mut().project().future;

        crate::frame::ACTIVE_FRAME.with(|active_frame| {
            // replace the previously active frame with
            let previous_frame = active_frame.replace(frame);
            // poll the inner future
            let ret = Future::poll(future, cx);
            // restore the previously active frame
            active_frame.set(previous_frame);
            ret
        })
    }
}
