use std::{cell::Cell, marker::PhantomPinned, ptr::NonNull, sync::Mutex};

use crate::{
    location::Location,
    linked_list,
};

type Siblings = linked_list::Pointers<Frame>;
type Children = linked_list::LinkedList<Frame, <Frame as linked_list::Link>::Target>;


pub struct Frame {
    location: Location,
    parent: Option<NonNull<Frame>>,
    children: Mutex<Children>,
    siblings: Siblings,
    _pinned: PhantomPinned,
}

impl Drop for Frame {
    fn drop(&mut self) {
        let raw = NonNull::from(self);
        unsafe {
            if let Some(parent) = raw.as_ref().parent {
                // remove this frame as a child of its parent
                parent.as_ref().children.lock().unwrap().remove(raw);
            } else {
                // this is a task; deregister it
                crate::task::deregister(raw);
            }
        }
    }
}


thread_local! {
    /// The [`Frame`] of the currently-executing [traced future](crate::Traced) (if any).
    pub(crate) static ACTIVE_FRAME: std::cell::Cell<Option<NonNull<Frame>>>  = const { Cell::new(None) };
}

impl Frame {
    /// Construct a new, uninitialized `Frame`.
    pub(crate) fn new(location: Location) -> Self {
        Self {
            location,
            parent: None,
            children: Mutex::new(linked_list::LinkedList::new()),
            siblings: linked_list::Pointers::new(),
            _pinned: PhantomPinned,
        }
    }

    /// Initialize the given `Frame`.
    /// 
    /// **SAFETY:** Must only be called once.
    pub(crate) unsafe fn initialize(&mut self) {
        if let Some(parent) = ACTIVE_FRAME.with(Cell::get) {
            self.parent = Some(parent);
            parent.as_ref().children.lock().unwrap().push_front(NonNull::from(self));
        } else {
            crate::task::register(NonNull::from(self));
        }
    }
}

unsafe impl linked_list::Link for Frame {
    type Handle = NonNull<Self>;
    type Target = Self;

    fn as_raw(handle: &NonNull<Self>) -> NonNull<Self> {
        *handle
    }

    unsafe fn from_raw(ptr: NonNull<Self>) -> NonNull<Self> {
        ptr
    }

    unsafe fn pointers(target: NonNull<Self>) -> NonNull<linked_list::Pointers<Self>> {
        let me = target.as_ptr();
        let field = ::std::ptr::addr_of_mut!((*me).siblings);
        ::core::ptr::NonNull::new_unchecked(field)
    }
}

impl core::fmt::Display for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        display(f, self, true, "─ ")
    }
}

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
    let children = frame.children.lock().unwrap();
    let len = children.len();
    children.for_each(|frame| {
        let is_last = (i + 1) == len;
        display(f, frame, is_last, &next).unwrap();
        i += 1;
    });

    Ok(())
}

