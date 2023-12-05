#[cfg(test)]
use crate::block::Anchor;
#[cfg(test)]
use crate::block::FreeBlock;
use crate::block::Offset;
use crate::consts;

#[repr(align(4))]
pub struct Header<const FLL: usize> {
    fl_bitmap: u16,
    sl_bitmaps: [u16; FLL],
    free_lists: [FirstLevel; FLL],
}

impl<const FLL: usize> Header<FLL> {
    pub const fn new() -> Self {
        const FREE_LIST: FreeList = None;
        const FIRST_LEVEL: FirstLevel = [FREE_LIST; consts::SLL as usize];

        Self {
            fl_bitmap: 0,
            sl_bitmaps: [0; FLL],
            free_lists: [FIRST_LEVEL; FLL],
        }
    }

    /// # Safety
    /// - caller must perform bounds checks
    pub unsafe fn get_free_list(&self, fl: u8, sl: u8) -> FreeList {
        #[cfg(any(fuzzing, test))]
        debug_assert!(usize::from(fl) < FLL);
        #[cfg(any(fuzzing, test))]
        debug_assert!(sl < consts::SLL);

        *self
            .free_lists
            .get_unchecked(usize::from(fl))
            .get_unchecked(usize::from(sl))
    }

    /// # Safety
    /// - caller must perform bounds checks
    pub unsafe fn set_free_list(&mut self, fl: u8, sl: u8, free_list: FreeList) {
        #[cfg(any(fuzzing, test))]
        debug_assert!(usize::from(fl) < FLL);
        #[cfg(any(fuzzing, test))]
        debug_assert!(sl < consts::SLL);

        *self
            .free_lists
            .get_unchecked_mut(usize::from(fl))
            .get_unchecked_mut(usize::from(sl)) = free_list;
    }

    pub fn clear_fl_bit(&mut self, fl: u8) {
        self.fl_bitmap &= !1u16.wrapping_shl(fl.into());
    }

    pub fn set_fl_bit(&mut self, fl: u8) {
        self.fl_bitmap |= 1u16.wrapping_shl(fl.into());
    }

    #[cfg(test)]
    pub fn is_fl_bit_set(&self, fl: u8) -> bool {
        let mask = 1u16.wrapping_shl(fl.into());
        self.fl_bitmap & mask == mask
    }

    pub fn suitable_fls(&self, fl: u8) -> u16 {
        let mask = (!0u16).wrapping_shl(fl.into());
        self.fl_bitmap & mask
    }

    /// # Safety
    /// - caller must perform bounds checks
    pub unsafe fn get_sl_bitmap(&self, fl: u8) -> u16 {
        *self.sl_bitmap(fl)
    }

    /// # Safety
    /// - caller must perform bounds checks
    pub unsafe fn clear_sl_bit(&mut self, fl: u8, sl: u8) {
        let sl_bitmap = self.sl_bitmap_mut(fl);
        *sl_bitmap &= !1u16.wrapping_shl(sl.into());
    }

    /// # Safety
    /// - caller must perform bounds checks
    pub unsafe fn set_sl_bit(&mut self, fl: u8, sl: u8) {
        let sl_bitmap = self.sl_bitmap_mut(fl);
        *sl_bitmap |= 1u16.wrapping_shl(sl.into());
    }

    /// # Safety
    /// - caller must perform bounds checks
    #[cfg(test)]
    pub unsafe fn is_sl_bit_set(&self, fl: u8, sl: u8) -> bool {
        let mask = 1u16.wrapping_shl(sl.into());
        *self.sl_bitmap(fl) & mask == mask
    }

    pub unsafe fn suitable_sls(&self, fl: u8, sl: u8) -> u16 {
        let sl_bitmap = self.get_sl_bitmap(fl);
        let mask = (!0u16).wrapping_shl(sl.into());
        sl_bitmap & mask
    }

    /// # Safety
    /// - caller must perform bounds checks
    pub unsafe fn is_sl_empty(&self, fl: u8) -> bool {
        self.get_sl_bitmap(fl) == 0
    }

    /// # Safety
    /// - caller must perform bounds checks
    unsafe fn sl_bitmap(&self, fl: u8) -> &u16 {
        #[cfg(any(fuzzing, test))]
        debug_assert!(usize::from(fl) < FLL);

        self.sl_bitmaps.get_unchecked(usize::from(fl))
    }

    /// # Safety
    /// - caller must perform bounds checks
    unsafe fn sl_bitmap_mut(&mut self, fl: u8) -> &mut u16 {
        #[cfg(any(fuzzing, test))]
        debug_assert!(usize::from(fl) < FLL);

        self.sl_bitmaps.get_unchecked_mut(usize::from(fl))
    }

    #[cfg(test)]
    pub unsafe fn linked_free_blocks<'a>(&self, anchor: Anchor<'a>) -> Vec<FreeBlock<'a>> {
        let mut blocks = vec![];

        for free_lists in self.free_lists {
            for mut free_list in free_lists {
                while let Some(offset) = free_list {
                    let block = unsafe { anchor.get_free_block(offset) };

                    free_list = block.get_next_free();
                    blocks.push(block);
                }
            }
        }

        blocks
    }
}

type FirstLevel = [FreeList; consts::SLL as usize];
type FreeList = Option<Offset>;

#[cfg(test)]
mod tests {
    use super::Header;

    #[test]
    fn fl_bitmap_roundtrip() {
        let mut header = Header::<1>::new();
        assert!(!header.is_fl_bit_set(0));
        header.set_fl_bit(0);
        assert!(header.is_fl_bit_set(0));
    }

    #[test]
    fn sl_bitmap_roundtrip() {
        let mut header = Header::<1>::new();
        unsafe {
            assert!(!header.is_sl_bit_set(0, 0));
            header.set_sl_bit(0, 0);
            assert!(header.is_sl_bit_set(0, 0));
        }
    }
}
