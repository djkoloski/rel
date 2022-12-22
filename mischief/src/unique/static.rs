use ::core::{
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
};

use crate::{GhostMut, GhostRef, Slot, Unique};

/// A type that can lease access to a type without any context.
pub trait Static: Sized {
    /// The unique type that can be used to lease this static memory location.
    type Unique: Unique;
    /// The type that a reference can be created to.
    type Target;

    /// Returns a mutable reference to a slot of the target type.
    fn slot(ghost: GhostMut<'_, Self::Unique>) -> Slot<'_, Self::Target>;

    /// Returns a shared reference to the target type.
    ///
    /// # Safety
    ///
    /// The slot returned from `slot` be initialized.
    unsafe fn value(ghost: GhostRef<'_, Self::Unique>) -> &'_ Self::Target;
}

/// A lease on a static memory location for a statically-checked lifetime.
#[derive(Unique)]
#[mischief = "crate"]
#[repr(transparent)]
pub struct StaticVal<'a, S: Static> {
    #[unique]
    ghost: GhostMut<'a, S::Unique>,
}

impl<S: Static> Drop for StaticVal<'_, S> {
    fn drop(&mut self) {
        let mut slot = S::slot(self.ghost.as_mut());
        // SAFETY: This `StaticVal` owns the value in the static variable, so it
        // may consume the value by dropping it.
        unsafe { slot.assume_init_drop() }
    }
}

impl<'a, S: Static> StaticVal<'a, S> {
    /// Creates a new scope from a unique borrow and an initial value.
    pub fn new(unique: &'a mut S::Unique, value: S::Target) -> Self {
        let mut ghost = GhostMut::new(unique);
        let mut slot = S::slot(ghost.as_mut());
        slot.write(value);
        Self { ghost }
    }

    /// Consumes the `StaticVal`, returning the static value it contained.
    pub fn read(self) -> S::Target {
        let mut this = ManuallyDrop::new(self);
        let slot = S::slot(this.ghost.as_mut());
        let maybe_uninit = slot.as_maybe_uninit();
        // SAFETY:
        // - This `StaticVal` initialized the value of the slot when it was
        //   created.
        // - `self` will not drop the value of the slot because it was moved
        //   into a `ManuallyDrop`.
        unsafe { maybe_uninit.assume_init_read() }
    }

    /// Creates a mutable borrow of this static value.
    pub fn as_mut(&mut self) -> StaticMut<'_, S> {
        StaticMut {
            ghost: self.ghost.as_mut(),
        }
    }

    /// Creates a shared borrow of this static value.
    pub fn as_ref(&self) -> StaticRef<'_, S> {
        StaticRef {
            ghost: self.ghost.as_ref(),
        }
    }
}

impl<S: Static> Deref for StaticVal<'_, S> {
    type Target = S::Target;

    fn deref(&self) -> &Self::Target {
        // SAFETY: This `StaticVal` initialized the slot when it was created.
        unsafe { S::value(self.ghost.as_ref()) }
    }
}

impl<S: Static> DerefMut for StaticVal<'_, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let mut slot = S::slot(self.ghost.as_mut());
        // SAFETY: This `StaticVal` initialized the slot when it was created.
        unsafe { slot.assume_init_mut() }
    }
}

/// A mutable reference to some static value.
#[repr(transparent)]
pub struct StaticMut<'a, S: Static> {
    ghost: GhostMut<'a, S::Unique>,
}

impl<'a, S: Static> StaticMut<'a, S> {
    /// Creates a mutable borrow of this static value.
    pub fn as_mut(&mut self) -> StaticMut<'_, S> {
        StaticMut {
            ghost: self.ghost.as_mut(),
        }
    }

    /// Creates a shared borrow of this static value.
    pub fn as_ref(&self) -> StaticRef<'_, S> {
        StaticRef {
            ghost: self.ghost.as_ref(),
        }
    }
}

impl<S: Static> Deref for StaticMut<'_, S> {
    type Target = S::Target;

    fn deref(&self) -> &Self::Target {
        // SAFETY: The `StaticVal` that this `StaticMut` is borrowed from
        // initialized the slot when it was created.
        unsafe { S::value(self.ghost.as_ref()) }
    }
}

impl<S: Static> DerefMut for StaticMut<'_, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let mut slot = S::slot(self.ghost.as_mut());
        // SAFETY: The `StaticVal` that this `StaticMut` is borrowed from
        // initialized the slot when it was created.
        unsafe { slot.assume_init_mut() }
    }
}

/// A reference to some static value.
#[repr(transparent)]
pub struct StaticRef<'a, S: Static> {
    ghost: GhostRef<'a, S::Unique>,
}

impl<S: Static> Clone for StaticRef<'_, S> {
    #[inline]
    fn clone(&self) -> Self {
        Self { ghost: self.ghost }
    }
}

impl<S: Static> Copy for StaticRef<'_, S> {}

impl<S: Static> Deref for StaticRef<'_, S> {
    type Target = S::Target;

    fn deref(&self) -> &Self::Target {
        // SAFETY: The `StaticVal` that this `StaticRef` is borrowed from
        // initialized the slot when it was created.
        unsafe { S::value(self.ghost) }
    }
}

/// Creates a type that provides safe access to a static variable using a unique
/// value.
#[macro_export]
macro_rules! lease_static {
    ($unique:ty => $name:ident: $ty:ty) => {
        $crate::lease_static!(@declare $name);
        $crate::lease_static!(@impl $unique => $name: $ty)
    };
    ($unique:ty => pub $name:ident: $ty:ty) => {
        $crate::lease_static!(@declare $name pub);
        $crate::lease_static!(@impl $unique => $name: $ty)
    };
    ($unique:ty => pub ($($vis:tt)*) $name:ident: $ty:ty) => {
        $crate::lease_static!(@declare $name pub($($vis)*));
        $crate::lease_static!(@impl $unique => $name: $ty)
    };
    (@declare $name:ident $($vis:tt)*) => {
        $($vis)* struct $name(::core::marker::PhantomData<()>);
    };
    (@impl $unique:ty => $name:ident: $target:ty) => {
        const _: () = {
            use ::core::mem::MaybeUninit;
            static mut VALUE: MaybeUninit<$target> = MaybeUninit::uninit();

            impl $crate::Static for $name {
                type Unique = $unique;
                type Target = $target;

                fn slot(
                    _: $crate::GhostMut<'_, Self::Unique>,
                ) -> $crate::Slot<'_, Self::Target> {
                    // SAFETY: Holding a `GhostMut` of the `Static`'s `Unique`
                    // value conveys mutable access to the underlying slot.
                    unsafe { $crate::Slot::new(&mut VALUE) }
                }

                unsafe fn value(
                    _: $crate::GhostRef<'_, Self::Unique>,
                ) -> &'_ Self::Target {
                    // SAFETY: The caller has guaranteed that `VALUE` is
                    // initialized.
                    unsafe { VALUE.assume_init_ref() }
                }
            }
        };
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn vend() {
        use crate::{runtime_token, StaticVal};

        struct Gumball {
            size: i32,
        }

        runtime_token!(Quarter);
        lease_static!(Quarter => Vend: Gumball);

        let mut quarter = Quarter::acquire();
        let mut vend =
            StaticVal::<Vend>::new(&mut quarter, Gumball { size: 100 });

        {
            assert_eq!(::core::mem::size_of_val(&vend.as_ref()), 0);

            let mut gumball = &mut *vend;
            gumball.size = 6;

            assert_eq!(vend.as_ref().size, 6);

            let mut gumball_2 = &mut *vend;
            gumball_2.size = 4;

            assert_eq!(vend.as_ref().size, 4);
        }
    }
}
