use core::{
    borrow::{Borrow, BorrowMut},
    cmp,
    marker::{PhantomData, PhantomPinned},
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use abs_buff::{TrBuffSegmMut, TrBuffSegmRef, TrBuffSegmView};

use super::{
    reclaim_::BuffSegmReclaim,
    NoReclaim, TrReclaim,
};

/// The rented slice for tx of the [RingBuffer](crate::ring_buffer::RingBuffer)
#[repr(C)]
pub struct SegmMut<B, T, R>
where
    B: BorrowMut<[MaybeUninit<T>]>,
    R: TrReclaim<MaybeUninit<T>>,
{
    _mark_t_: PhantomData<[T]>,
    _pinned_: PhantomPinned,
    offset_: usize,
    reclaim_: Option<R>,
    slice_mut_: B,
}

impl<B, T, R> SegmMut<B, T, R>
where
    B: BorrowMut<[MaybeUninit<T>]>,
    R: TrReclaim<MaybeUninit<T>>,
{
    /// Create by borrowing a mut slice from an implicit source. And the items
    /// of this mut slice will be returned back to or moved out from the source 
    /// by `reclaim`.
    ///
    /// ## Safety
    /// 
    /// - `reclaim` should be capable of semantically move item out when this
    ///     slice `into_iter`
    pub const fn new(slice_mut: B, reclaim: Option<R>) -> Self {
        SegmMut {
            _mark_t_: PhantomData,
            _pinned_: PhantomPinned,
            offset_: 0usize,
            reclaim_: reclaim,
            slice_mut_: slice_mut,
        }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.slice_mut_.borrow().len()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.capacity() - self.offset_
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.capacity() == self.offset_
    }

    pub fn as_slice(&self) -> &[MaybeUninit<T>] {
        let slice: &[MaybeUninit<T>] = self.slice_mut_.borrow();
        &slice[self.offset_..]
    }

    pub fn as_slice_mut(&mut self) -> &mut [MaybeUninit<T>] {
        let slice: &mut [MaybeUninit<T>] = self.slice_mut_.borrow_mut();
        &mut slice[self.offset_..]
    }

    pub fn dump_from<S: TrBuffSegmRef<T>>(
        &mut self,
        source: &mut S,
    ) -> usize {
        TrBuffSegmMut::dump_from_segm(self, source)
    }

    pub fn take_segm_mut(
        &mut self,
        length: usize,
    ) -> SegmMut<&mut [MaybeUninit<T>], T, BuffSegmReclaim> {
        unsafe {
            let mut this_ptr = NonNull::new_unchecked(self);
            let slice = this_ptr.as_mut().as_slice_mut();
            let size = cmp::min(length, slice.len());
            let slice = &mut slice[..size];
            let offset_ptr =
                NonNull::new_unchecked(&mut this_ptr.as_mut().offset_);
            let reclaim = Option::Some(BuffSegmReclaim::new(offset_ptr));
            SegmMut::<&mut [MaybeUninit<T>], T, BuffSegmReclaim>
                ::new(slice, reclaim)
        }
    }

    pub fn clone_from(&mut self, source: &[T]) -> usize
    where
        T: Clone,
    {
        TrBuffSegmMut::clone_from_slice(self, source)
    }
}

impl<B, T> SegmMut<B, T, NoReclaim>
where
    B: BorrowMut<[MaybeUninit<T>]>,
{
    /// Create by borrowing a slice from an implicit source. 
    ///
    /// ## Safety
    /// - `slice` must be managed by the source buffer;
    pub const fn no_reclaim(slice_mut: B) -> Self {
        SegmMut::new(slice_mut, Option::None)
    }
}

impl<B, T, R> Drop for SegmMut<B, T, R>
where
    B: BorrowMut<[MaybeUninit<T>]>,
    R: TrReclaim<MaybeUninit<T>>,
{
    fn drop(&mut self) {
        let Option::Some(mut r) = self.reclaim_.take() else {
            return;
        };
        r.reclaim(self)
    }
}

impl<B, T, R> Borrow<[MaybeUninit<T>]> for SegmMut<B, T, R>
where
    B: BorrowMut<[MaybeUninit<T>]>,
    R: TrReclaim<MaybeUninit<T>>,
{
    #[inline]
    fn borrow(&self) -> &[MaybeUninit<T>] {
        self.as_slice()
    }
}

impl<B, T, R> BorrowMut<[MaybeUninit<T>]> for SegmMut<B, T, R>
where
    B: BorrowMut<[MaybeUninit<T>]>,
    R: TrReclaim<MaybeUninit<T>>,
{
    #[inline]
    fn borrow_mut(&mut self) -> &mut [MaybeUninit<T>] {
        self.as_slice_mut()
    }
}

impl<B, T, R> AsRef<[MaybeUninit<T>]> for SegmMut<B, T, R>
where
    B: BorrowMut<[MaybeUninit<T>]>,
    R: TrReclaim<MaybeUninit<T>>,
{
    #[inline]
    fn as_ref(&self) -> &[MaybeUninit<T>] {
        self.as_slice()
    }
}

impl<B, T, R> AsMut<[MaybeUninit<T>]> for SegmMut<B, T, R>
where
    B: BorrowMut<[MaybeUninit<T>]>,
    R: TrReclaim<MaybeUninit<T>>,
{
    #[inline]
    fn as_mut(&mut self) -> &mut [MaybeUninit<T>] {
        self.as_slice_mut()
    }
}

impl<B, T, R> Deref for SegmMut<B, T, R>
where
    B: BorrowMut<[MaybeUninit<T>]>,
    R: TrReclaim<MaybeUninit<T>>,
{
    type Target = [MaybeUninit<T>];

    #[inline]
    fn deref(&self) -> &[MaybeUninit<T>] {
        self.as_slice()
    }
}

impl<B, T, R> DerefMut for SegmMut<B, T, R>
where
    B: BorrowMut<[MaybeUninit<T>]>,
    R: TrReclaim<MaybeUninit<T>>,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut [MaybeUninit<T>] {
        self.as_slice_mut()
    }
}

impl<B, T, R> TrBuffSegmView for SegmMut<B, T, R>
where
    B: BorrowMut<[MaybeUninit<T>]>,
    R: TrReclaim<MaybeUninit<T>>,
{
    type Item = MaybeUninit<T>;

    #[inline]
    fn capacity(&self) -> usize {
        SegmMut::capacity(self)
    }

    #[inline]
    fn is_empty(&self) -> bool {
        SegmMut::is_empty(self)
    }

    #[inline]
    fn len(&self) -> usize {
        SegmMut::len(self)
    }

    fn iter_ptr(&self) -> impl Iterator<Item = *const Self::Item> {
        self.slice_mut_
            .borrow()
            .iter()
            .map(|x| x as *const MaybeUninit<T>)
    }
}

impl<B, T, R> SegmMut<B, T, R>
where
    B: BorrowMut<[MaybeUninit<T>]>,
    T: Copy,
    R: TrReclaim<MaybeUninit<T>>,
{
    /// A convenient wrapper around [copy_from_slice](<[T]>::copy_from_slice)
    pub fn copy_from_slice(&mut self, src: &[T]) {
        let slice = unsafe {
            let p = self.deref_mut() as *mut [MaybeUninit<T>] as *mut [T];
            &mut *p
        };
        slice.copy_from_slice(src);
    }
}

impl<B, T, R> TrBuffSegmMut<T> for SegmMut<B, T, R>
where
    B: BorrowMut<[MaybeUninit<T>]>,
    R: TrReclaim<MaybeUninit<T>>,
{
    #[inline]
    fn take_segm_mut(&mut self, length: usize) -> impl TrBuffSegmMut<T> {
        SegmMut::take_segm_mut(self, length)
    }
}

#[cfg(test)]
mod tests_ {
    use core::mem::MaybeUninit;

    use super::SegmMut;

    #[test]
    fn segm_len_should_eq_as_slice_len() {
        const ARR_SIZE: usize = 64;
        let mut buff = [MaybeUninit::zeroed(); ARR_SIZE];
        for (u, x) in buff.iter_mut().enumerate() {
            let _ = *x.write(u);
        }
        let mut segm = SegmMut::no_reclaim(buff.as_mut_slice());
        let slice = segm.as_slice();
        assert_eq!(segm.len(), ARR_SIZE);
        assert_eq!(slice.len(), segm.len());

        const SLICE_LEN: usize = ARR_SIZE >> 1;
        let taken_slice = segm.take_segm_mut(SLICE_LEN);
        for (u, x) in taken_slice.as_ref().iter().enumerate() {
            assert_eq!(unsafe { x.assume_init_read() }, u)
        }
        drop(taken_slice);
        assert_eq!(segm.len(), ARR_SIZE - SLICE_LEN);

        let taken_slice = segm.take_segm_mut(ARR_SIZE);
        assert_eq!(taken_slice.len(), ARR_SIZE- SLICE_LEN);
        for (u, x) in taken_slice.as_ref().iter().enumerate() {
            assert_eq!(unsafe { x.assume_init_read() }, u + SLICE_LEN)
        }
        drop(taken_slice);
        assert_eq!(segm.len(), 0);
    }
}
