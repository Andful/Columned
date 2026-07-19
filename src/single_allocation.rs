use core::{
    alloc::{AllocError, Allocator, Layout},
    mem::{MaybeUninit, transmute},
    ptr::NonNull,
    sync::atomic::{AtomicBool, Ordering},
};
use std::marker::PhantomCovariantLifetime;

///Allocator which will allocate only once at a time.
pub struct SingleAllocation<'a> {
    allocated: AtomicBool,
    data: *mut [MaybeUninit<u8>],
    _pd: PhantomCovariantLifetime<'a>,
}

impl SingleAllocation<'_> {
    ///Construct a new [OnceAllocator], which will use the `data` as its underlying memory.
    pub fn new<T>(data: &mut [MaybeUninit<T>]) -> Self {
        Self {
            allocated: AtomicBool::new(false),
            data: unsafe { transmute::<*mut [MaybeUninit<T>], *mut [MaybeUninit<u8>]>(data) },
            _pd: PhantomCovariantLifetime::new(),
        }
    }
}

unsafe impl Allocator for SingleAllocation<'_> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let start = self.data.as_mut_ptr();
        let offset = start.align_offset(layout.align());
        if offset + layout.size() >= self.data.len() {
            return Err(AllocError);
        }
        let start = start.wrapping_add(offset);
        //TODO maybe check if overflow did occur

        if self
            .allocated
            .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
        {
            return Err(AllocError);
        }

        let Some(ptr) = NonNull::new(start.cast::<u8>().cast_slice(layout.size())) else {
            unreachable!();
        };

        Ok(ptr)
    }

    unsafe fn deallocate(&self, _: std::ptr::NonNull<u8>, _: std::alloc::Layout) {
        if self
            .allocated
            .compare_exchange(true, false, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
        {
            panic!("Trying to deallocate while the allocator has no active allocations");
        }
    }
}
