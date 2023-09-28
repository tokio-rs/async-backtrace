use std::{iter::FusedIterator, marker::PhantomPinned, pin::Pin, ptr::NonNull};

use crate::{
    cell::{Cell, UnsafeCell},
    linked_list,
    sync::Mutex,
    Location,
};

pin_project_lite::pin_project! {
/// A function frame that may be reported in async backtraces.
///
// A Frame in an intrusive, doubly-linked tree of Frames.
#[derive(Debug)]
pub struct Frame {
    // The location associated with this frame.
    location: Location,

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
        if this.is_uninitialized() {
            return;
        }

        let this = this.into_ref().get_ref();

        if let Some(parent) = this.parent() {
            // remove this frame as a child of its parent
            unsafe {
                parent.children.with_mut(|children| (*children).remove(this.into()));
            }
        } else {
            // this is a task; deregister it
            crate::tasks::deregister(this);
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

    #[cfg(loom)]
    loom::thread_local! {
        /// The [`Frame`] of the currently-executing [traced future](crate::Traced) (if any).
        static ACTIVE_FRAME: crate::cell::Cell<Option<NonNull<Frame>>> = Cell::new(None);
    }

    #[cfg(not(loom))]
    std::thread_local! {
        /// The [`Frame`] of the currently-executing [traced future](crate::Traced) (if any).
        #[allow(clippy::declare_interior_mutable_const)]
        static ACTIVE_FRAME: crate::cell::Cell<Option<NonNull<Frame>>> = const { Cell::new(None) };
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
#[derive(Debug)]
enum Kind {
    /// The frame is not yet initialized.
    Uninitialized,

    /// The frame is the root node in its tree.
    Root {
        /// This mutex must be locked when accessing the
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
    ///
    /// If you are framing a [`Future`](core::future::Future), use the
    /// [`frame`](macro@crate::frame) or [`framed`](macro@crate::framed) macros
    /// instead.
    ///
    /// # Examples
    /// ```
    /// use async_backtrace::*;
    ///
    /// let frame = Frame::new(location!());
    /// ```
    pub fn new(location: Location) -> Self {
        Self {
            location,
            kind: Kind::Uninitialized,
            children: UnsafeCell::new(linked_list::LinkedList::new()),
            siblings: linked_list::Pointers::new(),
            _pinned: PhantomPinned,
        }
    }

    /// Runs a given function on this frame.
    ///
    /// If an invocation of `Frame::in_scope` is nested within `f`, those frames
    /// will be initialized with this frame as their parent.
    ///
    /// # Example
    ///
    /// ```
    /// # use core::{task::{Context, Poll}, pin::Pin};
    /// # use tokio::io::{AsyncRead, ReadBuf};
    /// # use pin_project_lite::*;
    /// pin_project! {
    ///     #[must_use = "streams do nothing unless you `.await` or poll them"]
    ///     pub struct Take<R> {
    ///         #[pin]
    ///         inner: R,
    ///         // Add a `Frame` field to your type.
    ///         #[pin]
    ///         frame: async_backtrace::Frame,
    ///         limit: u64,
    ///     }
    /// }
    ///
    /// impl<R: AsyncRead> AsyncRead for Take<R> {
    ///     fn poll_read(
    ///         self: Pin<&mut Self>,
    ///         cx: &mut Context<'_>,
    ///         buf: &mut ReadBuf<'_>,
    ///     ) -> Poll<Result<(), std::io::Error>> {
    ///         // And run your async routines `in_scope` of `Frame`.
    ///         let this = self.project();
    ///         this.frame.in_scope(|| {
    ///             unimplemented!("method internals go here")  
    ///         })
    ///     }
    /// }
    /// ```
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
            active: &'a Cell<Option<NonNull<Frame>>>,
        ) -> impl Drop + 'a {
            // If needed, initialize this frame.
            if frame.is_uninitialized() {
                let maybe_parent = active.get().map(|parent| parent.as_ref());
                frame.as_mut().initialize_unchecked(maybe_parent)
            }

            let frame = frame.into_ref().get_ref();

            // If this is the root frame, lock its children. This lock is inherited by
            // `f()`.
            let maybe_mutex_guard = if let Kind::Root { mutex } = &frame.kind {
                // Ignore poisoning. This is fine, since absolutely nothing between this line,
                // and the execution of `drop(maybe_mutex_guard)` can unwind-panic, *except* for
                // the execution of the user-provided function `f`. An unwind-panic of `f` will
                // not make this crate's state inconsistent, since the parent frame is always
                // restored by the below invocation of `crate::defer` upon its drop.
                Some(match mutex.lock() {
                    Ok(guard) => guard,
                    Err(err) => err.into_inner(),
                })
            } else {
                None
            };

            // Replace the previously-active frame with this frame.
            let previously_active = active.replace(Some(frame.into()));

            // At the end of this scope, restore the previously-active frame.
            crate::defer(move || {
                active.set(previously_active);
                drop(maybe_mutex_guard);
            })
        }

        unsafe {
            // SAFETY: We uphold `with`'s invariants by restoring the previously active
            // frame after the execution of `f()`.
            active_frame::with(|active| {
                // Activate this frame.
                let _restore = activate(self, active);
                // Finally, execute the given function.
                f()
            })
        }
    }

    /// Produces a boxed slice over this frame's ancestors.
    pub(crate) fn backtrace_locations(&self) -> Box<[Location]> {
        let len = self.backtrace().count();
        let mut vec = Vec::with_capacity(len);
        vec.extend(self.backtrace().map(Frame::location));
        vec.into_boxed_slice()
    }

    /// Produces the [`Location`] associated with this frame.
    pub(crate) fn location(&self) -> Location {
        self.location
    }

    /// Produces `true` if this `Frame` is uninitialized, otherwise false.
    fn is_uninitialized(&self) -> bool {
        self.kind.is_uninitialized()
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
                crate::tasks::register(self.into_ref().get_ref());
            }
            // This frame has a parent...
            Some(parent) => {
                // ...it is not the root of its tree.
                *self.as_mut().project().kind = Kind::node(parent);
                // ...and its parent should be notified that is has a new child.
                let this = NonNull::from(self.into_ref().get_ref());
                parent
                    .children
                    .with_mut(|children| (*children).push_front(this));
            }
        };
    }

    /// Executes the given function with a reference to the active frame on this
    /// thread (if any).
    pub(crate) fn with_active<F, R>(f: F) -> R
    where
        F: FnOnce(Option<&Frame>) -> R,
    {
        Frame::with_active_cell(|cell| f(cell.get()))
    }

    pub(crate) fn with_active_cell<F, R>(f: F) -> R
    where
        F: FnOnce(&Cell<Option<&Frame>>) -> R,
    {
        #[allow(clippy::needless_lifetimes)]
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

    /// Produces the mutex (if any) guarding this frame's children.
    pub(crate) fn mutex(&self) -> Option<&Mutex<()>> {
        if let Kind::Root { mutex } = &self.kind {
            Some(mutex)
        } else {
            None
        }
    }

    pub(crate) unsafe fn fmt<W: core::fmt::Write>(
        &self,
        w: &mut W,
        subframes_locked: bool,
    ) -> std::fmt::Result {
        unsafe fn fmt_helper<W: core::fmt::Write>(
            f: &mut W,
            frame: &Frame,
            is_last: bool,
            prefix: &str,
            subframes_locked: bool,
            copies: usize,
        ) -> core::fmt::Result {
            let location = frame.location();
            let current;
            let next;

            if is_last {
                if copies != 1 {
                    current = format!("{prefix}└╼ {copies}x {location}");
                } else {
                    current = format!("{prefix}└╼ {location}");
                }
                next = format!("{prefix}   ");
            } else {
                if copies != 1 {
                    current = format!("{prefix}├╼ {copies}x {location}");
                } else {
                    current = format!("{prefix}├╼ {location}");
                }
                next = format!("{prefix}│  ");
            }

            // print all but the first three codepoints of current
            write!(f, "{}", {
                let mut current = current.chars();
                current.next().unwrap();
                current.next().unwrap();
                current.next().unwrap();
                &current.as_str()
            })?;

            if subframes_locked {
                let mut subframes = frame.subframes().peekable();
                let mut copies = 1;
                while let Some(subframe) = subframes.next() {
                    if subframes
                        .peek()
                        .map(|next| next.deep_eq(subframe))
                        .unwrap_or(false)
                    {
                        copies += 1;
                    } else {
                        writeln!(f)?;
                        let is_last = subframes.peek().is_none();
                        fmt_helper(f, subframe, is_last, &next, true, copies)?;
                        copies = 1;
                    }
                }
            } else {
                writeln!(f)?;
                write!(f, "{prefix}└┈ [POLLING]")?;
            }

            Ok(())
        }

        fmt_helper(w, self, true, "  ", subframes_locked, 1)
    }

    /// Produces the parent frame of this frame.
    pub(crate) fn parent(&self) -> Option<&Frame> {
        if self.is_uninitialized() {
            None
        } else if let Kind::Node { parent } = self.kind {
            Some(unsafe { parent.as_ref() })
        } else {
            None
        }
    }

    /// Produces the root frame of this futures tree.
    pub(crate) fn root(&self) -> &Frame {
        let mut frame = self;
        while let Some(parent) = frame.parent() {
            frame = parent;
        }
        frame
    }

    /// Produces an iterator over this frame's ancestors.
    pub(crate) fn backtrace(&self) -> impl Iterator<Item = &Frame> + FusedIterator {
        /// An iterator that traverses up the tree of [`Frame`]s from a leaf.
        #[derive(Clone)]
        pub(crate) struct Backtrace<'a> {
            frame: Option<&'a Frame>,
        }

        impl<'a> Backtrace<'a> {
            pub(crate) fn from_leaf(frame: &'a Frame) -> Self {
                Self { frame: Some(frame) }
            }
        }

        impl<'a> Iterator for Backtrace<'a> {
            type Item = &'a Frame;

            fn next(&mut self) -> Option<Self::Item> {
                let curr = self.frame;
                self.frame = curr.and_then(Frame::parent);
                curr
            }
        }

        impl<'a> FusedIterator for Backtrace<'a> {}

        Backtrace::from_leaf(self)
    }

    /// Produces an iterator over this frame's children, in order from
    /// less-recently initialized to more recently initialized.
    ///
    /// # Safety
    /// The caller must ensure that the corresponding Kind::Root{mutex} is
    /// locked.  The caller must also ensure that the returned iterator is
    /// dropped before the mutex is dropped.
    pub(crate) unsafe fn subframes(&self) -> impl Iterator<Item = &Frame> + FusedIterator {
        pub(crate) struct Subframes<'a> {
            iter: linked_list::Iter<'a, Frame>,
        }

        impl<'a> Subframes<'a> {
            pub(crate) unsafe fn from_parent(frame: &'a Frame) -> Self {
                Self {
                    iter: frame.children.with(|children| (*children).iter()),
                }
            }
        }

        impl<'a> Iterator for Subframes<'a> {
            type Item = &'a Frame;

            fn next(&mut self) -> Option<Self::Item> {
                self.iter.next().map(|frame| unsafe { frame.as_ref() })
            }
        }

        impl<'a> FusedIterator for Subframes<'a> {}

        Subframes::from_parent(self)
    }

    /// # Safety
    /// The caller must ensure that the corresponding Kind::Root{mutex} is
    /// locked.
    pub(crate) unsafe fn deep_eq(&self, other: &Frame) -> bool {
        if self.location() != other.location() {
            return false;
        }

        let mut self_subframes = self.subframes();
        let mut other_subframes = other.subframes();

        loop {
            match (self_subframes.next(), other_subframes.next()) {
                (Some(self_subframe), Some(other_subframe)) => {
                    if !self_subframe.deep_eq(other_subframe) {
                        return false;
                    }
                }
                (None, None) => {
                    return true;
                }
                _ => {
                    return false;
                }
            }
        }
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

    /// True if kind is [`Kind::Uninitialized`].
    fn is_uninitialized(&self) -> bool {
        matches!(&self, Kind::Uninitialized)
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
        NonNull::new_unchecked(field)
    }
}
