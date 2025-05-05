use core::ptr::NonNull;

/// To receive the message about the amount of consumed buffer.
pub trait TrForward {
    fn forward(&mut self, consumed: usize);
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NoForward;

impl NoForward {
    pub const fn new() -> Self {
        NoForward
    }

    pub fn forward(&mut self, consumed: usize) {
        let _ = consumed;
    }
}

impl TrForward for NoForward {
    #[inline]
    fn forward(&mut self, consumed: usize) {
        NoForward::forward(self, consumed);
    }
}

pub struct IncrConsumed(NonNull<usize>);

impl IncrConsumed {
    /// ## Safety
    /// - The consumed counter must outlive the instance creating
    pub const unsafe fn new(consumed: &mut usize) -> Self {
        IncrConsumed(unsafe { NonNull::new_unchecked(consumed) })
    }

    pub fn forward(&mut self, consumed: usize) {
        unsafe {
            let c_mut = self.0.as_mut();
            *c_mut += consumed;
        }
    }
}

impl TrForward for IncrConsumed {
    #[inline]
    fn forward(&mut self, consumed: usize) {
        IncrConsumed::forward(self, consumed);
    }
}
