use core::mem::{self, MaybeUninit};

use crate::block::{Anchor, FreeBlock, Offset, UsedBlock};
use crate::{consts, Tlsf};

impl<'a, const FLL: usize> Tlsf<'a, FLL> {
    /// Gives the allocator a chunk of memory to manage
    ///
    /// The allocator MAY only be initialized once. Subsequent invocations of this method will be
    /// ignored.
    pub fn initialize(&mut self, memory: &'a mut [MaybeUninit<u32>]) {
        if self.anchor.is_some() {
            #[cfg(all(test, not(miri)))]
            cov_mark::hit!(initialized_twice);

            return;
        }

        let mut total_size = memory
            .len()
            .saturating_mul(mem::size_of::<u32>())
            .min(consts::MAX_POOL_SIZE);

        // skip the first `BLOCK_ALIGN` bytes to ensure `Offset` is a non-zero value
        let mut uncompressed_offset = usize::from(consts::BLOCK_ALIGN);

        let free_header_size = usize::from(FreeBlock::HEADER_SIZE);
        if total_size < uncompressed_offset + free_header_size {
            return;
        }

        total_size -= uncompressed_offset;

        let anchor = Anchor::new(memory);
        let used_header_size = usize::from(UsedBlock::HEADER_SIZE);

        let mut prev_phys_block = None;
        while total_size >= free_header_size {
            let usable_size = (total_size - used_header_size).try_into().unwrap_or({
                #[cfg(all(test, not(miri)))]
                cov_mark::hit!(initialize_max_usable_size);

                consts::MAX_USABLE_SIZE
            });

            let offset = unsafe { Offset::compress(uncompressed_offset) };

            let step = usize::from(usable_size) + used_header_size;
            // due to the `MAX_POOL_SIZE` cap we know this won't cause an overflow (see tests)
            // LLVM can't figure that out and keeps the overflow checks so we help it here with
            // the `wrapping_` operations
            #[cfg(not(any(fuzzing, test)))]
            {
                total_size = total_size.wrapping_sub(step);
                uncompressed_offset = uncompressed_offset.wrapping_sub(step);
            }

            // the version with overflow checks is used to detect bugs in the `stress` test
            #[cfg(any(fuzzing, test))]
            {
                total_size -= step;
                uncompressed_offset += step;
            }

            let is_last_phys_block = total_size < free_header_size;
            let block = unsafe {
                anchor.create_free_block(offset, usable_size, is_last_phys_block, prev_phys_block)
            };
            prev_phys_block = Some(offset);

            unsafe { self.header.push(anchor, block) }
        }

        self.anchor = Some(anchor);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_anchor() {
        let mut tlsf = Tlsf::<1>::empty();
        let mut memory = [MaybeUninit::uninit(); 2];
        tlsf.initialize(&mut memory);
        assert!(tlsf.anchor.is_none());
        assert!(tlsf.free_blocks().is_empty());
    }

    #[test]
    fn one() {
        let mut tlsf = Tlsf::<1>::empty();
        let mut memory = [MaybeUninit::uninit(); 3];
        tlsf.initialize(&mut memory);

        let blocks = tlsf.free_blocks();
        assert_eq!(1, blocks.len());
        assert_eq!(4, blocks[0].usable_size());
    }

    #[test]
    fn initialized_twice() {
        let mut tlsf = Tlsf::<1>::empty();
        let mut memory = [MaybeUninit::uninit(); 3];
        tlsf.initialize(&mut memory);

        {
            #[cfg(not(miri))]
            cov_mark::check!(initialized_twice);
            tlsf.initialize(&mut []);
        }
    }

    #[test]
    fn max_usable_size() {
        #[cfg(not(miri))]
        cov_mark::check!(initialize_max_usable_size);

        let mut tlsf = Tlsf::<1>::empty();
        let mut memory = [MaybeUninit::uninit(); 16 * 1024 + 2];
        tlsf.initialize(&mut memory);

        let blocks = tlsf.free_blocks();
        assert_eq!(1, blocks.len());
        assert!(blocks[0].is_last_phys_block());
        assert_eq!(consts::MAX_USABLE_SIZE, blocks[0].usable_size());
    }

    #[test]
    fn two() {
        let mut tlsf = Tlsf::<1>::empty();
        let mut memory = [MaybeUninit::uninit(); 16 * 1024 + 3];
        tlsf.initialize(&mut memory);
        let blocks = tlsf.free_blocks();
        assert_eq!(2, blocks.len());
        let [a, b] = blocks.try_into().unwrap();

        assert_eq!(consts::MAX_USABLE_SIZE, a.usable_size());
        assert!(!a.is_last_phys_block());

        assert_eq!(4, b.usable_size());
        assert!(b.is_last_phys_block());
    }

    #[cfg(not(miri))] // slow
    #[test]
    fn max_pool_size() {
        let mut tlsf = Tlsf::<1>::empty();
        let mut memory =
            vec![MaybeUninit::<u32>::uninit(); consts::MAX_POOL_SIZE].into_boxed_slice();
        tlsf.initialize(&mut memory[..]);

        let blocks = tlsf.free_blocks();
        assert_eq!(4, blocks.len());
        let total_usable_size = blocks
            .iter()
            .map(|block| block.usable_size() as usize)
            .sum::<usize>();
        assert_eq!(262_128, total_usable_size);
    }

    // try to trigger any sort of arithmetic overflow
    #[cfg(not(miri))] // slow
    #[test]
    fn stress() {
        let mut memory = vec![MaybeUninit::<u32>::uninit(); 65 * 1024].into_boxed_slice();
        for i in 0..memory.len() {
            let memory = &mut memory[..i];
            let mut tlsf = Tlsf::<1>::empty();
            tlsf.initialize(memory);
        }
    }
}
