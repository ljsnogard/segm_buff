#![no_std]

// Required to implement FnOnce
#![feature(unboxed_closures)]
#![feature(fn_traits)]

#![feature(try_trait_v2)]

// We always pull in `std` during tests, because it's just easier
// to write tests when you can assume you're on a capable platform
#[cfg(test)]
extern crate std;

mod forward_;
mod segm_mut_;
mod segm_ref_;

pub use forward_::{IncrConsumed, NoForward};
pub use segm_mut_::SegmMut;
pub use segm_ref_::SegmRef;

pub mod x_deps {
    pub use abs_buff;
    pub use abs_buff::x_deps::{abs_iter, abs_sync};
}
