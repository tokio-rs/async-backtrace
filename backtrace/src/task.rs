use crate::frame::Frame;
use dashmap::DashSet as Set;
use once_cell::sync::Lazy;
use rustc_hash::FxHasher;
use std::{hash::BuildHasherDefault, ptr::NonNull};

#[derive(Hash, Eq, PartialEq)]
#[repr(transparent)]
pub struct Task(NonNull<Frame>);

impl Task {
    /*pub fn with_current<F, R>(f: F) -> R
    where
        F: FnOnce(Option<&Task>) -> R,
    {
        Frame::with_current(|current| {
            f(current.get().map(Frame::root).map(|frame| Task(NonNull::from(frame))))
        })
    }*/

    //pub(super) fn display_nonblocking(&self) -> std::fmt::Result {}
}

unsafe impl Send for Task {}
unsafe impl Sync for Task {}

/// A handle to the set of tasks.
pub struct Tasks(pub(super) ());

static TASK_SET: Lazy<Set<Task, BuildHasherDefault<FxHasher>>> = Lazy::new(Set::default);

impl Tasks {
    /// Grab a handle to the set of tasks.
    pub(super) const fn new() -> Self {
        Self(())
    }
}

impl core::fmt::Display for Tasks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        TASK_SET.iter().try_for_each(|task| task.fmt(f))
    }
}

impl core::fmt::Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::sync::{Mutex, TryLockError};
        unsafe {
            // SAFETY: It's verboten to construct a `Task` with a dangling pointer to a
            // frame.
            let frame = self.0.as_ref();
            let current_task = Frame::current().map(|frame| NonNull::from(frame.as_ref().root()));
            let maybe_lock = frame
                .mutex()
                .filter(|_| Some(self.0) != current_task)
                .map(Mutex::try_lock);
            let recurse = match maybe_lock {
                None | Some(Ok(..)) => true,
                Some(Err(TryLockError::WouldBlock)) => false,
                Some(Err(TryLockError::Poisoned(..))) => panic!("poisoned"),
            };

            // SAFETY: Calling `frame::fmt` requires that accesses to `frame.children` are
            // datarace free. This is ensured by locking the root frame of the
            // task.
            frame.fmt(f, recurse)
        }
    }
}

/// Register a given root frame as a task.
///
/// **SAFETY:** You vow to remove the given frame prior to it being dropped.
pub(crate) unsafe fn register(root_frame: &Frame) {
    let unique = TASK_SET.insert(Task(NonNull::from(root_frame)));
    debug_assert_eq!(unique, true);
}

/// De-register a given root frame as a task.
pub(crate) fn deregister(root_frame: &Frame) {
    TASK_SET.remove(&Task(NonNull::from(root_frame)));
}
