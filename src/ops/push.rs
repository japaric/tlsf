use crate::block::{Anchor, FreeBlock};
use crate::header::Header;
use crate::mapping::Indices;

impl<const FLL: usize> Header<FLL> {
    /// # Safety
    /// - `block` must be covered by `anchor`
    pub(super) unsafe fn push<'a>(&mut self, anchor: Anchor<'a>, block: FreeBlock<'a>) {
        let Indices { fl, sl } = Header::<FLL>::mapping_insert(block.usable_size());

        let free_list = unsafe { self.get_free_list(fl, sl) };

        if let Some(head) = free_list {
            #[cfg(all(test, not(miri)))]
            cov_mark::hit!(push_non_empty_list);

            let head = anchor.get_free_block(head);
            head.set_prev_free(anchor.offset_of(&block));
            block.set_next_free(anchor.offset_of(&head));
        } else {
            #[cfg(all(test, not(miri)))]
            cov_mark::hit!(push_empty_list);

            block.clear_next_free();
        }
        block.clear_prev_free();

        let new_free_list = Some(anchor.offset_of(&block));
        self.set_free_list(fl, sl, new_free_list);

        self.set_fl_bit(fl);
        self.set_sl_bit(fl, sl);
    }
}

#[cfg(test)]
mod tests {
    use core::mem::MaybeUninit;

    use super::*;
    use crate::helpers::FreeBlocks;

    #[test]
    fn one() {
        let mut header = Header::<1>::new();
        let mut memory = [MaybeUninit::uninit(); 3];
        let mut free_blocks = FreeBlocks::new(&mut memory);
        let anchor = free_blocks.anchor;
        unsafe {
            let block = free_blocks.next(4, true);

            {
                #[cfg(not(miri))]
                cov_mark::check!(push_empty_list);

                header.push(anchor, block);
            }
        }

        let blocks = unsafe { header.free_blocks(free_blocks.anchor) };
        assert_eq!(1, blocks.len());
        let [block] = blocks.try_into().unwrap();
        assert_eq!(4, block.usable_size());
        assert!(header.is_fl_bit_set(0));
        unsafe {
            assert!(header.is_sl_bit_set(0, 1));
        }
    }

    #[test]
    fn two_same_list() {
        let mut header = Header::<1>::new();
        let mut memory = [MaybeUninit::uninit(); 5];
        let mut free_blocks = FreeBlocks::new(&mut memory);
        let anchor = free_blocks.anchor;
        unsafe {
            let first = free_blocks.next(4, false);
            header.push(anchor, first);

            let second = free_blocks.next(4, true);
            {
                #[cfg(not(miri))]
                cov_mark::check!(push_non_empty_list);

                header.push(anchor, second);
            }
        }

        let blocks = unsafe { header.free_blocks(anchor) };
        assert_eq!(2, blocks.len());
        let [a, b] = blocks.try_into().unwrap();
        unsafe {
            assert_eq!(Some(anchor.offset_of(&a)), b.get_next_free());
            assert!(b.get_prev_free().is_none());

            assert!(a.get_next_free().is_none());
            assert_eq!(Some(anchor.offset_of(&b)), a.get_prev_free());
        }

        assert!(header.is_fl_bit_set(0));
        unsafe {
            assert!(header.is_sl_bit_set(0, 1));
        }
    }

    #[test]
    fn two_different_lists() {
        let mut header = Header::<1>::new();
        let mut memory = [MaybeUninit::uninit(); 6];
        let mut free_blocks = FreeBlocks::new(&mut memory);
        let anchor = free_blocks.anchor;
        unsafe {
            let first = free_blocks.next(4, false);
            header.push(anchor, first);

            let second = free_blocks.next(8, true);
            header.push(anchor, second);
        }

        let blocks = unsafe { header.free_blocks(anchor) };
        assert_eq!(2, blocks.len());
        assert_eq!(4, blocks[0].usable_size());
        assert!(header.is_fl_bit_set(0));
        unsafe {
            assert!(header.is_sl_bit_set(0, 1));
            assert!(header.is_sl_bit_set(0, 2));
        }
    }
}
