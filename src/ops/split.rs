use crate::block::{Anchor, FreeBlock, UsedBlock};

impl<'a> Anchor<'a> {
    /// # Safety
    /// - caller must perform bounds checking
    /// - block must be associated to `anchor`
    pub(super) unsafe fn split(&self, block: &FreeBlock<'a>, at: usize) -> FreeBlock<'a> {
        #[cfg(any(fuzzing, test))]
        debug_assert_eq!(0, at % 4);

        #[cfg(any(fuzzing, test))]
        debug_assert!(at >= FreeBlock::HEADER_SIZE.into());

        let total_size = block.total_size();

        #[cfg(any(fuzzing, test))]
        debug_assert!(total_size >= at + usize::from(FreeBlock::HEADER_SIZE));

        let used_header_size = usize::from(UsedBlock::HEADER_SIZE);
        let is_last_phys_block = block.is_last_phys_block();
        block.resize(at.wrapping_sub(used_header_size) as u16);

        let this_offset = self.offset_of(block);
        let new_offset = this_offset.add(at);

        let prev_phys_block = Some(self.offset_of(block));
        let new_block = self.create_free_block(
            new_offset,
            total_size.wrapping_sub(at).wrapping_sub(used_header_size) as u16,
            is_last_phys_block,
            prev_phys_block,
        );

        if let Some(next_block) = self.next_phys_block(&new_block) {
            #[cfg(all(test, not(miri)))]
            cov_mark::hit!(split_last_phys_block);

            next_block.set_prev_phys_block(new_offset);
        } else {
            #[cfg(all(test, not(miri)))]
            cov_mark::hit!(split_not_last_phys_block);

            // new_block becomes the last_phys_block; clear the bit in the original one
            block.clear_last_phys_block();
        }

        #[cfg(test)]
        assert_eq!(total_size, block.total_size() + new_block.total_size());

        new_block
    }
}

#[cfg(test)]
mod tests {
    use core::mem::MaybeUninit;

    use crate::block::Offset;
    use crate::helpers::FreeBlocks;

    #[test]
    fn it_works() {
        let mut memory = [MaybeUninit::uninit(); 5];

        let mut free_blocks = FreeBlocks::new(&mut memory);
        let anchor = free_blocks.anchor;
        unsafe {
            let offset = Offset::compress(4);
            let block = free_blocks.next(12, true);
            let new = anchor.split(&block, 8);

            assert_eq!(4, block.usable_size());
            assert_eq!(4, new.usable_size());

            assert_eq!(Some(offset), new.get_prev_phys_block());
        }
    }

    #[test]
    fn last_phys_block() {
        let mut memory = [MaybeUninit::uninit(); 5];

        let mut free_blocks = FreeBlocks::new(&mut memory);
        let anchor = free_blocks.anchor;
        unsafe {
            let block = free_blocks.next(12, true);
            let new = {
                #[cfg(not(miri))]
                cov_mark::check!(split_not_last_phys_block);

                anchor.split(&block, 8)
            };

            assert!(new.is_last_phys_block());
            assert!(!block.is_last_phys_block());
        }
    }

    #[test]
    fn not_last_phys_block() {
        let mut memory = [MaybeUninit::uninit(); 7];

        let mut free_blocks = FreeBlocks::new(&mut memory);
        let anchor = free_blocks.anchor;
        unsafe {
            let first = free_blocks.next(20, true);

            let last = anchor.split(&first, 16);
            assert!(!first.is_last_phys_block());

            let mid = {
                #[cfg(not(miri))]
                cov_mark::check!(split_last_phys_block);

                anchor.split(&first, 8)
            };
            assert_eq!(Some(anchor.offset_of(&mid)), last.get_prev_phys_block());
        }
    }
}
