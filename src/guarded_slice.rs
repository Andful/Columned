use core::{mem::MaybeUninit, ptr::NonNull};
use std::mem::{ManuallyDrop, forget, replace, take, transmute};

/// A slice, which memory is managed by a [Guard].
pub struct Guarded<'a, T>(&'a mut T)
where
    T: ?Sized;

impl<'a, T> Guarded<'a, T> {
    ///Return the underling slice. This will cause [Drop::drop] of `T` to not be called.
    pub fn forget(self) -> &'a mut T {
        let result = self.0 as *mut T; // This is probably wrong.
        let _ = unsafe { transmute::<Guarded<'a, T>, Guarded<'a, ManuallyDrop<T>>>(self) };
        unsafe { &mut *result }
    }
}

impl<'a, T> Guarded<'a, [T]> {
    ///Return the underling slice. This will cause [Drop::drop] of `T` to not be called.
    pub fn forget(mut self) -> &'a mut [T] {
        take(&mut self.0)
    }
}

impl<'a, T> Guarded<'a, MaybeUninit<T>> {
    ///Assume that the content of the slice are all initialized.
    /// # Safety
    /// All the elements of the slice must be initialized.
    pub unsafe fn assume_init(self) -> Guarded<'a, T> {
        unsafe { transmute::<Guarded<'a, MaybeUninit<T>>, Guarded<'a, T>>(self) }
    }
}

impl<'a, T> Guarded<'a, [MaybeUninit<T>]> {
    ///Assume that the content of the slice are all initialized.
    /// # Safety
    /// All the elements of the slice must be initialized.
    pub unsafe fn assume_init(self) -> Guarded<'a, [T]> {
        unsafe { transmute::<Guarded<'a, [MaybeUninit<T>]>, Guarded<'a, [T]>>(self) }
    }
}

impl<'a, T> Guarded<'a, T>
where
    T: ?Sized + Copy
{
    ///Return the underling slice. This is equivalent to [GuardedSlice::forget], but requires `T: Copy`. Therefore, it would lead to a compilation error if `Drop` is implemented for `T`.
    pub fn into_mut(self) -> &'a mut T {
        self.forget()
    }
}

impl<'a, T> Guarded<'a, [T]>
where
    T: Copy,
{
    ///Return the underling slice. This is equivalent to [GuardedSlice::forget], but requires `T: Copy`. Therefore, it would lead to a compilation error if `Drop` is implemented for `T`.
    pub fn into_mut(self) -> &'a mut [T] {
        self.forget()
    }
}

impl<T> core::ops::Deref for Guarded<'_, T>
where
    T: ?Sized,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<T> core::ops::DerefMut for Guarded<'_, T>
where
    T: ?Sized,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl<T> Drop for Guarded<'_, T>
where
    T: ?Sized,
{
    fn drop(&mut self) {
        if core::mem::needs_drop::<T>() {
            let to_drop = unsafe { core::mem::transmute::<&mut T, &mut ManuallyDrop<T>>(self.0) };
            unsafe { ManuallyDrop::drop(to_drop) };
        }
    }
}

impl<T> core::fmt::Debug for Guarded<'_, T>
where
    T: core::fmt::Debug + ?Sized,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self.0, f)
    }
}

#[derive(Debug)]
pub(crate) struct GuardedBuilderInner {
    pub(crate) next: Option<*mut GuardedBuilderInner>,
    pub(crate) ptr: Option<NonNull<u8>>,
    pub(crate) align: usize,
    pub(crate) size: usize,
}

/// Prepare an allocation of a slice, by specifying its size and
/// its initialization function.
/// The initialization function will be called upon the call of
/// [allocate], [allocate_in], [with_allocation] or [with_allocation_in].
pub struct GuardedBuilder<'guard, T>
where
    T: ?Sized,
{
    pub(crate) inner: GuardedBuilderInner,
    n: usize,
    pd: ::core::marker::PhantomData<&'guard T>,
}

impl<'a, T> GuardedBuilder<'a, [T]> {
    ///Prepare an allocation of a slice, by specifying its size and
    /// its initialization function.
    /// # Safety
    /// the initialization function `init` must initialize every element of its argument.
    pub fn new_slice(n: usize) -> Self {
        Self {
            inner: GuardedBuilderInner {
                next: None,
                ptr: None,
                align: align_of::<T>(),
                size: size_of::<T>() * n,
            },
            n,
            pd: Default::default(),
        }
    }

    ///Build a GuardedSlice.
    /// # Errors
    /// Returning `Err` indicates that this [GuardedSliceBuilder] was not
    /// [Subscriber::subscribe]d and successfully [Subscriber::finish]ed.
    pub unsafe fn build(self, init: impl FnOnce(&mut [MaybeUninit<T>])) -> Guarded<'a, [T]> {
        let Self {
            inner: GuardedBuilderInner { ptr, .. },
            n,
            ..
        } = self;
        let Some(ptr) = ptr else {
            panic!(
                "Attempting to build GuardedSliceBuilder that was not subscribed and for which the Subscriber successfully finished"
            );
        };
        let elem = ptr.as_ptr().cast::<MaybeUninit<T>>().cast_slice(n);
        debug_assert!(elem.is_aligned_to(align_of::<T>()));
        let elem = unsafe { &mut *elem };
        init(elem);
        Guarded(unsafe { std::mem::transmute::<&mut [MaybeUninit<T>], &mut [T]>(elem) })
    }

    ///Build slice initializing the value determined by `f`.
    pub fn build_from_fn(self, mut f: impl FnMut(usize) -> T) -> Guarded<'a, [T]> {
        unsafe {
            self.build(|elem| {
                elem.iter_mut().enumerate().for_each(|(i, e)| {
                    e.write(f(i));
                });
            })
        }
    }

    ///Build slice by initializing its values to [Default::default].
    pub fn build_default(self) -> Guarded<'a, [T]>
    where
        T: Default,
    {
        self.build_from_fn(|_| T::default())
    }
}

impl<'a, T> GuardedBuilder<'a, [MaybeUninit<T>]> {
    ///Build slice uninitialized.
    pub fn build_uninit(self) -> Guarded<'a, [MaybeUninit<T>]> {
        unsafe { self.build(|_| ()) }
    }
}

impl<'a, T> GuardedBuilder<'a, T>
where
    T: Sized,
{
    ///Prepare an allocation of a slice, by specifying its size and
    /// its initialization function.
    /// # Safety
    /// the initialization function `init` must initialize every element of its argument.
    pub fn new() -> Self {
        Self {
            inner: GuardedBuilderInner {
                next: None,
                ptr: None,
                align: align_of::<T>(),
                size: size_of::<T>(),
            },
            n: 1,
            pd: Default::default(),
        }
    }

    ///Build a GuardedSlice.
    /// # Errors
    /// Returning `Err` indicates that this [GuardedSliceBuilder] was not
    /// [Subscriber::subscribe]d and successfully [Subscriber::finish]ed.
    pub unsafe fn build(self, init: impl FnOnce(&mut MaybeUninit<T>)) -> Guarded<'a, T> {
        let Self {
            inner: GuardedBuilderInner { ptr, .. },
            ..
        } = self;
        let Some(ptr) = ptr else {
            panic!(
                "Attempting to build GuardedSliceBuilder that was not subscribed and for which the Subscriber successfully finished"
            );
        };
        let elem = ptr.as_ptr().cast::<MaybeUninit<T>>();
        let elem = unsafe { &mut *elem };
        init(elem);
        Guarded(unsafe { std::mem::transmute::<&mut MaybeUninit<T>, &mut T>(elem) })
    }

    ///Build slice initializing the value determined by `f`.
    pub fn build_from_fn(self, mut f: impl FnMut() -> T) -> Guarded<'a, T> {
        unsafe {
            self.build(|elem| {
                elem.write(f());
            })
        }
    }

    ///Build slice by initializing its values to [Default::default].
    pub fn build_default(self) -> Guarded<'a, T>
    where
        T: Default,
    {
        self.build_from_fn(|| T::default())
    }
}

impl<'a, T> GuardedBuilder<'a, MaybeUninit<T>> {
    ///Build slice uninitialized.
    pub fn build_uninit(self) -> Guarded<'a, MaybeUninit<T>> {
        unsafe { self.build(|_| ()) }
    }
}
