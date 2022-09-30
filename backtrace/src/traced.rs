use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use crate::frame::Frame;
use crate::task;

use pin_project_lite::pin_project;

use crate::location::Location;

/*
if it's the root, we need to add it to the global root set
    - which means locking it.
if it's not the root, we need to only lock this subtree of the global root set
    -
*/

pin_project! {
    /// Includes the wrapped future `F` in taskdumps.
    pub struct Traced<F> {
        #[pin]
        future: F,
        polled: bool,
        frame: Frame,
    }

    impl<T> PinnedDrop for Traced<T> {
        fn drop(this: Pin<&mut Self>) {
            let this = this.project();
            if let Some(parent) = this.frame.parent {
                // If this frame has a parent, it is not the root `Traced` in its futures tree.
                let parent = unsafe {
                    // SAFETY: When calling NonNull::as_ref, you have to ensure that all of the following is true:
                    // ✓ The pointer must be properly aligned.
                    // ✓ It must be “dereferenceable” in the sense defined in the module documentation.
                    // ✓ The pointer must point to an initialized instance of T.
                    // ✓ While this reference exists, the memory the pointer points to must not get mutated (except inside UnsafeCell).
                    parent.as_ref()
                };
                unsafe {
                    // SAFETY: When calling `LinkedList::remove`, the caller must ensure that:
                    // ✓ The given node-to-be-removed is not a part of any other linked list.
                    (&mut *parent.children.get()).remove(this.frame.into());
                }
            } else {
                // This frame lacks a parent; it is the root `Traced` in its futures tree.
                task::deregister(this.frame.into())
            }
        }
    }
}

impl<F> Traced<F> {
    /// Include the given `future` in taskdumps with the given `location`.
    pub fn new(future: F, location: Location) -> Self {
        Self {
            future,
            polled: false,
            frame: Frame::uninitialized(location),
        }
    }
}

impl<F> Future for Traced<F>
where
    F: Future,
{
    type Output = <F as Future>::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<<Self as Future>::Output> {
        let this = self.project();

        if !*this.polled {
            *this.polled = true;
            unsafe {
                // SAFETY: Callers of `Frame::initialize` must ensure that:
                // ✓ The method is only invoked once (ensured by `this.polled`).
                this.frame.initialize();
            }
        }

        let _guard = this.frame.tasklock.as_ref().map(|mutex| mutex.lock());

        // poll the future under the new current frame
        this.frame.run(|| Future::poll(this.future, cx))
    }
}
