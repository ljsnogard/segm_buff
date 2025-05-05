#![no_std]

// We always pull in `std` during tests, because it's just easier
// to write tests when you can assume you're on a capable platform
#[cfg(test)]
extern crate std;

mod forward_;
mod segm_mut_;
mod segm_ref_;
mod segm_seq_;

pub use forward_::{NoForward, IncrConsumed, TrForward};
pub use segm_mut_::SegmMut;
pub use segm_ref_::SegmRef;
pub use segm_seq_::SegmSeq;

pub mod x_deps {
    pub use abs_buff;
    pub use abs_iter;

    pub use abs_buff::x_deps::abs_sync;
}