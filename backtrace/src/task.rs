use once_cell::sync::Lazy;
use rustc_hash::FxHasher;
use std::{collections::HashSet as Set, hash::BuildHasherDefault, ptr::NonNull, sync::Mutex};

use crate::frame::Frame;

#[derive(Hash, Eq, PartialEq)]
struct Task(NonNull<Frame>);

unsafe impl Send for Task {}
unsafe impl Sync for Task {}

static TASKS: Lazy<Mutex<Set<Task, BuildHasherDefault<FxHasher>>>> = Lazy::new(Mutex::default);

/// Print out tasks.
pub fn dump() {
    TASKS.lock().unwrap().iter().for_each(|frame| {
        let frame = unsafe { frame.0.as_ref() };
        let pp = frame.to_string();
        println!("{}", pp);
    });
}

/// Register a root frame as a task.
///
/// **SAFETY:** The `root_frame` must be dereferencable.
pub(crate) unsafe fn register(root_frame: NonNull<Frame>) {
    TASKS.lock().unwrap().insert(Task(root_frame));
}

///
/// **SAFETY:** The `root_frame` must be dereferencable.
pub(crate) unsafe fn deregister(root_frame: NonNull<Frame>) {
    TASKS.lock().unwrap().remove(&Task(root_frame));
}
