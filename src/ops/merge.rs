use crate::block::{Anchor, FreeBlock};

impl<'a> FreeBlock<'a> {
    /// # Safety
    /// - caller must ensure that the merged size does not exceed `MAX_USABLE_SIZE`
    /// - caller must ensure that the blockss are contiguous
    pub unsafe fn merge(&self, anchor: Anchor<'a>, next: Self) {
        #[cfg(any(fuzzing, test))]
        debug_assert!(u16::try_from(
            usize::from(self.usable_size())
                .checked_add(next.total_size())
                .unwrap()
        )
        .is_ok());

        let new_usable_size = self.usable_size().wrapping_add(next.total_size() as u16);

        self.resize(new_usable_size);

        if let Some(next_next) = anchor.next_phys_block(&next) {
            #[cfg(all(test, not(miri)))]
            cov_mark::hit!(merge_with_not_last_phys_block);

            next_next.set_prev_phys_block(anchor.offset_of(self));
        } else {
            #[cfg(all(test, not(miri)))]
            cov_mark::hit!(merge_with_last_phys_block);

            self.set_last_phys_block();
        }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::MaybeUninit;

    use crate::helpers::FreeBlocks;

    #[test]
    fn next_is_last_phys_block() {
        let mut memory = [MaybeUninit::uninit(); 5];
        let mut free_blocks = FreeBlocks::new(&mut memory);
        let anchor = free_blocks.anchor;
        unsafe {
            let first = free_blocks.next(4, false);
            let second = free_blocks.next(4, true);

            {
                #[cfg(not(miri))]
                cov_mark::check!(merge_with_last_phys_block);

                first.merge(anchor, second);
            }

            assert_eq!(12, first.usable_size());
            assert!(first.is_last_phys_block());
        }
    }

    #[test]
    fn next_is_not_last_phys_block() {
        let mut memory = [MaybeUninit::uninit(); 7];
        let mut free_blocks = FreeBlocks::new(&mut memory);
        let anchor = free_blocks.anchor;
        unsafe {
            let first = free_blocks.next(4, false);
            let second = free_blocks.next(4, false);
            let third = free_blocks.next(4, true);

            {
                #[cfg(not(miri))]
                cov_mark::check!(merge_with_not_last_phys_block);

                first.merge(anchor, second);
            }

            assert_eq!(12, first.usable_size());
            assert!(!first.is_last_phys_block());
            assert_eq!(Some(anchor.offset_of(&first)), third.get_prev_phys_block());
        }
    }
}
