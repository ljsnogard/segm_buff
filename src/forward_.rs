use core::ptr::NonNull;

#[derive(Clone, Copy, Debug, Default)]
pub struct NoForward;

impl NoForward {
    pub const fn new() -> Self {
        NoForward
    }
}

impl FnOnce<(usize,)> for NoForward {
    type Output = ();

    extern "rust-call" fn call_once(self, args: (usize,)) -> Self::Output {
        let _ = args;
    }
}

pub struct IncrConsumed(NonNull<usize>);

impl IncrConsumed {
    /// ## Safety
    /// - The consumed counter must outlive the instance creating
    pub const unsafe fn new(consumed: &mut usize) -> Self {
        IncrConsumed(unsafe { NonNull::new_unchecked(consumed) })
    }

    fn forward(mut self, consumed: usize) {
        unsafe {
            let c_mut = self.0.as_mut();
            *c_mut += consumed;
        }
    }
}

impl FnOnce<(usize,)> for IncrConsumed {
    type Output = ();

    extern "rust-call" fn call_once(self, args: (usize,)) -> Self::Output {
        IncrConsumed::forward(self, args.0);
    }
}
