//Have to figure out how to handle no_std
use core::{
    alloc::{AllocError, Allocator, Layout},
    ptr::{NonNull, null},
};

//Todo possibly handle no_std
use std::alloc::Global;

use crate::Subscriber;
use crate::subscriber::SubscriberImpl;

///`Guard` manages and owns a contiguous allocation of memory. A `Guard` should be used only once for [allocate], [allocate_in], otherwise the allocation will fail.
pub struct Guard<A: Allocator = Global> {
    allocator: A,
    allocation: Option<(NonNull<[u8]>, Layout)>,
}

impl Default for Guard {
    fn default() -> Self {
        Self::new()
    }
}

impl Guard {
    /// Construct a new [Guard].
    pub const fn new() -> Self {
        Self::new_in(Global)
    }

    ///Gets the pointer of the allocation. If the allocation did not occur, [null()] will be returned.
    /// This pointer is not meant to be read from or written to (unless you know what you ar doing).
    pub fn as_ptr(&self) -> *const u8 {
        if let Some((ptr, _)) = self.allocation {
            ptr.as_ptr() as *const u8
        } else {
            null()
        }
    }

    ///Returns the two raw pointers spanning the allocation. If the allocation did not occure, [null()]`..`[null()] will be returned.
    pub fn as_ptr_range(&self) -> std::ops::Range<*const u8> {
        if let Some((_, layout)) = self.allocation {
            let ptr = self.as_ptr();
            ptr..(ptr.wrapping_add(layout.size()))
        } else {
            null()..null()
        }
    }
}

impl<A: Allocator> Guard<A> {
    /// Construct a new [Guard].
    pub const fn new_in(allocator: A) -> Self {
        Self {
            allocator,
            allocation: None,
        }
    }

    pub(crate) unsafe fn allocate(&mut self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let ptr = self.allocator.allocate(layout)?;
        self.allocation = Some((ptr, layout));
        Ok(ptr)
    }

    ///Construct a [Subscriber], for which multiple [crate::GuardedSliceBuilder] will [Subscriber::subscribe] to allocate a single contiguous allocation, wih the method [Subscriber::finish].
    pub fn subscriber(&mut self) -> impl Subscriber<'_> {
        if self.allocation.is_some() {
            panic!("This Guard has already been used for an allocation");
        }
        SubscriberImpl::new(self)
    }
}

impl<A: core::alloc::Allocator> Drop for Guard<A> {
    fn drop(&mut self) {
        if let Some((ptr, layout)) = self.allocation {
            unsafe { self.allocator.deallocate(ptr.as_non_null_ptr(), layout) }
        }
    }
}
