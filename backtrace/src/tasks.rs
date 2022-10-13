use crate::Frame;
use dashmap::DashSet as Set;
use once_cell::sync::Lazy;
use rustc_hash::FxHasher;
use std::{hash::BuildHasherDefault, ops::Deref, ptr::NonNull};

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
pub(crate) fn tasks() -> impl Iterator<Item = impl Deref<Target = Task>> {
    TASK_SET.iter()
}

impl Task {
    pub(crate) fn dump_tree(&self, wait_for_running_tasks: bool) -> String {
        use crate::sync::TryLockError;

        // safety: we promsie to not inspect the subframes without first locking
        let frame = unsafe { self.0.as_ref() };

        let current_task: Option<NonNull<Frame>> =
            Frame::with_active(|maybe_frame| maybe_frame.map(|frame| frame.root().into()));

        let maybe_lock = frame
            .mutex()
            // don't grab a lock if we're *in* the active task (it's already locked, then)
            .filter(|_| Some(self.0) != current_task)
            .map(|mutex| {
                if wait_for_running_tasks {
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
