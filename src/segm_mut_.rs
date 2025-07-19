use core::{
    borrow::{Borrow, BorrowMut},
    iter::IntoIterator,
    marker::PhantomData,
    mem::MaybeUninit,
    ops::{Deref, DerefMut, Try},
    ptr::NonNull,
};

use abs_buff::{
    x_deps::abs_iter, Demand, TrBuffSegmMut, TrBuffSegmView, TrOutput
};
use abs_iter::{TrAsSlice, TrAsSliceMut, TrItemsMutView, TrItemsRefView};

use super::forward_::{NoForward, IncrConsumed};

/// Wraps a single mut slice into a mut buffer (`TrBuffSegmMut`). 
#[repr(C)]
pub struct SegmMut<B, T, F>
where
    B: BorrowMut<[MaybeUninit<T>]>,
    F: FnOnce(usize),
{
    _using_t_: PhantomData<[T]>,
    consumed_: usize,
    forward_: Option<F>,
    slice_mut_: B,
}

impl<B, T, F> SegmMut<B, T, F>
where
    B: BorrowMut<[MaybeUninit<T>]>,
    F: FnOnce(usize),
{
    /// Create by borrowing a mut slice from an implicit source. And the items
    /// of this mut slice will be returned back to or moved out from the source 
    /// by `reclaim`.
    ///
    /// ## Safety
    /// 
    /// - `reclaim` should be capable of semantically move item out when this
    ///     slice `into_iter`
    pub const fn new(slice_mut: B, forward: Option<F>) -> Self {
        SegmMut {
            _using_t_: PhantomData,
            consumed_: 0usize,
            forward_: forward,
            slice_mut_: slice_mut,
        }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.slice_mut_.borrow().len()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.as_slice().len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.capacity() == self.consumed_
    }

    pub fn as_slice(&self) -> &[MaybeUninit<T>] {
        let slice: &[MaybeUninit<T>] = self.slice_mut_.borrow();
        debug_assert!(self.consumed_ <= slice.len());
        &slice[self.consumed_..]
    }

    pub fn iter_slices<'a>(
        &'a self,
    ) -> Option<&'a [MaybeUninit<T>]>
    where
        T: 'a,
    {
        if self.is_empty() {
            Option::None
        } else {
            let slice: &[MaybeUninit<T>] = self.slice_mut_.borrow();
            Option::Some(slice)
        }
    }

    pub fn as_slice_mut(&mut self) -> &mut [MaybeUninit<T>] {
        let slice: &mut [MaybeUninit<T>] = self.slice_mut_.borrow_mut();
        &mut slice[self.consumed_..]
    }

    pub fn iter_slices_mut<'a>(
        &'a mut self,
    ) -> Option<&'a mut [MaybeUninit<T>]>
    where
        T: 'a,
    {
        if self.is_empty() {
            Option::None
        } else {
            let slice_mut: &mut [MaybeUninit<T>] = self.slice_mut_.borrow_mut();
            let slice = &mut slice_mut[self.consumed_..];
            Option::Some(slice)
        }
    }

    pub fn take_segm_mut<'f>(
        &'f mut self,
        length: Demand<usize>,
    ) -> Option<SegmMut<&'f mut [MaybeUninit<T>], T, IncrConsumed>> {
        let demand = length.narrow_from_most(self.len())?;
        let size = demand.most().cloned()?;
        unsafe {
            let mut this_ptr = NonNull::new_unchecked(self);
            let slice = this_ptr.as_mut().as_slice_mut();
            let slice = &mut slice[..size];
            let forward = IncrConsumed::new(&mut self.consumed_);
            Option::Some(SegmMut::new(slice, Option::Some(forward)))
        }
    }

    pub fn as_output(&mut self) -> impl TrOutput<T> {
        TrBuffSegmMut::as_output(self)
    }
}

impl<B, T> SegmMut<B, T, NoForward>
where
    B: BorrowMut<[MaybeUninit<T>]>,
{
    /// Create by borrowing a slice from an implicit source. 
    ///
    /// ## Safety
    /// - `slice` must be managed by the source buffer;
    pub const fn no_forward(slice_mut: B) -> Self {
        SegmMut::new(slice_mut, Option::None)
    }
}

impl<B, T, F> Drop for SegmMut<B, T, F>
where
    B: BorrowMut<[MaybeUninit<T>]>,
    F: FnOnce(usize),
{
    fn drop(&mut self) {
        if let Option::Some(f) = self.forward_.take() {
            f(self.capacity());
        }
    }
}

impl<B, T, F> Borrow<[MaybeUninit<T>]> for SegmMut<B, T, F>
where
    B: BorrowMut<[MaybeUninit<T>]>,
    F: FnOnce(usize),
{
    #[inline]
    fn borrow(&self) -> &[MaybeUninit<T>] {
        self.as_slice()
    }
}

impl<B, T, F> BorrowMut<[MaybeUninit<T>]> for SegmMut<B, T, F>
where
    B: BorrowMut<[MaybeUninit<T>]>,
    F: FnOnce(usize),
{
    #[inline]
    fn borrow_mut(&mut self) -> &mut [MaybeUninit<T>] {
        self.as_slice_mut()
    }
}

impl<B, T, F> AsRef<[MaybeUninit<T>]> for SegmMut<B, T, F>
where
    B: BorrowMut<[MaybeUninit<T>]>,
    F: FnOnce(usize),
{
    #[inline]
    fn as_ref(&self) -> &[MaybeUninit<T>] {
        self.as_slice()
    }
}

impl<B, T, F> AsMut<[MaybeUninit<T>]> for SegmMut<B, T, F>
where
    B: BorrowMut<[MaybeUninit<T>]>,
    F: FnOnce(usize),
{
    #[inline]
    fn as_mut(&mut self) -> &mut [MaybeUninit<T>] {
        self.as_slice_mut()
    }
}

impl<B, T, F> Deref for SegmMut<B, T, F>
where
    B: BorrowMut<[MaybeUninit<T>]>,
    F: FnOnce(usize),
{
    type Target = [MaybeUninit<T>];

    #[inline]
    fn deref(&self) -> &[MaybeUninit<T>] {
        self.as_slice()
    }
}

impl<B, T, F> DerefMut for SegmMut<B, T, F>
where
    B: BorrowMut<[MaybeUninit<T>]>,
    F: FnOnce(usize),
{
    #[inline]
    fn deref_mut(&mut self) -> &mut [MaybeUninit<T>] {
        self.as_slice_mut()
    }
}

impl<B, T, F> TrAsSlice for SegmMut<B, T, F>
where
    B: BorrowMut<[MaybeUninit<T>]>,
    F: FnOnce(usize),
{
    type Elem = MaybeUninit<T>;
}

impl<B, T, F> TrAsSliceMut for SegmMut<B, T, F>
where
    B: BorrowMut<[MaybeUninit<T>]>,
    F: FnOnce(usize),
{}

impl<B, T, F> TrItemsRefView for SegmMut<B, T, F>
where
    B: BorrowMut<[MaybeUninit<T>]>,
    F: FnOnce(usize),
{
    type Item = MaybeUninit<T>;

    fn items_ref_view(&self) -> impl IntoIterator<Item: Borrow<Self::Item>> {
        self.as_slice().iter()
    }
}

impl<B, T, F> TrItemsMutView for SegmMut<B, T, F>
where
    B: BorrowMut<[MaybeUninit<T>]>,
    F: FnOnce(usize),
{
    type Item = MaybeUninit<T>;

    fn items_mut_view(
        &mut self,
    ) -> impl IntoIterator<Item: BorrowMut<Self::Item>> {
        self.as_slice_mut().iter_mut()
    }
}

impl<B, T, F> TrBuffSegmView for SegmMut<B, T, F>
where
    B: BorrowMut<[MaybeUninit<T>]>,
    F: FnOnce(usize),
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
    fn iter_slices(
        &self,
    ) -> impl IntoIterator<Item: TrAsSlice<Elem = Self::Item>> {
        SegmMut::iter_slices(self)
    }
}

impl<B, T, F> TrBuffSegmMut<T> for SegmMut<B, T, F>
where
    B: BorrowMut<[MaybeUninit<T>]>,
    F: FnOnce(usize),
{
    #[inline]
    fn iter_slices_mut<'a>(
        &'a mut self,
    ) -> impl IntoIterator<Item: TrAsSliceMut<Elem = MaybeUninit<T>>>
    where
        T: 'a,
    {
        SegmMut::iter_slices_mut(self)
    }

    #[inline]
    fn take_segm_mut(
        &mut self, 
        length: Demand<usize>,
    ) -> impl Try<Output: TrBuffSegmMut<T>> {
        SegmMut::take_segm_mut(self, length)
    }
}

#[cfg(test)]
mod tests_ {
    use core::mem::MaybeUninit;

    use super::{Demand, SegmMut};

    #[test]
    fn segm_len_should_eq_as_slice_len() {
        const ARR_SIZE: usize = 64;
        let mut buff = [MaybeUninit::zeroed(); ARR_SIZE];
        for (u, x) in buff.iter_mut().enumerate() {
            let _ = *x.write(u);
        }
        let mut segm = SegmMut::no_forward(buff.as_mut_slice());
        let slice = segm.as_slice();
        assert_eq!(segm.len(), ARR_SIZE);
        assert_eq!(slice.len(), segm.len());

        const SLICE_LEN: usize = ARR_SIZE >> 1;
        let Option::Some(taken_slice) = segm.take_segm_mut(Demand::exactly(SLICE_LEN)) else {
            panic!()
        };
        for (u, x) in taken_slice.as_ref().iter().enumerate() {
            assert_eq!(unsafe { x.assume_init_read() }, u)
        }
        drop(taken_slice);
        assert_eq!(segm.len(), ARR_SIZE - SLICE_LEN);

        let Option::Some(taken_slice) = segm.take_segm_mut(Demand::at_most(ARR_SIZE)) else {
            panic!()
        };
        assert_eq!(taken_slice.len(), ARR_SIZE- SLICE_LEN);
        for (u, x) in taken_slice.as_ref().iter().enumerate() {
            assert_eq!(unsafe { x.assume_init_read() }, u + SLICE_LEN)
        }
        drop(taken_slice);
        assert_eq!(segm.len(), 0);
    }
}
