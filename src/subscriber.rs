use core::{
    alloc::{AllocError, Layout},
    ptr::NonNull,
};
use std::marker::PhantomCovariantLifetime;

use crate::{GuardedBuilder, guard::GuardTrait, guarded_slice::GuardedBuilderInner};

#[derive(Debug)]
struct LinkedList<'builder> {
    first: *mut GuardedBuilderInner,
    last: *mut GuardedBuilderInner,
    pd: PhantomCovariantLifetime<'builder>,
}

/// Struct used to "subscribe" multiple [GuardedSliceBuilder] for their [crate::GuardedSlice] to be allocated into a single contiguous allocation.
pub struct Subscriber<'guard, 'builder> {
    guard: &'guard mut dyn GuardTrait,
    align: usize,
    size: usize,
    linked_list: Option<LinkedList<'builder>>,
}

impl<'guard, 'builder> Subscriber<'guard, 'builder> {
    pub(crate) fn new(guard: &'guard mut impl GuardTrait) -> Self {
        Self {
            guard,
            align: 1,
            size: 0,
            linked_list: None,
        }
    }

    ///To subscribe multiple [GuardedSliceBuilder] for their [crate::GuardedSlice] to be allocated into a single contiguous allocation.
    pub fn subscribe<T>(mut self, gsb: &'builder mut GuardedBuilder<'guard, T>) -> Self
    where
        T: ?Sized,
    {
        let GuardedBuilder { inner: node, .. } = gsb;
        self.size = self.size.div_ceil(node.align) * node.align;
        self.align = self.align.max(node.align);
        self.size += node.size;

        if let Some(linked_list) = &mut self.linked_list {
            unsafe {
                (*linked_list.last).next = Some(node);
            }
            linked_list.last = node;
        } else {
            self.linked_list = Some(LinkedList {
                first: node,
                last: node,
                pd: Default::default(),
            })
        }

        self
    }

    ///When all the [GuardedSliceBuilder] were [Self::subscribe]d, and the single contiguous allocation does occur.
    pub fn allocate(self) -> Result<(), AllocError> {
        let Self {
            guard,
            align,
            size,
            linked_list,
        } = self;
        let Some(linked_list) = linked_list else {
            return Ok(());
        };
        let layout = Layout::from_size_align(size, align).unwrap();

        let ptr = unsafe { guard.allocate(layout)? };

        let ptr: NonNull<u8> = ptr.cast();
        let mut ptr = ptr.as_ptr();

        let mut it = Some(linked_list.first);

        drop(linked_list); //drop linked_list.last which is a reference

        while let Some(node) = it {
            let node = unsafe { &mut *node };
            node.ptr = Some(NonNull::new(ptr).unwrap());
            it = node.next;
            ptr = ptr
                .wrapping_add(ptr.align_offset(node.align))
                .wrapping_add(node.size);
        }

        Ok(())
    }
}
