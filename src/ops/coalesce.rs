use crate::block::{Anchor, FreeBlock};
use crate::header::Header;

impl<const FLL: usize> Header<FLL> {
    pub(super) unsafe fn coalesce<'a>(&mut self, anchor: Anchor<'a>, mut block: FreeBlock<'a>) {
        let (prev, next) = self.merge_candidates(anchor, &block);

        if let Some(prev) = prev {
            self.unlink(anchor, &prev);
            prev.merge(anchor, block);
            block = prev;
        }

        if let Some(next) = next {
            self.unlink(anchor, &next);
            block.merge(anchor, next);
        }

        self.push(anchor, block);
    }

    unsafe fn merge_candidates<'a>(
        &self,
        anchor: Anchor<'a>,
        block: &FreeBlock<'a>,
    ) -> (Option<FreeBlock<'a>>, Option<FreeBlock<'a>>) {
        let size = usize::from(block.usable_size());
        let prev = anchor
            .prev_phys_block(block)
            .and_then(|block| block.try_into_free(anchor));
        let next = anchor
            .next_phys_block(block)
            .and_then(|block| block.try_into_free(anchor));

        match (prev, next) {
            (Some(prev), Some(next)) => {
                let prev_size = prev.total_size();
                let next_size = next.total_size();

                if u16::try_from(size.wrapping_add(prev_size).wrapping_add(next_size)).is_ok() {
                    #[cfg(all(test, not(miri)))]
                    cov_mark::hit!(merge_both_sides);

                    return (Some(prev), Some(next));
                } else if prev_size > next_size
                    && u16::try_from(size.wrapping_add(prev_size)).is_ok()
                {
                    return (Some(prev), None);
                } else if next_size > prev_size
                    && u16::try_from(size.wrapping_add(next_size)).is_ok()
                {
                    return (None, Some(next));
                }
            }

            (Some(prev), _) => {
                if u16::try_from(size.wrapping_add(prev.total_size())).is_ok() {
                    #[cfg(all(test, not(miri)))]
                    cov_mark::hit!(merge_prev);

                    return (Some(prev), None);
                }
            }

            (_, Some(next)) => {
                if u16::try_from(size.wrapping_add(next.total_size())).is_ok() {
                    #[cfg(all(test, not(miri)))]
                    cov_mark::hit!(merge_next);

                    return (None, Some(next));
                }
            }

            _ => {}
        }

        (None, None)
    }
}

#[cfg(test)]
mod tests {
    use core::mem::MaybeUninit;

    use super::*;
    use crate::helpers::FreeBlocks;

    #[test]
    fn next() {
        let mut header = Header::<1>::new();
        let mut memory = [MaybeUninit::uninit(); 5];
        let mut free_blocks = FreeBlocks::new(&mut memory);
        let anchor = free_blocks.anchor;
        unsafe {
            let first = free_blocks.next(4, false);

            let second = free_blocks.next(4, true);
            header.push(anchor, second);

            assert!(header.is_sl_bit_set(0, 1));
            assert!(header.is_fl_bit_set(0));
            {
                #[cfg(not(miri))]
                cov_mark::check!(merge_next);

                header.coalesce(anchor, first.clone());
            }
            assert!(header.is_sl_bit_set(0, 3));
            assert!(header.is_fl_bit_set(0));

            let [block] = header.free_blocks(anchor).try_into().unwrap();
            assert_eq!(12, block.usable_size());
            assert_eq!(anchor.offset_of(&block), anchor.offset_of(&first));
        }
    }

    #[test]
    fn prev() {
        let mut header = Header::<1>::new();
        let mut memory = [MaybeUninit::uninit(); 5];
        let mut free_blocks = FreeBlocks::new(&mut memory);
        let anchor = free_blocks.anchor;
        unsafe {
            let first = free_blocks.next(4, false);
            header.push(anchor, first.clone());

            let second = free_blocks.next(4, true);

            assert!(header.is_sl_bit_set(0, 1));
            assert!(header.is_fl_bit_set(0));
            {
                #[cfg(not(miri))]
                cov_mark::check!(merge_prev);

                header.coalesce(anchor, second);
            }
            assert!(header.is_sl_bit_set(0, 3));
            assert!(header.is_fl_bit_set(0));

            let [block] = header.free_blocks(anchor).try_into().unwrap();
            assert_eq!(12, block.usable_size());
            assert_eq!(anchor.offset_of(&block), anchor.offset_of(&first));
        }
    }

    #[test]
    fn both_sides() {
        let mut header = Header::<1>::new();
        let mut memory = [MaybeUninit::uninit(); 7];
        let mut free_blocks = FreeBlocks::new(&mut memory);
        let anchor = free_blocks.anchor;
        unsafe {
            let first = free_blocks.next(4, false);
            header.push(anchor, first.clone());

            let second = free_blocks.next(4, false);

            let third = free_blocks.next(4, true);
            header.push(anchor, third);

            assert!(header.is_sl_bit_set(0, 1));
            assert!(header.is_fl_bit_set(0));
            {
                #[cfg(not(miri))]
                cov_mark::check!(merge_both_sides);

                header.coalesce(anchor, second);
            }
            assert!(header.is_sl_bit_set(0, 5));
            assert!(header.is_fl_bit_set(0));

            let [block] = header.free_blocks(anchor).try_into().unwrap();
            assert_eq!(20, block.usable_size());
            assert_eq!(anchor.offset_of(&block), anchor.offset_of(&first));
        }
    }

    #[test]
    fn no_merge() {
        let mut header = Header::<1>::new();
        let mut memory = [MaybeUninit::uninit(); 7];
        let mut free_blocks = FreeBlocks::new(&mut memory);
        let anchor = free_blocks.anchor;
        unsafe {
            let first = free_blocks.next(4, false);
            first.into_used(anchor);

            let second = free_blocks.next(4, false);

            let third = free_blocks.next(4, true);
            third.into_used(anchor);

            assert!(!header.is_fl_bit_set(0));
            header.coalesce(anchor, second.clone());
            assert!(header.is_sl_bit_set(0, 1));
            assert!(header.is_fl_bit_set(0));

            let [block] = header.free_blocks(anchor).try_into().unwrap();
            assert_eq!(4, block.usable_size());
            assert_eq!(anchor.offset_of(&block), anchor.offset_of(&second));
        }
    }
}
