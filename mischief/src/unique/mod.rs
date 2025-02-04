//! Unique types and tools for constructing and using them.

mod ghost;
mod r#static;
mod token;

pub use ::mischief_derive::Unique;

pub use self::{ghost::*, r#static::*, token::*};

/// A type which guarantees that only one value can ever exist at a time.
///
/// # Safety
///
/// Only one value of this type may ever exist simultaneously.
pub unsafe trait Unique {}

// SAFETY: Mutable references may not alias, so a mutable reference of a unique
// type must also be unique.
unsafe impl<T: Unique> Unique for &mut T {}

/// Splits a unique value into several others.
#[macro_export]
macro_rules! split_unique {
    (fn $fn:ident($in:ty) -> $out:ident) => {
        $crate::split_unique!(@define $in => ($out,));
        $crate::split_unique!(@impl $fn($in) -> $out);
    };
    (fn $fn:ident($in:ty) -> ($($out:tt)*)) => {
        $crate::split_unique!(@define $in => ($($out)*));
        $crate::split_unique!(@impl $fn($in) -> ($($out)*));
    };
    (pub fn $fn:ident($in:ty) -> $out:ident) => {
        $crate::split_unique!(@define $in => ($out,) $($vis)*);
        $crate::split_unique!(@impl $fn($in) -> $out pub);
    };
    (pub fn $fn:ident($in:ty) -> ($($out:tt)*)) => {
        $crate::split_unique!(@define $in => ($($out)*) $($vis)*);
        $crate::split_unique!(@impl $fn($in) -> ($($out)*) pub);
    };
    (pub($($vis:tt)*) fn $fn:ident($in:ty) -> $out:ident) => {
        $crate::split_unique!(@define $in => ($out,) $($vis)*);
        $crate::split_unique!(@impl $fn($in) -> $out pub($($vis)*));
    };
    (pub($($vis:tt)*) fn $fn:ident($in:ty) -> ($($out:tt)*)) => {
        $crate::split_unique!(@define $in => ($($out)*) $($vis)*);
        $crate::split_unique!(@impl $fn($in) -> ($($out)*) pub($($vis)*));
    };

    (@impl $fn:ident($in:ty) -> $out:ident $($vis:tt)*) => {
        #[inline]
        $($vis)* fn $fn(unique: $in) -> $out {
            // Forgetting is semantically equivalent to moving into static
            // variable permanently.
            ::core::mem::forget(unique);
            $out(::core::marker::PhantomData)
        }
    };
    (@impl $fn:ident($in:ty) -> ($($out:ident),*) $($vis:tt)*) => {
        #[inline]
        $($vis)* fn $fn(unique: $in) -> ($($out),*) {
            // Forgetting is semantically equivalent to moving into static
            // variable permanently.
            ::core::mem::forget(unique);
            ($(
                $out(::core::marker::PhantomData)
            ),*)
        }
    };
    (@impl $fn:ident($in:ty) -> ($($out:ident,)*) $($vis:tt)*) => {
        #[inline]
        $($vis)* fn $fn(unique: $in) -> ($($out,)*) {
            // Forgetting is semantically equivalent to moving into static
            // variable permanently.
            ::core::mem::forget(unique);
            ($(
                $out(::core::marker::PhantomData),
            )*)
        }
    };

    (@define $in:ty => () $($vis:tt)*) => {};
    (@define
        $in:ty => ($out_first:ident $(, $out_rest:ident)* $(,)?) $($vis:tt)*
    ) => {
        $($vis)* struct $out_first(
            ::core::marker::PhantomData<&'static mut $in>,
        );

        // SAFETY: `$out` can only be acquired by exchanging another `Unique`
        // for it. That unique value is retained indefinitely, so the exchange
        // can only ever be performed once.
        unsafe impl $crate::Unique for $out_first {}

        $crate::split_unique!(@define $in => ($($out_rest,)*) $($vis:tt)*);
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn split_unique() {
        use crate::{runtime_token, Unique};

        #[inline]
        fn assert_unique<T: Unique>() {}

        runtime_token!(Foo);
        split_unique!(fn barify(Foo) -> (Bar, Baz, Bat));

        assert_unique::<Foo>();
        assert_unique::<Bar>();
        assert_unique::<Baz>();
        assert_unique::<Bat>();

        let (bar, baz, bat): (Bar, Baz, Bat) = barify(Foo::acquire());
        assert!(matches!(Foo::try_acquire(), Err(_)));

        let _ = (bar, baz, bat);

        assert!(matches!(Foo::try_acquire(), Err(_)));
    }
}
