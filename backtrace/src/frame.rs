use crate::{linked_list, location::Location, task};
use std::{cell::Cell, marker::PhantomPinned, ptr::NonNull, sync::Mutex};

thread_local! {
    /// The [`Frame`] of the currently-executing [traced future](crate::Traced) (if any).
    static ACTIVE_FRAME: Cell<Option<NonNull<Frame>>> = Cell::new(None);
}

/// Metadata about the invocation of a [traced future](crate::Traced).
pub(crate) struct Frame {
    /// A source location.
    pub(crate) location: Location,

    /// A pointer to the parent `Frame`, if any.
    pub(crate) parent: Option<NonNull<Frame>>,

    /// Sub-`Frame`s.
    pub(crate) children: Mutex<linked_list::LinkedList<Self, <Self as linked_list::Link>::Target>>,

    // Sibling `Frame`s.
    pub(crate) pointers: linked_list::Pointers<Self>,

    // Should never be `!Unpin`.
    pub(crate) _p: PhantomPinned,
}

impl Frame {
    /// Construct a new frame for a given location.
    pub(crate) fn uninitialized(location: Location) -> Self {
        Frame {
            location,
            parent: Some(NonNull::dangling()),
            children: Mutex::new(linked_list::LinkedList::new()),
            pointers: linked_list::Pointers::new(),
            _p: PhantomPinned,
        }
    }

    /// Initialize the `parent`, `children` and `pointers` fields of this frame.
    ///
    /// SAFETY: This method must be invoked at most once.
    pub(crate) unsafe fn initialize(&mut self) {
        // The parent of this frame (if any) is the frame held by ACTIVE_FRAME (if any).
        self.parent = ACTIVE_FRAME.with(Cell::get);

        if let Some(parent) = self.parent {
            // If this frame has a parent, notify the parent that it has a new child.
            let parent = unsafe {
                // SAFETY: When calling NonNull::as_ref, you have to ensure that all of the following is true:
                // ✓ The pointer must be properly aligned.
                // ✓ It must be “dereferenceable” in the sense defined in the module documentation.
                // ✓ The pointer must point to an initialized instance of T.
                // ✓ While this reference exists, the memory the pointer points to must not get mutated (except inside UnsafeCell).
                parent.as_ref()
            };
            // Add this frame as a child of its parent.
            parent.children.lock().unwrap().push_front(self.into());
        } else {
            // Otherwise, this frame as a root (i.e., task).
            task::register(self.into());
        }
    }

    /// Run `f` using this frame as the [`ACTIVE_FRAME`].
    #[inline(always)]
    pub(crate) fn run<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        ACTIVE_FRAME.with(|active_frame| {
            let previous_frame = active_frame.replace(Some(NonNull::from(self)));
            let ret = f();
            active_frame.set(previous_frame);
            ret
        })
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
        let field = ::std::ptr::addr_of_mut!((*me).pointers);
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
