use core::{
    borrow::Borrow,
    iter::IntoIterator,
    marker::PhantomData,
    ops::{Deref, Try},
    ptr::NonNull,
};

use abs_buff::{
    x_deps::abs_iter,
    Demand, TrBuffSegmRef, TrBuffSegmView,
};
use abs_iter::{TrItemsRefView, TrAsSlice};

use super::forward_::{IncrConsumed, NoForward};

/// Wraps a single slice into a buffer (`TrBuffSegmRef`).
#[repr(C)]
pub struct SegmRef<B, T, F>
where
    B: Borrow<[T]>,
    F: FnOnce(usize),
{
    _using_t_: PhantomData<[T]>,
    consumed_: usize,
    forward_: Option<F>,
    slice_ref_: B,
}

impl<B, T, F> SegmRef<B, T, F>
where
    B: Borrow<[T]>,
    F: FnOnce(usize),
{
    /// Create by borrowing a slice from an implicit source. And the items of 
    /// this slice will be returned back to or moved out of the source by
    /// `reclaim`.
    pub const fn new(slice: B, forward: Option<F>) -> Self {
        SegmRef {
            _using_t_: PhantomData,
            consumed_: 0usize,
            forward_: forward,
            slice_ref_: slice,
        }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.slice_ref_.borrow().len()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.as_slice().len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.capacity() == self.consumed_
    }

    pub fn as_slice(&self) -> &[T] {
        let slice: &[T] = self.slice_ref_.borrow();
        debug_assert!(self.consumed_ <= slice.len());
        &slice[self.consumed_..]
    }

    pub fn iter_slices<'a>(
        &'a self,
    ) -> impl IntoIterator<Item: TrAsSlice<Elem = T>>
    where
        T: 'a
    {
        let opt = if self.is_empty() {
            Option::None
        } else {
            let slice_ref: &[T] = self.slice_ref_.borrow();
            let slice = &slice_ref[self.consumed_..];
            Option::Some(slice)
        };
        opt.into_iter()
    }

    pub fn take_segm_ref<'f>(
        &'f mut self,
        length: &Demand<usize>,
    ) -> Option<SegmRef<&'f [T], T, IncrConsumed>> {
        let demand = length.narrow_from_max(self.len())?;
        let size = demand.max().cloned()?;
        unsafe {
            let this_ptr = NonNull::new_unchecked(self);
            let slice = this_ptr.as_ref().as_slice();
            let slice = &slice[..size];
            let forward = IncrConsumed::new(&mut self.consumed_);
            Option::Some(SegmRef::new(slice, Option::Some(forward)))
        }
    }
}

impl<B, T> SegmRef<B, T, NoForward>
where
    B: Borrow<[T]>,
{
    /// Create by borrowing a slice from an implicit source but no reclaim 
    #[inline]
    pub const fn no_reclaim(slice: B) -> Self {
        SegmRef::new(slice, Option::Some(NoForward::new()))
    }
}

impl<B, T, F> Drop for SegmRef<B, T, F>
where
    B: Borrow<[T]>,
    F: FnOnce(usize),
{
    fn drop(&mut self) {
        if let Option::Some(f) = self.forward_.take() {
            f(self.capacity());
        }
    }
}

impl<B, T, F> Deref for SegmRef<B, T, F>
where
    B: Borrow<[T]>,
    F: FnOnce(usize),
{
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<B, T, F> Borrow<[T]> for SegmRef<B, T, F>
where
    B: Borrow<[T]>,
    F: FnOnce(usize),
{
    #[inline]
    fn borrow(&self) -> &[T] {
        self.as_slice()
    }
}

impl<B, T, F> AsRef<[T]> for SegmRef<B, T, F>
where
    B: Borrow<[T]>,
    F: FnOnce(usize),
{
    #[inline]
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<B, T, F> TrAsSlice for SegmRef<B, T, F>
where
    B: Borrow<[T]>,
    F: FnOnce(usize),
{
    type Elem = T;
}

impl<B, T, F> TrItemsRefView for SegmRef<B, T, F>
where
    B: Borrow<[T]>,
    F: FnOnce(usize),
{
    type Item = T;

    fn items_ref_view(&self) -> impl IntoIterator<Item: Borrow<Self::Item>> {
        self.as_slice().iter()
    }
}

impl<B, T, F> TrBuffSegmView for SegmRef<B, T, F>
where
    B: Borrow<[T]>,
    F: FnOnce(usize),
{
    type Item = T;

    #[inline]
    fn capacity(&self) -> usize {
        SegmRef::capacity(self)
    }

    #[inline]
    fn is_empty(&self) -> bool {
        SegmRef::is_empty(self)
    }

    #[inline]
    fn iter_slices(
        &self,
    ) -> impl IntoIterator<Item: TrAsSlice<Elem = Self::Item>> {
        SegmRef::iter_slices(self)
    }
}

impl<B, T, F> TrBuffSegmRef<T> for SegmRef<B, T, F>
where
    B: Borrow<[T]>,
    F: FnOnce(usize),
{
    #[inline]
    fn take_segm_ref(
        &mut self,
        length: &Demand<usize>,
    ) -> impl Try<Output: TrBuffSegmRef<T>> {
        SegmRef::take_segm_ref(self, length)
    }
}

#[cfg(test)]
mod tests_ {
    use super::{Demand, SegmRef};

    #[test]
    fn segm_len_should_eq_as_slice_len() {
        const ARR_SIZE: usize = 64;
        let mut buff = [0usize; ARR_SIZE];
        for (u, x) in buff.iter_mut().enumerate() {
            *x = u
        }
        let mut segm = SegmRef::no_reclaim(buff.as_slice());
        let slice = segm.as_slice();
        assert_eq!(segm.len(), ARR_SIZE);
        assert_eq!(slice.len(), buff.len());

        const SLICE_LEN: usize = ARR_SIZE >> 1;
        let Option::Some(taken_slice) = segm.take_segm_ref(Demand::exactly(SLICE_LEN)) else {
            panic!()
        };
        for (u, x) in taken_slice.as_ref().iter().enumerate() {
            assert_eq!(*x, u)
        }
        drop(taken_slice);
        assert_eq!(segm.len(), buff.len() - SLICE_LEN);

        let Option::Some(taken_slice) = segm.take_segm_ref(Demand::with_max(ARR_SIZE)) else {
            panic!()
        };
        assert_eq!(taken_slice.len(), buff.len() - SLICE_LEN);
        for (u, x) in taken_slice.as_ref().iter().enumerate() {
            assert_eq!(*x, u + SLICE_LEN)
        }
        drop(taken_slice);
        assert_eq!(segm.len(), 0);
    }
}
