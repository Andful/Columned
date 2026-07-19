//Have to figure out how to handle no_std
use core::{mem::MaybeUninit, ptr::NonNull};

use crate::GuardedSliceBuilder;

pub(crate) const MAX_CHAIN_LENGTH: usize = 32;

const fn max(a: usize, b: usize) -> usize {
    if a > b { a } else { b }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct IndexAndAlign {
    pub(crate) index: usize,
    pub(crate) align: usize,
}

const fn sorted_insert<const N: usize>(
    mut index: usize,
    to_insert: IndexAndAlign,
    mut data: [IndexAndAlign; N],
) -> [IndexAndAlign; N] {
    data[index] = to_insert;
    while index > 0 {
        if data[index].align > data[index - 1].align {
            (data[index], data[index - 1]) = (data[index - 1], data[index]);
        } else {
            break;
        }
        index -= 1;
    }
    data
}

trait Sealed {}

#[allow(private_bounds)]
pub trait ChainNode: Sealed {
    const INDEX: usize;
    const MAX_ALIGNMENT: usize;
    #[allow(private_interfaces)]
    const SORTED_INDICES_AND_SIZES: [IndexAndAlign; MAX_CHAIN_LENGTH]; //Fixed because I don't know how to make it "dynamic"
    fn distribute_pointers(&mut self, sizes: &[NonNull<u8>]);
    fn populate_sizes(&mut self, sizes: &mut [MaybeUninit<usize>]);
}

pub(crate) struct Chain<'a, 'b, A, F, B>
where
    F: FnOnce(&mut [std::mem::MaybeUninit<A>]),
    B: ChainNode,
{
    gsb: &'a mut GuardedSliceBuilder<'b, A, F>,
    next: B,
}

impl<'a, 'b, A, F, B> Chain<'a, 'b, A, F, B>
where
    F: FnOnce(&mut [std::mem::MaybeUninit<A>]),
    B: ChainNode,
{
    pub(crate) fn new(gsb: &'a mut GuardedSliceBuilder<'b, A, F>, next: B) -> Self {
        Self { gsb, next }
    }
}

impl Sealed for () {}

impl ChainNode for () {
    const INDEX: usize = usize::MAX;
    const MAX_ALIGNMENT: usize = 1;
    #[allow(private_interfaces)]
    const SORTED_INDICES_AND_SIZES: [IndexAndAlign; MAX_CHAIN_LENGTH] = [IndexAndAlign {
        index: usize::MAX,
        align: 0,
    }; MAX_CHAIN_LENGTH];
    fn distribute_pointers(&mut self, _sizes: &[NonNull<u8>]) {}
    fn populate_sizes(&mut self, _sizes: &mut [MaybeUninit<usize>]) {}
}

impl<A, F, B> Sealed for Chain<'_, '_, A, F, B>
where
    F: FnOnce(&mut [std::mem::MaybeUninit<A>]),
    B: ChainNode,
{
}

impl<A, F, B> ChainNode for Chain<'_, '_, A, F, B>
where
    F: FnOnce(&mut [std::mem::MaybeUninit<A>]),
    B: ChainNode,
{
    const INDEX: usize = B::INDEX.wrapping_add(1);
    const MAX_ALIGNMENT: usize = max(core::mem::align_of::<A>(), B::MAX_ALIGNMENT);
    const SORTED_INDICES_AND_SIZES: [IndexAndAlign; MAX_CHAIN_LENGTH] = sorted_insert(
        Self::INDEX,
        IndexAndAlign {
            index: Self::INDEX,
            align: core::mem::align_of::<A>(),
        },
        B::SORTED_INDICES_AND_SIZES,
    );
    fn distribute_pointers(&mut self, ptrs: &[NonNull<u8>]) {
        self.gsb.set_ptr(ptrs[Self::INDEX].cast());
        self.next.distribute_pointers(ptrs);
    }
    fn populate_sizes(&mut self, sizes: &mut [MaybeUninit<usize>]) {
        sizes[Self::INDEX].write(core::mem::size_of::<A>() * self.gsb.n());
        self.next.populate_sizes(sizes);
    }
}
