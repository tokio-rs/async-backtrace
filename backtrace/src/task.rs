use crate::frame::Frame;
use dashmap::DashSet;
use once_cell::sync::Lazy;
use rustc_hash::FxHasher;
use std::{
    hash::{BuildHasherDefault, Hash, Hasher},
    ptr::NonNull,
};

static TASKS: Lazy<DashSet<Task, BuildHasherDefault<FxHasher>>> = Lazy::new(DashSet::default);

pub fn dump() {
    TASKS
        .iter()
        .for_each(|frame| println!("{}", unsafe { frame.root_frame.as_ref() }));
}

pub(crate) fn register(root_frame: NonNull<Frame>) {
    TASKS.insert(Task { root_frame });
}

pub(crate) fn deregister(root_frame: NonNull<Frame>) {
    TASKS.remove(&Task { root_frame });
}

struct Task {
    root_frame: NonNull<Frame>,
}

impl Hash for Task {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.root_frame.hash(state)
    }
}

impl Eq for Task {}

impl PartialEq for Task {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self.root_frame.as_ptr(), other.root_frame.as_ptr())
    }
}

unsafe impl Send for Task {}
unsafe impl Sync for Task {}
