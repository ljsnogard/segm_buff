use core::{
    borrow::Borrow,
    iter::{IntoIterator, Iterator},
    marker::PhantomData,
};

use abs_buff::{TrBuffSegmRef, TrBuffSegmView};
use abs_iter::{TrArray, TrMutSliceLike, TrSliceLike};

use crate::{IncrConsumed, SegmMut, SegmRef, TrForward};

pub struct SegmSeq<B, T, F>
where
    B: TrSliceLike,
    B::Elem: TrSliceLike<Elem = T>,
    T: Sized,
    F: TrForward,
{
    _use_t_: PhantomData<[T]>,
    source_: B,
    index_: usize,
    offset_: usize,
    capacity_: usize,
    forward_: F,
}

impl<B, T, F> SegmSeq<B, T, F>
where
    B: TrSliceLike,
    B::Elem: TrSliceLike<Elem = T>,
    T: Sized,
    F: TrForward,
{
    /// ## Safety
    ///
    /// - Slices in source must not overlap with one another
    pub unsafe fn new_unchecked(source: B, forward: F) -> Self {
        let s_arr= source.as_ref();
        let capacity = s_arr
            .iter()
            .map(|s| s.as_ref().len())
            .sum();
        SegmSeq {
            _use_t_: PhantomData,
            source_: source,
            index_: 0usize,
            offset_: 0usize,
            capacity_: capacity,
            forward_: forward,
        }
    }

    pub const fn capacity(&self) -> usize {
        self.capacity_
    }

    pub fn is_empty(&self) -> bool {
        let s_arr= self.source_.as_ref();
        if s_arr.is_empty() {
            return true;
        }
        if self.index_ == s_arr.len() - 1 {
            self.offset_ == s_arr[self.index_].as_ref().len()
        } else {
            false
        }
    }

    pub fn len(&self) -> usize {
        if self.capacity_ == 0 {
            return self.capacity_;
        }
        let s_arr = self.source_.as_ref();
        let mut consumed  = 0usize;
        for i in 0usize .. self.index_ {
            let s = &s_arr[i];
            if i < self.index_ {
                consumed += s.as_ref().len();
            } else {
                consumed += self.offset_;
            }
        };
        debug_assert!(self.capacity_ >= consumed);
        self.capacity_ - consumed
    }

    pub const fn iter_slices(&self) -> SegmSeqSlicesIterator<'_, B, T, F> {
        SegmSeqSlicesIterator::new(self)
    }
}

impl<B, T, F> TrBuffSegmView for SegmSeq<B, T, F>
where
    B: TrSliceLike,
    B::Elem: TrSliceLike<Elem = T>,
    T: Sized,
    F: TrForward,
{
    type Item = T;

    #[inline]
    fn capacity(&self) -> usize {
        SegmSeq::capacity(self)
    }

    #[inline]
    fn is_empty(&self) -> bool {
        SegmSeq::is_empty(self)
    }

    #[inline]
    fn len(&self) -> usize {
        SegmSeq::len(self)
    }

    fn iter_ptr(&self) -> impl Iterator<Item = *const Self::Item> {
        self.iter_slices()
            .flat_map(|s| s.into_iter())
            .map(|x| x as *const Self::Item)
    }
}

impl<B, T, F> TrBuffSegmRef<T> for SegmSeq<B, T, F>
where
    B: TrSliceLike,
    B::Elem: TrSliceLike<Elem = T>,
    T: Sized,
    F: TrForward,
{
    type Slice<'a> = &'a [T]
    where
        T: 'a,
        Self: 'a;

    type Segm<'a> = SegmSeq<&'a [Self::Slice<'a>], T, IncrConsumed>
    where
        T: 'a,
        Self: 'a;

    fn iter_slices<'a>(&'a mut self) -> impl IntoIterator<Item = Self::Slice<'a>>
    where
        T: 'a
    {
        SegmSeq::iter_slices(self)
    }

    fn take_segm_ref<'a>(
        &'a mut self,
        length: abs_buff::Demand<usize>,
    ) -> Option<Self::Segm<'a>> {
        todo!()
    }
}

pub struct SegmSeqSlicesIterator<'a, B, T, F>
where
    B: TrSliceLike,
    B::Elem: TrSliceLike<Elem = T>,
    T: Sized,
    F: TrForward,
{
    seq_: &'a SegmSeq<B, T, F>,
    index_: usize,
}

impl<'a, B, T, F> SegmSeqSlicesIterator<'a, B, T, F>
where
    B: TrSliceLike,
    B::Elem: TrSliceLike<Elem = T>,
    T: Sized,
    F: TrForward,
{
    pub const fn new(seq: &'a SegmSeq<B, T, F>) -> Self {
        SegmSeqSlicesIterator {
            seq_: seq,
            index_: seq.index_,
        }
    }
}

impl<'a, B, T, F> Iterator for SegmSeqSlicesIterator<'a, B, T, F>
where
    B: TrSliceLike,
    B::Elem: TrSliceLike<Elem = T>,
    T: Sized,
    F: TrForward,
{
    type Item = &'a [T];

    fn next(&mut self) -> Option<Self::Item> {
        if self.index_ >= self.seq_.len() {
            return Option::None;
        }
        let a = self.seq_.source_.as_ref();
        let b = a[self.index_].borrow();
        if self.index_ == self.seq_.index_ {
            Option::Some(&b[self.seq_.offset_..])
        } else {
            Option::Some(b)
        }
    }
}

