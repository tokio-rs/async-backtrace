use std::{marker::PhantomPinned, pin::Pin, ptr::NonNull};

use crate::{
    cell::{Cell, UnsafeCell},
    linked_list,
    sync::Mutex,
    Location,
};

pin_project_lite::pin_project! {
    /// A [`Location`] in an intrusive, doubly-linked tree of [`Location`]s.
    pub struct Frame {
        // The location associated with this frame.
        location: Location,

        // Have the below fields been initialized yet?
        initialized: bool,

        // The kind of this frame — either a root or a node.
        kind: Kind,

        // The children of this frame.
        children: UnsafeCell<Children>,

        // The siblings of this frame.
        #[pin]
        siblings: Siblings,

        // Since `Frame` is part of an intrusive linked list, it must remain pinned.
        _pinned: PhantomPinned,
    }

    impl PinnedDrop for Frame {
        fn drop(this: Pin<&mut Self>) {
            // If this frame has not yet been initialized, there's no need to do anything special upon drop.
            if !this.initialized {
                return;
            }

            let this = this.into_ref().get_ref();

            if let Some(parent) = this.parent() {
                // remove this frame as a child of its parent
                unsafe {
                    parent.children.with_mut(|children| (&mut *children).remove(this.into()));
                }
            } else {
                // this is a task; deregister it
                crate::task::deregister(this);
            }
        }
    }
}

// It is safe to transfer a `Frame` across thread boundaries, as it does not
// contain any pointers to thread-local storage, nor does it enable interior
// mutation on shared pointers without locking.
unsafe impl Send for Frame {}

mod active_frame {
    use super::Frame;
    use crate::cell::Cell;
    use core::ptr::NonNull;

    thread_local! {
        /// The [`Frame`] of the currently-executing [traced future](crate::Traced) (if any).
        #[cfg(not(loom))]
        static ACTIVE_FRAME: crate::cell::Cell<Option<NonNull<Frame>>> = const { Cell::new(None) };

        /// The [`Frame`] of the currently-executing [traced future](crate::Traced) (if any).
        #[cfg(loom)]
        static ACTIVE_FRAME: crate::cell::Cell<Option<NonNull<Frame>>> = Cell::new(None);
    }

    /// By calling this function, you pinky-swear to ensure that the value of
    /// `ACTIVE_FRAME` is always a valid (dereferenceable) `NonNull<Frame>`.
    pub(crate) unsafe fn with<F, R>(f: F) -> R
    where
        F: FnOnce(&Cell<Option<NonNull<Frame>>>) -> R,
    {
        ACTIVE_FRAME.with(f)
    }
}

/// The kind of a [`Frame`].
enum Kind {
    /// The frame is the root node in its tree.
    Root {
        /// This mutex must be locked when modifying the
        /// [children][Frame::children] or [siblings][Frame::siblings] of this
        /// frame.
        mutex: Mutex<()>,
    },
    /// The frame is *not* the root node of its tree.
    Node {
        /// The parent of this frame.
        parent: NonNull<Frame>,
    },
}

/// The siblings of a frame.
type Siblings = linked_list::Pointers<Frame>;

/// The children of a frame.
type Children = linked_list::LinkedList<Frame, <Frame as linked_list::Link>::Target>;

impl Frame {
    /// Construct a new, uninitialized `Frame`.
    pub fn new(location: Location) -> Self {
        Self {
            location,
            initialized: false,
            kind: Kind::Node {
                parent: NonNull::dangling(),
            },
            children: UnsafeCell::new(linked_list::LinkedList::new()),
            siblings: linked_list::Pointers::new(),
            _pinned: PhantomPinned,
        }
    }

    /// Runs a given function on this frame.
    ///
    /// If an invocation of `Frame::in_scope` is nested within `f`, those frames
    /// will be initialized with this frame as their parent.
    pub fn in_scope<F, R>(self: Pin<&mut Self>, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        // This non-generic preparation routine has been factored out of `in_scope`'s
        // body, so as to reduce the monomorphization burden on the compiler.
        //
        // The soundness of other routines in this module depend on this function *not*
        // being leaked from `in_scope`. In general, the drop-guard pattern cannot
        // safely and soundly be used for frame management. If we attempt to provide
        // such an API, we must ensure that unsoudness does not occur if child frames
        // are dropped before their parents, or if a drop-guard is held across an
        // `await` point.
        unsafe fn activate<'a>(
            mut frame: Pin<&'a mut Frame>,
            current_cell: &'a Cell<Option<NonNull<Frame>>>,
        ) -> impl Drop + 'a {
            // If needed, initialize this frame.
            if !frame.initialized {
                *frame.as_mut().project().initialized = true;
                let maybe_parent = if let Some(parent) = current_cell.get() {
                    Some(parent.as_ref())
                } else {
                    None
                };
                frame.as_mut().initialize_unchecked(maybe_parent)
            }

            let frame = frame.into_ref().get_ref();

            // If this is the root frame, lock its children. This lock is inherited by
            // `f()`.
            let maybe_mutex_guard = if let Kind::Root { mutex } = &frame.kind {
                Some(mutex.lock().unwrap())
            } else {
                None
            };

            // Replace the previously-active frame with this frame.
            let previously_active = current_cell.replace(Some(frame.into()));

            // At the end of this scope, restore the previously-active frame.
            crate::defer(move || {
                current_cell.set(previously_active);
                drop(maybe_mutex_guard);
            })
        }

        unsafe {
            // SAFETY: We uphold `with`'s invariants by restoring the previously active
            // frame after the execution of `f()`.
            active_frame::with(|current_cell| {
                // Activate this frame.
                let _restore = activate(self, current_cell);
                // Finally, execute the given function.
                f()
            })
        }
    }

    /// Produces an iterator over this frame's ancestors.
    pub fn backtrace(&self) -> impl Iterator<Item = &Frame> {
        let mut next = Some(self);
        core::iter::from_fn(move || {
            let curr = next;
            next = curr.and_then(Frame::parent);
            curr
        })
    }

    /// Produces the [`Location`] associated with this frame.
    pub fn location(&self) -> Location {
        self.location
    }

    /// Produces the parent frame of this frame.
    pub(crate) fn parent(&self) -> Option<&Frame> {
        if !self.initialized {
            return None;
        } else if let Kind::Node { parent } = self.kind {
            Some(unsafe { parent.as_ref() })
        } else {
            None
        }
    }

    /// Initializes this frame, unconditionally.
    ///
    /// ## Safety
    /// This method must only be called, at most, once.
    #[inline(never)]
    unsafe fn initialize_unchecked(mut self: Pin<&mut Self>, maybe_parent: Option<&Frame>) {
        match maybe_parent {
            // This frame has no parent...
            None => {
                // ...it is the root of its tree,
                *self.as_mut().project().kind = Kind::root();
                // ...and must be registered as a task.
                crate::task::register(self.into_ref().get_ref());
            }
            // This frame has a parent...
            Some(parent) => {
                // ...it is not the root of its tree.
                *self.as_mut().project().kind = Kind::node(parent);
                // ...and its parent should be notified that is has a new child.
                let this = NonNull::from(self.into_ref().get_ref());
                parent
                    .children
                    .with_mut(|children| (&mut *children).push_front(this));
            }
        };
    }

    pub(crate) fn current() -> Option<NonNull<Frame>> {
        unsafe {
            // SAFETY: This function does not provide the ability to mutate the cell, only
            // to retrieve its contents.
            active_frame::with(Cell::get)
        }
    }

    /// Executes the given function with a reference to the active frame on this
    /// thread (if any).
    pub fn with_current<F, R>(f: F) -> R
    where
        F: FnOnce(Option<&Frame>) -> R,
    {
        Frame::with_current_cell(|cell| f(cell.get()))
    }

    pub(crate) fn with_current_cell<F, R>(f: F) -> R
    where
        F: FnOnce(&Cell<Option<&Frame>>) -> R,
    {
        unsafe fn into_ref<'a, 'b>(
            cell: &'a Cell<Option<NonNull<Frame>>>,
        ) -> &'a Cell<Option<&'b Frame>> {
            // SAFETY: `Cell<NonNull<Frame>>` has the same layout has `Cell<&Frame>`,
            // because both `Cell` and `NonNull` are `#[repr(transparent)]`, and because
            // `*const Frame` has the same layout as `&Frame`.
            core::mem::transmute(cell)
        }

        unsafe {
            // SAFETY: We uphold `with`'s invariants, by only providing `f` with a
            // *reference* to the frame.
            active_frame::with(|cell| {
                let cell = into_ref(cell);
                f(cell)
            })
        }
    }

    /// Produces the root frame of this futures tree.
    pub(crate) fn root(&self) -> &Frame {
        let mut frame = self;
        while let Some(parent) = frame.parent() {
            frame = parent;
        }
        return frame;
    }

    /// Produces the mutex (if any) guarding this frame's children.
    pub(crate) fn mutex(&self) -> Option<&Mutex<()>> {
        if let Kind::Root { mutex } = &self.kind {
            Some(mutex)
        } else {
            None
        }
    }

    pub(crate) unsafe fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        recurse: bool,
    ) -> std::fmt::Result {
        unsafe fn fmt_helper<W: core::fmt::Write>(
            mut f: &mut W,
            frame: &Frame,
            is_last: bool,
            prefix: &str,
            recurse: bool,
        ) -> core::fmt::Result {
            let location = frame.location();
            let fn_fmt = location.fn_name;
            let file_fmt = format!(
                "{}:{}:{}",
                location.file_name, location.line_no, location.col_no
            );

            let current;
            let next;

            if is_last {
                current = format!("{prefix}└╼ {fn_fmt} at {file_fmt}");
                next = format!("{}   ", prefix);
            } else {
                current = format!("{prefix}├╼ {fn_fmt} at {file_fmt}");
                next = format!("{}│  ", prefix);
            }

            writeln!(&mut f, "{}", {
                let mut current = current.chars();
                current.next().unwrap();
                current.next().unwrap();
                current.next().unwrap();
                &current.as_str()
            })?;

            if recurse {
                frame.children.with(|children| {
                    (&*children).for_each(|frame, is_last| {
                        fmt_helper(f, frame, is_last, &next, recurse).unwrap();
                    });
                });
            } else {
                writeln!(&mut f, "{prefix}└┈ [POLLING]")?;
            }

            Ok(())
        }

        fmt_helper(f, self, true, "  ", recurse)
    }
}

impl Kind {
    /// Produces a new [`Kind::Root`].
    fn root() -> Self {
        Kind::Root {
            mutex: Mutex::new(()),
        }
    }

    /// Produces a new [`Kind::Node`].
    fn node(parent: &Frame) -> Self {
        Kind::Node {
            parent: NonNull::from(parent),
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
