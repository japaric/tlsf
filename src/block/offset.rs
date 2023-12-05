use core::num::NonZeroU16;

use crate::consts;

/// Compressed offset
#[derive(Debug, Clone, Copy)]
#[cfg_attr(test, derive(PartialOrd, PartialEq, Ord, Eq))]
pub struct Offset(NonZeroU16);

impl Offset {
    pub const MAX_UNCOMPRESSED_VALUE: usize = (u16::MAX as usize) << consts::BLOCK_ALIGN_LOG2;

    /// compresses the given `offset`
    ///
    /// # Safety
    /// - caller must ensure `offset` fits into a `u16` value after compression
    pub unsafe fn compress(offset: usize) -> Self {
        let compressed = offset >> consts::BLOCK_ALIGN_LOG2;

        #[cfg(any(fuzzing, test))]
        debug_assert!(u16::try_from(compressed).is_ok());

        Self(NonZeroU16::new_unchecked(compressed as u16))
    }

    /// returns the compressed offset value
    pub(super) fn get(&self) -> usize {
        self.0.get() as usize
    }

    /// # Safety
    /// - caller must ensure the result can be compressed
    pub unsafe fn add(&self, uncompressed: usize) -> Offset {
        Self::compress(self.uncompress().wrapping_add(uncompressed))
    }

    #[cfg(test)]
    pub fn max() -> Self {
        Self(NonZeroU16::new(u16::MAX).unwrap())
    }

    pub(super) fn uncompress(&self) -> usize {
        self.get() << consts::BLOCK_ALIGN_LOG2
    }
}
