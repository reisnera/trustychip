use bitvec::prelude::*;

// Helper macros

macro_rules! concat_to_c_str {
    ( $($s:expr),+ ) => { {
        use std::os::raw::c_char;
        concat!($($s),+, "\0").as_ptr() as *const c_char
    } };
}

// Extension traits

/// An extension trait which adds methods to bitvec's BitSlice.
pub trait BitSliceExt {
    /// Splits a BitSlice into three BitSlices at the two provided indices.
    fn split_at_two(&self, first: usize, second: usize) -> (&Self, &Self, &Self);
}

impl<O, T> BitSliceExt for BitSlice<O, T>
where
    O: BitOrder,
    T: BitStore,
{
    #[inline]
    fn split_at_two(&self, first: usize, second: usize) -> (&Self, &Self, &Self) {
        assert!(first <= second, "first index must be <= second");
        assert!(second <= self.len(), "index out of bounds");
        let (a, rest) = unsafe { self.split_at_unchecked(first) };
        let (b, c) = unsafe { rest.split_at_unchecked(second - first) };
        (a, b, c)
    }
}
