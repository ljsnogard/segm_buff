use core::ptr::NonNull;

use abs_buff::TrBuffSegmView;

pub trait TrReclaim<T>: Sized {
    fn reclaim<S: TrBuffSegmView<Item = T>>(&mut self, s: &mut S);
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NoReclaim;

impl<T> TrReclaim<T> for NoReclaim {
    #[inline]
    fn reclaim<S: TrBuffSegmView<Item = T>>(&mut self, s: &mut S) {
        let _ = s;
    }
}

pub struct BuffSegmReclaim(NonNull<usize>);

impl BuffSegmReclaim {
    #[inline]
    pub(super) const unsafe fn new(offset_ptr: NonNull<usize>) -> Self {
        BuffSegmReclaim(offset_ptr)
    }
}

impl<T> TrReclaim<T> for BuffSegmReclaim {
    #[inline]
    fn reclaim<S: TrBuffSegmView<Item = T>>(&mut self, s: &mut S) {
        let offset_mut = unsafe { self.0.as_mut() };
        *offset_mut += s.len()
    }
}
