use core::alloc::Layout;
use core::mem::MaybeUninit;

use crate::block::{Anchor, FreeBlock};
use crate::header::Header;
use crate::ops::util;
use crate::{consts, Tlsf};

impl<'a, const FLL: usize> Tlsf<'a, FLL> {
    /// Allocates a memory block compatible with the specified `layout`
    ///
    /// This function returns `None` when `layout` has a `size` equal to zero and when there's
    /// insufficient free memory to satisfy the request
    pub fn memalign(&mut self, layout: Layout) -> Option<&'a mut [MaybeUninit<u32>]> {
        let anchor = self.anchor?;
        unsafe { self.header.memalign(anchor, layout) }
    }
}

impl<const FLL: usize> Header<FLL> {
    unsafe fn memalign<'a>(
        &mut self,
        anchor: Anchor<'a>,
        layout: Layout,
    ) -> Option<&'a mut [MaybeUninit<u32>]> {
        if layout.size() == 0 {
            return None;
        }

        let size = layout.size().try_into().ok()?;
        let align = layout.align().try_into().ok()?;

        let size = util::round_up_block_size(size)?;
        let worst_case_size = worst_case_size(size, align)?;
        let mut block = self.pop(anchor, worst_case_size)?;

        block = self.adjust_free_block_alignment(anchor, block, align);

        block = self.adjust_free_block_size(anchor, block, size);

        #[cfg(any(fuzzing, test))]
        debug_assert!(block.usable_size() >= size);

        let alloc = block.into_used(anchor);

        #[cfg(any(fuzzing, test))]
        debug_assert_eq!(0, alloc.as_ptr() as usize % align as usize);

        Some(alloc)
    }

    unsafe fn adjust_free_block_alignment<'a>(
        &mut self,
        anchor: Anchor<'a>,
        block: FreeBlock<'a>,
        align: u16,
    ) -> FreeBlock<'a> {
        let align = usize::from(align);
        let address = block.body_ptr().as_ptr() as usize;
        let rem = unsafe { address.checked_rem(align).unwrap_unchecked() };

        if rem != 0 {
            let mut at = align.wrapping_sub(rem);
            if at < FreeBlock::HEADER_SIZE.into() {
                #[cfg(all(test, not(miri)))]
                cov_mark::hit!(alloc_adjust_align_is_lt_free_header_size);

                at += align;
            } else {
                #[cfg(all(test, not(miri)))]
                cov_mark::hit!(alloc_adjust_align_is_gte_free_header_size);
            }
            let new = unsafe { anchor.split(&block, at) };
            unsafe { self.push(anchor, block) }
            new
        } else {
            block
        }
    }
}

// in the worst case scenario the block will be already `align`-byte aligned
// the alignment of usable part of the block will be off by 4 bytes
// (UsedBlock::HEADER_SIZE)
// the block can be split in 2 but the first block will have a total size of at
// least 8 bytes (FreeBlock::HEADER_SIZE)
fn worst_case_size(size: u16, align: u16) -> Option<u16> {
    if align <= consts::BLOCK_ALIGN.into() {
        Some(size)
    } else {
        align.checked_add(4)?.checked_add(size)
    }
}

#[cfg(test)]
mod tests {
    use core::mem::MaybeUninit;

    use super::*;
    use crate::block::UsedBlock;

    #[test]
    fn worst_case_size() {
        // implementation and tests are hard-coded to these values
        assert_eq!(4, UsedBlock::HEADER_SIZE);
        assert_eq!(8, FreeBlock::HEADER_SIZE);

        // repr(packed)
        assert_eq!(Some(1), super::worst_case_size(1, 1));

        assert_eq!(Some(2), super::worst_case_size(2, 2));
        assert_eq!(Some(4), super::worst_case_size(4, 2));

        assert_eq!(Some(4), super::worst_case_size(4, 4));
        assert_eq!(Some(8), super::worst_case_size(8, 4));

        assert_eq!(Some(16), super::worst_case_size(4, 8));
        assert_eq!(Some(20), super::worst_case_size(8, 8));
        assert_eq!(Some(28), super::worst_case_size(16, 8));

        assert_eq!(Some(32), super::worst_case_size(12, 16));
        assert_eq!(Some(36), super::worst_case_size(16, 16));
        assert_eq!(Some(40), super::worst_case_size(20, 16));
    }

    #[test]
    fn no_split() {
        let mut tlsf = Tlsf::<1>::empty();
        let mut memory = [MaybeUninit::uninit(); 3];
        tlsf.initialize(&mut memory);

        let [free] = tlsf.free_blocks().try_into().unwrap();
        assert_eq!(4, free.usable_size());

        let alloc = tlsf.memalign(Layout::new::<u8>()).unwrap();
        assert_eq!(1, alloc.len());
        assert!(tlsf.free_blocks().is_empty());
    }

    #[test]
    fn split_size() {
        let mut tlsf = Tlsf::<1>::empty();
        let mut memory = [MaybeUninit::uninit(); 5];
        tlsf.initialize(&mut memory);

        let [free] = tlsf.free_blocks().try_into().unwrap();
        assert_eq!(16, free.total_size());

        let alloc = {
            #[cfg(not(miri))]
            cov_mark::check!(alloc_adjust_size);

            tlsf.memalign(Layout::new::<u8>())
        }
        .unwrap();
        assert_eq!(1, alloc.len());

        let [remaining] = tlsf.free_blocks().try_into().unwrap();
        assert_eq!(8, remaining.total_size());
    }

    #[test]
    fn split_align_small() {
        #[repr(align(8))]
        struct Aligned<T>(T);

        let mut tlsf = Tlsf::<1>::empty();
        let mut memory = Aligned([MaybeUninit::uninit(); 8]);
        tlsf.initialize(&mut memory.0[1..]);

        let [free] = tlsf.free_blocks().try_into().unwrap();
        assert_eq!(24, free.total_size());

        let alloc = {
            #[cfg(not(miri))]
            cov_mark::check!(alloc_adjust_align_is_lt_free_header_size);

            tlsf.memalign(Layout::new::<u64>())
        }
        .unwrap();
        assert_eq!(2, alloc.len());

        let [free] = tlsf.free_blocks().try_into().unwrap();
        assert_eq!(12, free.total_size());
    }

    #[test]
    fn split_align_large() {
        #[repr(align(16))]
        struct Aligned<T>(T);

        let mut tlsf = Tlsf::<1>::empty();
        let mut memory = Aligned([MaybeUninit::uninit(); 11]);
        tlsf.initialize(&mut memory.0[..]);

        let [free] = tlsf.free_blocks().try_into().unwrap();
        assert_eq!(40, free.total_size());

        let alloc = {
            #[cfg(not(miri))]
            cov_mark::check!(alloc_adjust_align_is_gte_free_header_size);

            tlsf.memalign(Layout::new::<Aligned<u8>>())
        }
        .unwrap();
        assert_eq!(4, alloc.len());

        let [align_adjust, size_adjust] = tlsf.free_blocks().try_into().unwrap();
        assert_eq!(8, align_adjust.total_size());
        assert_eq!(12, size_adjust.total_size());
    }

    #[test]
    fn odd_layout() {
        let mut tlsf = Tlsf::<1>::empty();
        let mut memory = [MaybeUninit::uninit(); 6];
        tlsf.initialize(&mut memory);

        let [free] = tlsf.free_blocks().try_into().unwrap();
        assert_eq!(20, free.total_size());

        let layout = Layout::from_size_align(1, 8).unwrap();
        let alloc = tlsf.memalign(layout).unwrap();
        assert_eq!(1, alloc.len());

        let [free] = tlsf.free_blocks().try_into().unwrap();
        assert_eq!(12, free.total_size());
    }

    #[cfg(not(miri))] // slow
    #[test]
    fn stress() {
        let mut memory = vec![MaybeUninit::<u32>::uninit(); 65 * 1024].into_boxed_slice();
        let memory = &mut memory[..];
        let mut tlsf = Tlsf::<1>::empty();
        tlsf.initialize(memory);

        let mut count = 0;
        while let Some(alloc) = tlsf.memalign(Layout::new::<u8>()) {
            count += 1;
            assert!(!alloc.is_empty());
            assert!(alloc.len() <= 2);
            alloc.iter_mut().for_each(|mu| {
                mu.write(!0);
            });
        }

        assert!(tlsf.free_blocks().is_empty());
        assert_eq!(count, tlsf.blocks().count());
    }
}
