use crate::frame::Frame;
use dashmap::DashSet;
use once_cell::sync::Lazy;
use rustc_hash::FxHasher;
use std::{
    hash::{BuildHasherDefault, Hash, Hasher},
    ptr::NonNull, sync::Mutex,
};

static TASKS: Lazy<DashSet<Task, BuildHasherDefault<FxHasher>>> = Lazy::new(DashSet::default);

pub fn dump() {
    TASKS
        .iter()
        .for_each(|task| println!("{}", *task));
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

impl core::fmt::Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn display<W: core::fmt::Write>(
            mut f: &mut W,
            frame: &Frame,
            is_last: bool,
            prefix: &str,
        ) -> core::fmt::Result {
            let location = &frame.location;
            let fn_fmt = location.fn_name;
            let file_fmt = format!(
                "{}:{}:{}",
                location.file_name, location.line_no, location.col_no
            );

            let current;
            let next;

            if is_last {
                current = format!("{prefix}└─\u{a0}{fn_fmt} at {file_fmt}");
                next = format!("{}\u{a0}\u{a0}\u{a0}", prefix);
            } else {
                current = format!("{prefix}├─\u{a0}{fn_fmt} at {file_fmt}");
                next = format!("{}│\u{a0}\u{a0}", prefix);
            }

            writeln!(&mut f, "{}", {
                let mut current = current.chars();
                current.next().unwrap();
                current.next().unwrap();
                current.next().unwrap();
                &current.as_str()
            })?;

            let mut i = 0;
            let _lock = frame.tasklock.as_ref().map(Mutex::lock);
            let children = unsafe { &mut *frame.children.get() };
            let len = children.len();
            children.for_each(|frame| {
                let is_last = (i + 1) == len;
                display(f, frame, is_last, &next).unwrap();
                i += 1;
            });

            Ok(())
        }

        let frame = unsafe {
            self.root_frame.as_ref()
        };
        display(f, frame, true, "─ ")
    }
}

