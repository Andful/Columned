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

type NopInitializer<T> = fn(&mut [core::mem::MaybeUninit<core::mem::MaybeUninit<T>>]);

/// Prepare an allocation of a slice, by specifying its size and
/// its initialization function.
/// The initialization function will be called upon the call of
/// [allocate], [allocate_in], [with_allocation] or [with_allocation_in].
pub struct GuardedSliceBuilder<'a, T, F>
where
    F: FnOnce(&mut [::core::mem::MaybeUninit<T>]),
{
    ptr: Option<NonNull<MaybeUninit<T>>>,
    n: usize,
    init: F,
    pd: ::core::marker::PhantomData<&'a T>,
}

fn nop<T>(_: &mut [MaybeUninit<MaybeUninit<T>>]) {}

impl<T> GuardedSliceBuilder<'_, MaybeUninit<T>, fn(&mut [MaybeUninit<MaybeUninit<T>>])> {
    ///Prepare an allocation of a slice, by specifying only its size, but for which, its initialization is deferred to after allocation with [GuardedSlice::assume_init].
    pub fn new_uninit(n: usize) -> Self {
        unsafe { GuardedSliceBuilder::new(n, nop) }
    }
}

fn write_default<T: Default>(e: &mut [MaybeUninit<T>]) {
    e.iter_mut().for_each(|e| {
        e.write(T::default());
    });
}

impl<T> GuardedSliceBuilder<'_, T, fn(&mut [MaybeUninit<T>])>
where
    T: Default,
{
    ///Prepare an allocation of a slice, which its initial values will be [Default].
    pub fn new_default(n: usize) -> Self {
        unsafe { GuardedSliceBuilder::new(n, write_default) }
    }
}

impl<'a, T, F> GuardedSliceBuilder<'a, T, F>
where
    F: FnOnce(&mut [::core::mem::MaybeUninit<T>]),
{
    ///Prepare an allocation of a slice, by specifying its size and
    /// its initialization function.
    /// # Safety
    /// the initialization function `init` must initialize every element of its argument.
    pub unsafe fn new(n: usize, init: F) -> Self {
        Self {
            ptr: None,
            n,
            init,
            pd: Default::default(),
        }
    }

    pub(crate) fn set_ptr(&mut self, ptr: NonNull<MaybeUninit<T>>) {
        self.ptr = Some(ptr)
    }

    pub(crate) fn n(&self) -> usize {
        self.n
    }

    ///Build a GuardedSlice.
    /// # Errors
    /// Returning `Err` indicates that this [GuardedSliceBuilder] was not
    /// [Subscriber::subscribe]d and successfully [Subscriber::finish]ed.
    pub fn build(self) -> GuardedSlice<'a, T> {
        let Some(ptr) = self.ptr else {
            panic!(
                "Attempting to build GuardedSliceBuilder that was not subscribed and for which the Subscriber successfully finished"
            );
        };
        let slice = slice_from_raw_parts_mut(ptr.as_ptr(), self.n);
        let slice = unsafe { &mut *slice };
        (self.init)(slice);
        GuardedSlice(unsafe { std::mem::transmute::<&mut [MaybeUninit<T>], &mut [T]>(slice) })
    }
}

impl<T> GuardedSliceBuilder<'_, core::mem::MaybeUninit<T>, NopInitializer<T>> {}
