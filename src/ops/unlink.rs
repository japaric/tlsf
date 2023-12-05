use crate::block::{Anchor, FreeBlock};
use crate::header::Header;
use crate::mapping::Indices;

impl<const FLL: usize> Header<FLL> {
    pub(super) unsafe fn unlink<'a>(&mut self, anchor: Anchor<'a>, block: &FreeBlock<'a>) {
        let Indices { fl, sl } = Self::mapping_insert(block.usable_size());

        #[cfg(any(fuzzing, test))]
        debug_assert!(self.get_free_list(fl, sl).is_some());

        match (block.get_prev_free(), block.get_next_free()) {
            (None, None) => {
                #[cfg(all(test, not(miri)))]
                cov_mark::hit!(unlink_last);

                self.set_free_list(fl, sl, None);

                self.clear_sl_bit(fl, sl);

                if self.is_sl_empty(fl) {
                    self.clear_fl_bit(fl);
                }
            }

            (None, Some(next_offset)) => {
                #[cfg(all(test, not(miri)))]
                cov_mark::hit!(unlink_head);

                let next_block = anchor.get_free_block(next_offset);
                next_block.clear_prev_free();

                self.set_free_list(fl, sl, Some(next_offset));
            }

            (Some(prev_offset), None) => {
                #[cfg(all(test, not(miri)))]
                cov_mark::hit!(unlink_tail);

                let prev_block = anchor.get_free_block(prev_offset);
                prev_block.clear_next_free();
            }

            (Some(prev_offset), Some(next_offset)) => {
                #[cfg(all(test, not(miri)))]
                cov_mark::hit!(unlink_middle);

                let prev = anchor.get_free_block(prev_offset);
                let next = anchor.get_free_block(next_offset);

                prev.set_next_free(next_offset);
                next.set_prev_free(prev_offset);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::MaybeUninit;

    use super::*;
    use crate::helpers::FreeBlocks;

    #[test]
    fn last_block_in_free_list() {
        let mut header = Header::<1>::new();
        let mut memory = [MaybeUninit::uninit(); 3];
        let mut free_blocks = FreeBlocks::new(&mut memory);
        let anchor = free_blocks.anchor;
        unsafe {
            let first = free_blocks.next(4, true);
            header.push(anchor, first.clone());

            assert!(header.is_sl_bit_set(0, 1));
            assert!(header.is_fl_bit_set(0));

            {
                #[cfg(not(miri))]
                cov_mark::check!(unlink_last);

                header.unlink(anchor, &first);
            }

            assert!(!header.is_sl_bit_set(0, 1));
            assert!(!header.is_fl_bit_set(0));
            assert!(header.linked_free_blocks(anchor).is_empty());
        }
    }

    #[test]
    fn free_list_head() {
        let mut header = Header::<1>::new();
        let mut memory = [MaybeUninit::uninit(); 5];
        let mut free_blocks = FreeBlocks::new(&mut memory);
        let anchor = free_blocks.anchor;
        unsafe {
            let tail = free_blocks.next(4, false);
            header.push(anchor, tail.clone());

            let head = free_blocks.next(4, true);
            header.push(anchor, head.clone());

            assert!(header.is_sl_bit_set(0, 1));
            assert!(header.is_fl_bit_set(0));

            {
                #[cfg(not(miri))]
                cov_mark::check!(unlink_head);

                header.unlink(anchor, &head);
            }

            assert!(header.is_sl_bit_set(0, 1));
            assert!(header.is_fl_bit_set(0));
            let [remaining] = header.linked_free_blocks(anchor).try_into().unwrap();
            assert_eq!(anchor.offset_of(&tail), anchor.offset_of(&remaining));
        }
    }

    #[test]
    fn free_list_tail() {
        let mut header = Header::<1>::new();
        let mut memory = [MaybeUninit::uninit(); 5];
        let mut free_blocks = FreeBlocks::new(&mut memory);
        let anchor = free_blocks.anchor;
        unsafe {
            let tail = free_blocks.next(4, false);
            header.push(anchor, tail.clone());

            let head = free_blocks.next(4, true);
            header.push(anchor, head.clone());

            assert!(header.is_sl_bit_set(0, 1));
            assert!(header.is_fl_bit_set(0));

            {
                #[cfg(not(miri))]
                cov_mark::check!(unlink_tail);

                header.unlink(anchor, &tail);
            }

            assert!(header.is_sl_bit_set(0, 1));
            assert!(header.is_fl_bit_set(0));
            let [remaining] = header.linked_free_blocks(anchor).try_into().unwrap();
            assert_eq!(anchor.offset_of(&head), anchor.offset_of(&remaining));
        }
    }

    #[test]
    fn from_middle_of_free_list() {
        let mut header = Header::<1>::new();
        let mut memory = [MaybeUninit::uninit(); 7];

        let mut free_blocks = FreeBlocks::new(&mut memory);
        let anchor = free_blocks.anchor;
        unsafe {
            let tail = free_blocks.next(4, false);
            header.push(anchor, tail.clone());

            let middle = free_blocks.next(4, false);
            header.push(anchor, middle.clone());

            let head = free_blocks.next(4, true);
            header.push(anchor, head.clone());

            assert!(header.is_sl_bit_set(0, 1));
            assert!(header.is_fl_bit_set(0));

            {
                #[cfg(not(miri))]
                cov_mark::check!(unlink_middle);

                header.unlink(anchor, &middle);
            }

            assert!(header.is_sl_bit_set(0, 1));
            assert!(header.is_fl_bit_set(0));
            let [a, b] = header.linked_free_blocks(anchor).try_into().unwrap();
            assert_eq!(anchor.offset_of(&head), anchor.offset_of(&a));
            assert_eq!(anchor.offset_of(&tail), anchor.offset_of(&b));
        }
    }
}
