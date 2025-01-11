use core::{
    borrow::Borrow,
    cmp,
    marker::{PhantomData, PhantomPinned},
    ops::Deref,
    ptr::NonNull,
};

use abs_buff::{TrBuffSegmRef, TrBuffSegmView};

use super::{
    reclaim_::BuffSegmReclaim,
    NoReclaim, TrReclaim,
};

/// A wrapper around a slice borrowed from a buffer and its reclaim function.
/// Designed for [RingBuffer](crate::ring_buffer::RingBuffer) but capable of
/// being a simple stream buffer to support the consuming semantics.
#[repr(C)]
pub struct SegmRef<B, T, R>
where
    B: Borrow<[T]>,
    R: TrReclaim<T>,
{
    _mark_t_: PhantomData<[T]>,
    _pinned_: PhantomPinned,
    offset_: usize,
    reclaim_: Option<R>,
    slice_ref_: B,
}

impl<B, T, R> SegmRef<B, T, R>
where
    B: Borrow<[T]>,
    R: TrReclaim<T>,
{
    /// Create by borrowing a slice from an implicit source. And the items of 
    /// this slice will be returned back to or moved out of the source by
    /// `reclaim`.
    pub const fn new(slice: B, reclaim: Option<R>) -> Self {
        SegmRef {
            _mark_t_: PhantomData,
            _pinned_: PhantomPinned,
            offset_: 0usize,
            reclaim_: reclaim,
            slice_ref_: slice,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.slice_ref_.borrow().len() - self.offset_
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.slice_ref_.borrow().len() == self.offset_
    }

    pub fn as_slice(&self) -> &[T] {
        let slice: &[T] = self.slice_ref_.borrow();
        debug_assert!(self.offset_ <= slice.len());
        &slice[self.offset_..]
    }

    pub fn take_segm_ref(
        &mut self,
        length: usize,
    ) -> SegmRef<&[T], T, BuffSegmReclaim> {
        unsafe {
            let mut this_ptr = NonNull::new_unchecked(self);
            let slice = this_ptr.as_ref().as_slice();
            let size = cmp::min(length, slice.len());
            let slice = &slice[..size];
            let offset_ptr =
                NonNull::new_unchecked(&mut this_ptr.as_mut().offset_);
            let reclaim = Option::Some(BuffSegmReclaim::new(offset_ptr));
            SegmRef::<&[T], T, BuffSegmReclaim>::new(slice, reclaim)
        }
    }
}

impl<B, T> SegmRef<B, T, NoReclaim>
where
    B: Borrow<[T]>,
{
    /// Create by borrowing a slice from an implicit source but no reclaim 
    #[inline]
    pub const fn no_reclaim(slice: B) -> Self {
        SegmRef::new(slice, Option::None)
    }
}

impl<B, T, R> Drop for SegmRef<B, T, R>
where
    B: Borrow<[T]>,
    R: TrReclaim<T>,
{
    fn drop(&mut self) {
        let Option::Some(mut r) = self.reclaim_.take() else {
            return;
        };
        r.reclaim(self)
    }
}

impl<B, T, R> Deref for SegmRef<B, T, R>
where
    B: Borrow<[T]>,
    R: TrReclaim<T>,
{
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<B, T, R> Borrow<[T]> for SegmRef<B, T, R>
where
    B: Borrow<[T]>,
    R: TrReclaim<T>,
{
    #[inline]
    fn borrow(&self) -> &[T] {
        self.as_slice()
    }
}

impl<B, T, R> AsRef<[T]> for SegmRef<B, T, R>
where
    B: Borrow<[T]>,
    R: TrReclaim<T>,
{
    #[inline]
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<B, T, R> TrBuffSegmView for SegmRef<B, T, R>
where
    B: Borrow<[T]>,
    R: TrReclaim<T>,
{
    type Item = T;

    fn is_empty(&self) -> bool {
        SegmRef::is_empty(self)
    }

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

impl<B, T, R> TrBuffSegmRef<T> for SegmRef<B, T, R>
where
    B: Borrow<[T]>,
    R: TrReclaim<T>,
{
    #[inline]
    fn take_segm_ref(&mut self, length: usize) -> impl TrBuffSegmRef<T> {
        SegmRef::take_segm_ref(self, length)
    }
}

#[cfg(test)]
mod tests_ {
    use super::SegmRef;

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
        let taken_slice = segm.take_segm_ref(SLICE_LEN);
        for (u, x) in taken_slice.as_ref().iter().enumerate() {
            assert_eq!(*x, u)
        }
        drop(taken_slice);
        assert_eq!(segm.len(), buff.len() - SLICE_LEN);

        let taken_slice = segm.take_segm_ref(ARR_SIZE);
        assert_eq!(taken_slice.len(), buff.len() - SLICE_LEN);
        for (u, x) in taken_slice.as_ref().iter().enumerate() {
            assert_eq!(*x, u + SLICE_LEN)
        }
        drop(taken_slice);
        assert_eq!(segm.len(), 0);
    }
}
