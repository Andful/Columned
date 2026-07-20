use core::{
    alloc::{AllocError, Layout},
    ptr::NonNull,
};

use crate::{GuardedSliceBuilder, guard::GuardTrait, guarded_slice::GuardedSliceBuilderInner};

#[derive(Debug)]
struct LinkedList<'builder> {
    first: *mut GuardedSliceBuilderInner,
    last: &'builder mut GuardedSliceBuilderInner,
}

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
    pub fn subscribe<T>(mut self, gsb: &'builder mut GuardedSliceBuilder<'guard, T>) -> Self {
        let GuardedSliceBuilder { inner: node, .. } = gsb;
        self.size = self.size.div_ceil(node.align) * node.align;
        self.align = self.align.max(node.align);
        self.size += node.n * node.size;

        if let Some(linked_list) = &mut self.linked_list {
            linked_list.last.next = Some(node);
            linked_list.last = node;
        } else {
            self.linked_list = Some(LinkedList {
                first: node,
                last: node,
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

        while let Some(node) = it {
            let node = unsafe { &mut *node };
            node.ptr = Some(NonNull::new(ptr).unwrap());
            it = node.next;
            ptr = ptr
                .wrapping_add(ptr.align_offset(node.align))
                .wrapping_add(node.size * node.n);
        }

        Ok(())
    }
}
