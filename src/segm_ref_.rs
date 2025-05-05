use core::{
    borrow::Borrow,
    iter::Iterator,
    marker::PhantomData,
    ops::Deref,
    ptr::NonNull,
};

use abs_buff::{Demand, TrBuffSegmRef, TrBuffSegmView};
use abs_iter::TrItemsRefView;

use crate::NoForward;

use super::forward_::{IncrConsumed, TrForward};

/// A wrapper around a slice borrowed from a buffer and its reclaim function.
/// Designed for [RingBuffer](crate::ring_buffer::RingBuffer) but capable of
/// being a simple stream buffer to support the consuming semantics.
#[repr(C)]
pub struct SegmRef<B, T, F>
where
    B: Borrow<[T]>,
    F: TrForward,
{
    _using_t_: PhantomData<[T]>,
    consumed_: usize,
    forward_: F,
    slice_ref_: B,
}

impl<B, T, F> SegmRef<B, T, F>
where
    B: Borrow<[T]>,
    F: TrForward,
{
    /// Create by borrowing a slice from an implicit source. And the items of 
    /// this slice will be returned back to or moved out of the source by
    /// `reclaim`.
    pub const fn new(slice: B, forward: F) -> Self {
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
        self.capacity() - self.consumed_
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

    pub fn take_segm_ref<'f>(
        &'f mut self,
        length: Demand<usize>,
    ) -> Option<SegmRef<&'f [T], T, IncrConsumed>> {
        let Option::Some(size) = length.most_with(self.len()) else {
            return Option::None;
        };
        unsafe {
            let this_ptr = NonNull::new_unchecked(self);
            let slice = this_ptr.as_ref().as_slice();
            let slice = &slice[..size];
            let forward = IncrConsumed::new(&mut self.consumed_);
            Option::Some(SegmRef::new(slice, forward))
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
        SegmRef::new(slice, NoForward::new())
    }
}

impl<B, T, F> Drop for SegmRef<B, T, F>
where
    B: Borrow<[T]>,
    F: TrForward,
{
    fn drop(&mut self) {
        self.forward_.forward(self.capacity());
    }
}

impl<B, T, F> Deref for SegmRef<B, T, F>
where
    B: Borrow<[T]>,
    F: TrForward,
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
    F: TrForward,
{
    #[inline]
    fn borrow(&self) -> &[T] {
        self.as_slice()
    }
}

impl<B, T, F> AsRef<[T]> for SegmRef<B, T, F>
where
    B: Borrow<[T]>,
    F: TrForward,
{
    #[inline]
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<B, T, F> TrItemsRefView for SegmRef<B, T, F>
where
    B: Borrow<[T]>,
    F: TrForward,
{
    type Item = T;
    type View<'view> = &'view T
    where
        Self: 'view;

    fn items_ref_view(&self) -> impl Iterator<Item = Self::View<'_>> {
        self.as_slice().iter()
    }
}

impl<B, T, F> TrBuffSegmView for SegmRef<B, T, F>
where
    B: Borrow<[T]>,
    F: TrForward,
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
    fn len(&self) -> usize {
        SegmRef::len(self)
    }

    fn iter_ptr(&self) -> impl Iterator<Item = *const Self::Item> {
        self.slice_ref_
            .borrow()
            .iter()
            .map(|x| x as *const T)
    }
}

impl<B, T, F> TrBuffSegmRef<T> for SegmRef<B, T, F>
where
    B: Borrow<[T]>,
    F: TrForward,
{
    type Slice<'a> = &'a [T]
    where
        T: 'a,
        Self: 'a;


    type Segm<'a> = SegmRef<&'a [T], T, IncrConsumed>
    where
        T: 'a,
        Self: 'a;

    #[inline]
    fn take_segm_ref<'f>(
        &'f mut self,
        length: Demand<usize>,
    ) -> Option<Self::Segm<'f>> {
        SegmRef::take_segm_ref(self, length)
    }

    fn iter_slices<'a>(&'a mut self) -> impl IntoIterator<Item = Self::Slice<'a>>
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
        let Option::Some(taken_slice) = segm.take_segm_ref(Demand::at_most(SLICE_LEN)) else {
            panic!()
        };
        for (u, x) in taken_slice.as_ref().iter().enumerate() {
            assert_eq!(*x, u)
        }
        drop(taken_slice);
        assert_eq!(segm.len(), buff.len() - SLICE_LEN);

        let Option::Some(taken_slice) = segm.take_segm_ref(Demand::at_most(ARR_SIZE)) else {
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
