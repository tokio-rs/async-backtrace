use crate::Frame;
use dashmap::DashSet as Set;
use once_cell::sync::Lazy;
use rustc_hash::FxHasher;
use std::{hash::BuildHasherDefault, ops::Deref, ptr::NonNull};

/// A top-level [framed](crate::framed) future.
#[derive(Hash, Eq, PartialEq)]
#[repr(transparent)]
pub struct Task(NonNull<Frame>);

unsafe impl Send for Task {}
unsafe impl Sync for Task {}

static TASK_SET: Lazy<Set<Task, BuildHasherDefault<FxHasher>>> = Lazy::new(Set::default);

/// Register a given root frame as a task.
///
/// **SAFETY:** You vow to remove the given frame prior to it being dropped.
pub(crate) unsafe fn register(root_frame: &Frame) {
    let unique = TASK_SET.insert(Task(NonNull::from(root_frame)));
    debug_assert!(unique);
}

/// De-register a given root frame as a task.
pub(crate) fn deregister(root_frame: &Frame) {
    TASK_SET.remove(&Task(NonNull::from(root_frame)));
}

/// An iterator over tasks.
///
/// **NOTE:** The creation and destruction of some or all tasks will be blocked
/// for as long as the return value of this function is live.
pub fn tasks() -> impl Iterator<Item = impl Deref<Target = Task>> {
    TASK_SET.iter()
}

impl Task {
    /// The location of this task.
    pub fn location(&self) -> crate::Location {
        // safety: we promise to not inspect the subframes without first locking
        let frame = unsafe { self.0.as_ref() };
        frame.location()
    }

    /// Pretty-prints this task as a tree.
    ///
    /// If `block_until_idle` is `false`, the output will note that this task is
    /// currently being polled, and will not descend into its sub-frames.
    /// Otherwise, if `block_until_idle` is `true` this routine will block
    /// until this task is no longer being polled, then recursively descend and
    /// pretty-print its sub-frames.
    pub fn pretty_tree(&self, block_until_idle: bool) -> String {
        use crate::sync::TryLockError;

        // safety: we promise to not inspect the subframes without first locking
        let frame = unsafe { self.0.as_ref() };

        let current_task: Option<NonNull<Frame>> =
            Frame::with_active(|maybe_frame| maybe_frame.map(|frame| frame.root().into()));

        let maybe_lock = &frame
            .mutex()
            // don't grab a lock if we're *in* the active task (it's already locked, then)
            .filter(|_| Some(self.0) != current_task)
            .map(|mutex| {
                if block_until_idle {
                    mutex.lock().map_err(TryLockError::from)
                } else {
                    mutex.try_lock()
                }
            });

        let subframes_locked = match maybe_lock {
            None | Some(Ok(..)) => true,
            Some(Err(TryLockError::WouldBlock)) => false,
            Some(Err(err @ TryLockError::Poisoned(..))) => panic!("{:?}", err),
        };

        let mut string = String::new();

        unsafe {
            frame.fmt(&mut string, subframes_locked).unwrap();
        }

        string
    }
}
