use crate::block::{Anchor, FreeBlock};
use crate::header::Header;
use crate::mapping;
use crate::mapping::Indices;
#[cfg(test)]
use crate::Tlsf;

impl<const FLL: usize> Header<FLL> {
    /// # Safety
    /// - `header` must be associated to the given `anchor`
    pub unsafe fn pop<'a>(&mut self, anchor: Anchor<'a>, size: u16) -> Option<FreeBlock<'a>> {
        if size > Header::<FLL>::MAX_ALLOC_SIZE {
            return None;
        }

        let guess = unsafe { Header::<FLL>::mapping_search(size) };
        let hit = unsafe { self.find_suitable_free_list(guess)? };

        let head = unsafe { self.get_free_list(hit.fl, hit.sl) };

        #[cfg(any(fuzzing, test))]
        debug_assert!(head.is_some());

        let offset = unsafe { head.unwrap_unchecked() };
        let block = anchor.get_free_block(offset);

        #[cfg(any(fuzzing, test))]
        debug_assert!(block.get_prev_free().is_none());

        self.unlink(anchor, &block);

        Some(block)
    }

    unsafe fn find_suitable_free_list(&self, guess: Indices) -> Option<Indices> {
        #[cfg(any(fuzzing, test))]
        debug_assert!(usize::from(guess.fl) < FLL);
        #[cfg(any(fuzzing, test))]
        debug_assert!(guess.sl < crate::consts::SLL);

        let suitable_sl = self.suitable_sls(guess.fl, guess.sl);
        let (fl, sl) = if suitable_sl != 0 {
            #[cfg(all(test, not(miri)))]
            cov_mark::hit!(found_suitable_list_at_guess_fl);

            (guess.fl, mapping::find_first_bit_set(suitable_sl))
        } else {
            let suitable_fl = self.suitable_fls(guess.fl.wrapping_add(1));
            if suitable_fl == 0 {
                #[cfg(all(test, not(miri)))]
                cov_mark::hit!(found_no_suitable_list);

                return None;
            }

            #[cfg(all(test, not(miri)))]
            cov_mark::hit!(found_suitable_list_at_higher_fl);

            let fl = mapping::find_first_bit_set(suitable_fl);
            (fl, mapping::find_first_bit_set(self.get_sl_bitmap(fl)))
        };

        Some(Indices { fl, sl })
    }
}

#[cfg(test)]
impl<'a, const FLL: usize> Tlsf<'a, FLL> {
    fn pop_free(&mut self, size: u16) -> Option<FreeBlock<'a>> {
        let anchor = self.anchor?;

        unsafe { self.header.pop(anchor, size) }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::MaybeUninit;

    use super::*;
    use crate::helpers::FreeBlocks;

    #[test]
    fn when_no_free_blocks() {
        let mut tlsf = Tlsf::<1>::empty();
        let mut memory = [];
        tlsf.initialize(&mut memory);

        let block = tlsf.pop_free(0);
        assert!(block.is_none());
    }

    #[test]
    fn leaves_list_empty() {
        let mut tlsf = Tlsf::<1>::empty();
        let mut memory = [MaybeUninit::uninit(); 3];
        tlsf.initialize(&mut memory);

        let block = tlsf.pop_free(0).unwrap();
        assert_eq!(4, block.usable_size());
        assert!(!tlsf.header.is_fl_bit_set(0));
        unsafe {
            assert!(tlsf.header.is_sl_empty(0));
        }
    }

    #[test]
    fn list_stays_nonempty() {
        let mut header = Header::<1>::new();
        let mut memory = [MaybeUninit::uninit(); 5];
        let mut free_blocks = FreeBlocks::new(&mut memory);
        let anchor = free_blocks.anchor;
        unsafe {
            let first = free_blocks.next(4, false);
            header.push(anchor, first);

            let second = free_blocks.next(4, true);
            header.push(anchor, second);

            let _block = header.pop(anchor, 0).unwrap();

            // remaing block becomes unlinked
            let [other] = header.linked_free_blocks(anchor).try_into().unwrap();
            assert!(other.get_prev_free().is_none());
            assert!(other.get_next_free().is_none());

            assert!(header.is_fl_bit_set(0));
            assert!(header.is_sl_bit_set(0, 1));
        }
    }

    #[test]
    fn found_suitable_list_at_guess_fl() {
        let mut header = Header::<1>::new();
        let mut memory = [MaybeUninit::uninit(); 8];
        let mut free_blocks = FreeBlocks::new(&mut memory);
        let anchor = free_blocks.anchor;
        unsafe {
            let first = free_blocks.next(8, false);
            header.push(anchor, first);

            let second = free_blocks.next(12, true);
            header.push(anchor, second);

            let block = {
                #[cfg(not(miri))]
                cov_mark::check!(found_suitable_list_at_guess_fl);

                header.pop(anchor, 4).unwrap()
            };

            assert_eq!(8, block.usable_size());
        }
    }

    #[test]
    fn found_suitable_list_at_higher_fl() {
        let mut header = Header::<2>::new();
        let mut memory = [MaybeUninit::uninit(); 20];
        let mut free_blocks = FreeBlocks::new(&mut memory);
        let anchor = free_blocks.anchor;
        unsafe {
            let first = free_blocks.next(4, false);
            header.push(anchor, first);

            let second = free_blocks.next(64, true);
            header.push(anchor, second);

            let block = {
                #[cfg(not(miri))]
                cov_mark::check!(found_suitable_list_at_higher_fl);

                header.pop(anchor, 8).unwrap()
            };

            assert_eq!(64, block.usable_size());
        }
    }

    #[test]
    fn found_no_suitable_list() {
        let mut header = Header::<1>::new();
        let mut memory = [MaybeUninit::uninit(); 20];
        let mut free_blocks = FreeBlocks::new(&mut memory);
        let anchor = free_blocks.anchor;
        unsafe {
            let block = free_blocks.next(4, true);
            header.push(anchor, block);

            let res = {
                #[cfg(not(miri))]
                cov_mark::check!(found_no_suitable_list);

                header.pop(anchor, 8)
            };

            assert!(res.is_none());
        }
    }
}
