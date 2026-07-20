use core::{
    mem::MaybeUninit,
    ptr::{NonNull, slice_from_raw_parts_mut},
};

/// A slice, which memory is managed by a [Guard].
pub struct GuardedSlice<'a, T>(&'a mut [T]);

impl<'a, T> GuardedSlice<'a, T> {
    ///Return the underling slice. This will cause [Drop::drop] of `T` to not be called.
    pub fn forget(mut self) -> &'a mut [T] {
        core::mem::take(&mut self.0)
    }
}

impl<'a, T> GuardedSlice<'a, core::mem::MaybeUninit<T>> {
    ///Assume that the content of the slice are all initialized.
    /// # Safety
    /// All the elements of the slice must be initialized.
    pub unsafe fn assume_init(mut self) -> GuardedSlice<'a, T> {
        GuardedSlice(unsafe {
            core::mem::transmute::<&'a mut [core::mem::MaybeUninit<T>], &'a mut [T]>(
                core::mem::take(&mut self.0),
            )
        })
    }
}

impl<'a, T> GuardedSlice<'a, T>
where
    T: Copy,
{
    ///Return the underling slice. This is equivalent to [GuardedSlice::forget], but requires `T: Copy`. Therefore, it would lead to a compilation error if `Drop` is implemented for `T`.
    pub fn into_slice(self) -> &'a mut [T] {
        self.forget()
    }
}

impl<T> core::ops::Deref for GuardedSlice<'_, T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<T> core::ops::DerefMut for GuardedSlice<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl<T> Drop for GuardedSlice<'_, T> {
    fn drop(&mut self) {
        if core::mem::needs_drop::<T>() {
            let to_drop = core::mem::take(&mut self.0);
            let to_drop = unsafe {
                core::mem::transmute::<&mut [T], &mut [core::mem::ManuallyDrop<T>]>(to_drop)
            };
            to_drop.iter_mut().for_each(|e| unsafe {
                core::mem::ManuallyDrop::drop(e);
            });
        }
    }
}

impl<T> core::fmt::Debug for GuardedSlice<'_, T>
where
    T: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self.0, f)
    }
}

#[derive(Debug)]
pub(crate) struct GuardedSliceBuilderInner {
    pub(crate) next: Option<*mut GuardedSliceBuilderInner>,
    pub(crate) ptr: Option<NonNull<u8>>,
    pub(crate) align: usize,
    pub(crate) size: usize,
    pub(crate) n: usize,
}

/// Prepare an allocation of a slice, by specifying its size and
/// its initialization function.
/// The initialization function will be called upon the call of
/// [allocate], [allocate_in], [with_allocation] or [with_allocation_in].
pub struct GuardedSliceBuilder<'guard, T> {
    pub(crate) inner: GuardedSliceBuilderInner,
    pd: ::core::marker::PhantomData<&'guard T>,
}

impl<'a, T> GuardedSliceBuilder<'a, T> {
    ///Prepare an allocation of a slice, by specifying its size and
    /// its initialization function.
    /// # Safety
    /// the initialization function `init` must initialize every element of its argument.
    pub fn new(n: usize) -> Self {
        Self {
            inner: GuardedSliceBuilderInner {
                next: None,
                ptr: None,
                align: align_of::<T>(),
                size: size_of::<T>(),
                n,
            },
            pd: Default::default(),
        }
    }

    ///Build a GuardedSlice.
    /// # Errors
    /// Returning `Err` indicates that this [GuardedSliceBuilder] was not
    /// [Subscriber::subscribe]d and successfully [Subscriber::finish]ed.
    pub unsafe fn build(self, init: impl FnOnce(&mut [MaybeUninit<T>])) -> GuardedSlice<'a, T> {
        let Self {
            inner: GuardedSliceBuilderInner { ptr, n, .. },
            ..
        } = self;
        let Some(ptr) = ptr else {
            panic!(
                "Attempting to build GuardedSliceBuilder that was not subscribed and for which the Subscriber successfully finished"
            );
        };
        let slice = slice_from_raw_parts_mut(ptr.as_ptr().cast::<MaybeUninit<T>>(), n);
        let slice = unsafe { &mut *slice };
        init(slice);
        GuardedSlice(unsafe { std::mem::transmute::<&mut [MaybeUninit<T>], &mut [T]>(slice) })
    }

    pub fn build_from_fn(self, mut f: impl FnMut(usize) -> T) -> GuardedSlice<'a, T> {
        unsafe {
            self.build(|slice| {
                slice.iter_mut().enumerate().for_each(|(i, e)| {
                    e.write(f(i));
                })
            })
        }
    }

    pub fn build_default(self) -> GuardedSlice<'a, T>
    where
        T: Default,
    {
        self.build_from_fn(|_| T::default())
    }
}

impl<'a, T> GuardedSliceBuilder<'a, MaybeUninit<T>> {
    pub fn build_uninit(self) -> GuardedSlice<'a, MaybeUninit<T>> {
        unsafe { self.build(|_| ()) }
    }
}
