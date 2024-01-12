#![doc = include_str!("../README.md")]
//!
#![cfg_attr(feature = "internal-doc-images",
cfg_attr(all(),
doc = ::embed_doc_image::embed_image!("memalign-histograms", "images/memalign-histograms.svg")))]
#![cfg_attr(feature = "internal-doc-images",
cfg_attr(all(),
doc = ::embed_doc_image::embed_image!("malloc-histograms", "images/malloc-histograms.svg")))]
#![cfg_attr(
    not(feature = "internal-doc-images"),
    doc = "**Doc images not enabled**. Compile with feature `doc-images` and Rust version >= 1.54 \
           to enable."
)]
//!

#![cfg_attr(not(any(test, fuzzing)), no_std)]
#![deny(missing_docs)]

use crate::block::Anchor;
pub use crate::block::Block;
use crate::header::Header;

mod block;
mod consts;
mod header;
#[cfg(any(test, fuzzing))]
mod helpers;
mod mapping;
mod ops;

#[cfg(fuzzing)]
pub use crate::helpers::Memory;

/// The Two-Level Segregated Fit (TLSF) memory allocator
pub struct Tlsf<'a, const FLL: usize> {
    anchor: Option<Anchor<'a>>,
    header: Header<FLL>,
}

impl<'a, const FLL: usize> Tlsf<'a, FLL> {
    /// Creates a new TLSF allocator with no associated memory
    ///
    /// NOTE: Before you call [`Tlsf::memalign`], you must initialize the allocator with [`Tlsf::initialize`]
    pub const fn empty() -> Self {
        Self {
            header: Header::new(),
            anchor: None,
        }
    }
}

#[cfg(not(miri))]
#[cfg(test)]
mod tests {
    use core::alloc::Layout;
    use core::mem;
    use core::ptr::NonNull;

    use helpers::Memory;
    use rand::seq::SliceRandom;
    use rand::{RngCore, SeedableRng};
    use rand_xorshift::XorShiftRng;

    use super::*;

    #[test]
    fn stress() {
        const FLL: usize = 2;

        let mut tlsf = Tlsf::<{ FLL }>::empty();
        let mut memory = Memory::new();
        tlsf.initialize(memory.bytes());

        assert_eq!(4, tlsf.free_blocks().len());
        let total_size_before = tlsf
            .free_blocks()
            .iter()
            .map(|block| block.total_size())
            .sum::<usize>();

        // NOTE still not deterministic because the alignment of `memory` affects the outcome and
        // that's decided by the OS
        let seed = 244905504285122192707088002030163722993_u128;
        let mut rng = XorShiftRng::from_seed(seed.to_le_bytes());
        let mut allocs = vec![];
        let min_layout = Layout::new::<u8>();
        let mut allocated = 0;
        loop {
            let size = (rng.next_u32() as usize) % Header::<{ FLL }>::MAX_ALLOC_SIZE as usize;
            let align = 1 << (rng.next_u32() as u8 % 6);
            let layout = Layout::from_size_align(size, align).unwrap();

            let mut res = tlsf.memalign(layout);
            if res.is_none() {
                // the `alloc` can fail due to alignment requirements; do the smallest allocation
                // possible instead which should always work as long as we are not OOM
                res = tlsf.memalign(min_layout);
            }

            if let Some(alloc) = res {
                allocated += mem::size_of_val(alloc);

                alloc.iter_mut().for_each(|mu| {
                    mu.write(!0);
                });
                allocs.push(alloc);
            } else {
                break;
            }

            // sanity check that the "statistics" API matches reality
            let mut count = 0;
            let mut used = 0;
            for block in tlsf.blocks() {
                if block.is_used() {
                    used += usize::from(block.usable_size());
                    count += 1;
                }
            }
            assert_eq!(allocs.len(), count);
            assert_eq!(allocated, used);
        }

        assert_eq!(0, tlsf.free_blocks().len());

        allocs.shuffle(&mut rng);
        while let Some(alloc) = allocs.pop() {
            unsafe { tlsf.free(NonNull::from(alloc).cast()) }
        }

        let total_size_after = tlsf
            .free_blocks()
            .iter()
            .map(|block| block.total_size())
            .sum::<usize>();

        assert_eq!(total_size_before, total_size_after);
    }
}
