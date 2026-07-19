use core::{
    alloc::{AllocError, Allocator, Layout},
    mem::MaybeUninit,
    ptr::NonNull,
};

use std::alloc::Global;

use crate::{
    Guard, GuardedSliceBuilder,
    chain::{Chain, ChainNode, MAX_CHAIN_LENGTH},
};

trait Sealed {}

/// Struct used to "subscribe" multiple [GuardedSliceBuilder] for their [crate::GuardedSlice] to be allocated into a single contiguous allocation.
#[allow(private_bounds)]
pub trait Subscriber<'a>: Sealed {
    ///To subscribe multiple [GuardedSliceBuilder] for their [crate::GuardedSlice] to be allocated into a single contiguous allocation.
    fn subscribe<T>(self, gsb: &mut GuardedSliceBuilder<T>) -> impl Subscriber<'a>;

    ///When all the [GuardedSliceBuilder] were [Self::subscribe]d, and the single contiguous allocation does occur.
    fn finish(self) -> Result<(), AllocError>;
}

pub(crate) struct SubscriberImpl<'a, C, A = Global>
where
    C: ChainNode,
    A: Allocator,
{
    guard: &'a mut Guard<A>,
    chain: C,
}

impl<'a, A> SubscriberImpl<'a, (), A>
where
    A: Allocator,
{
    pub(crate) fn new(guard: &'a mut Guard<A>) -> Self {
        Self { guard, chain: () }
    }
}

impl<'a, C, A> Sealed for SubscriberImpl<'a, C, A>
where
    C: ChainNode,
    A: Allocator,
{
}

impl<'a, C, A> Subscriber<'a> for SubscriberImpl<'a, C, A>
where
    C: ChainNode,
    A: Allocator,
{
    ///To subscribe multiple [GuardedSliceBuilder] for their [crate::GuardedSlice] to be allocated into a single contiguous allocation.
    fn subscribe<T>(self, gsb: &mut GuardedSliceBuilder<T>) -> impl Subscriber<'a>
    where
        A: Allocator,
    {
        SubscriberImpl {
            guard: self.guard,
            chain: Chain::new(gsb, self.chain),
        }
    }

    ///When all the [GuardedSliceBuilder] were [Self::subscribe]d, and the single contiguous allocation does occur.
    fn finish(mut self) -> Result<(), AllocError> {
        let n = C::INDEX.wrapping_add(1);
        let mut sizes = [MaybeUninit::uninit(); MAX_CHAIN_LENGTH];

        self.chain.populate_sizes(&mut sizes);

        let sizes = unsafe {
            std::mem::transmute::<&mut [MaybeUninit<usize>], &mut [usize]>(&mut sizes[..n])
        };
        let align = C::MAX_ALIGNMENT;

        let size = sizes.iter().map(Clone::clone).sum();

        let layout = Layout::from_size_align(size, align).unwrap();

        let ptr = unsafe { self.guard.allocate(layout)? };

        let mut ptr: NonNull<u8> = ptr.cast();

        let mut ptrs = [NonNull::dangling(); MAX_CHAIN_LENGTH];

        for i in 0..n {
            let index = C::SORTED_INDICES_AND_SIZES[i].index;
            ptrs[index] = ptr;
            unsafe { ptr = ptr.add(sizes[index]) };
        }

        self.chain.distribute_pointers(&ptrs);

        Ok(())
    }
}
