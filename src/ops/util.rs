use crate::block::{Anchor, FreeBlock, UsedBlock};
use crate::consts;
use crate::header::Header;

pub fn round_up_block_size(num: u16) -> Option<u16> {
    let multiple = consts::BLOCK_ALIGN as u16;
    let rem = num % multiple;
    if rem == 0 {
        Some(num)
    } else {
        u16::try_from((num as u32) + (multiple as u32 - rem as u32)).ok()
    }
}

impl<const FLL: usize> Header<FLL> {
    pub(super) unsafe fn adjust_free_block_size<'a>(
        &mut self,
        anchor: Anchor<'a>,
        block: FreeBlock<'a>,
        size: u16,
    ) -> FreeBlock<'a> {
        if u32::from(block.usable_size()) >= u32::from(size) + u32::from(FreeBlock::HEADER_SIZE) {
            #[cfg(all(test, not(miri)))]
            cov_mark::hit!(alloc_adjust_size);

            let at = usize::from(UsedBlock::HEADER_SIZE) + usize::from(size);
            let new = unsafe { anchor.split(&block, at) };
            unsafe { self.push(anchor, new) }
        }

        block
    }
}
