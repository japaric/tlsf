use core::mem::MaybeUninit;
use core::num::NonZeroU16;

use super::util;
use crate::block::Anchor;
use crate::header::Header;
use crate::Tlsf;

impl<'a, const FLL: usize> Tlsf<'a, FLL> {
    /// Allocates a memory block of the requested `size`
    ///
    /// The returned block is guaranteed to have an alignment of 4 bytes and may exceed the
    /// requested `size`.
    ///
    /// This function returns `None` when `there's insufficient free memory to satisfy the request
    pub fn malloc(&mut self, size: NonZeroU16) -> Option<&'a mut [MaybeUninit<u32>]> {
        let anchor = self.anchor?;
        unsafe { self.header.malloc(anchor, size) }
    }
}

impl<const FLL: usize> Header<FLL> {
    unsafe fn malloc<'a>(
        &mut self,
        anchor: Anchor<'a>,
        size: NonZeroU16,
    ) -> Option<&'a mut [MaybeUninit<u32>]> {
        let size = util::round_up_block_size(size.into())?;

        let mut block = self.pop(anchor, size)?;

        block = self.adjust_free_block_size(anchor, block, size);

        #[cfg(any(fuzzing, test))]
        debug_assert!(block.usable_size() >= size);

        let alloc = block.into_used(anchor);

        Some(alloc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_split() {
        let mut tlsf = Tlsf::<1>::empty();
        let mut memory = [MaybeUninit::uninit(); 3];
        tlsf.initialize(&mut memory);

        let [free] = tlsf.free_blocks().try_into().unwrap();
        assert_eq!(4, free.usable_size());

        let alloc = tlsf.malloc(1.try_into().unwrap()).unwrap();
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

        let alloc = tlsf.malloc(1.try_into().unwrap()).unwrap();
        assert_eq!(1, alloc.len());

        let [remaining] = tlsf.free_blocks().try_into().unwrap();
        assert_eq!(8, remaining.total_size());
    }

    #[cfg(not(miri))] // slow
    #[test]
    fn stress() {
        let mut memory = vec![MaybeUninit::<u32>::uninit(); 65 * 1024].into_boxed_slice();
        let memory = &mut memory[..];
        let mut tlsf = Tlsf::<1>::empty();
        tlsf.initialize(memory);

        let mut count = 0;
        let size = 1.try_into().unwrap();
        while let Some(alloc) = tlsf.malloc(size) {
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
