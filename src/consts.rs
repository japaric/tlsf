use crate::block::{FreeBlock, Offset};
use crate::header::Header;

pub const BLOCK_ALIGN_LOG2: u8 = 2;
pub const BLOCK_ALIGN: u8 = 1 << BLOCK_ALIGN_LOG2;

// NOTE cannot be greater beacuse `sl_bitmaps` is `[u16; _]`
pub const SLL_LOG2: u8 = 4;

// all second level indices are smaller than this value
pub const SLL: u8 = 1 << SLL_LOG2;

// For small values of the `fl` index we would have to split sizes in the range
// of e.g. `4..8` into `SLL` smaller ranges. That doesn't make much sense so
// instead we merge all the rows with `fl < LOWER_SIZE_THRESHOLD` into a single
// row.
pub const MIN_FLL: u8 = SLL_LOG2 + BLOCK_ALIGN_LOG2;
pub const LOWER_SIZE_THRESHOLD: u16 = 1 << MIN_FLL;

impl<const FLL: usize> Header<FLL> {
    #[cfg(test)]
    const HEADER_SIZE: usize = core::mem::size_of::<Header<FLL>>();

    pub(crate) const REAL_FLL: u8 = {
        assert!(FLL > 0);
        assert!(FLL < 12);
        FLL as u8 + MIN_FLL - 1
    };

    pub(crate) const UPPER_SIZE_THRESHOLD: u16 =
        ((1u32 << Self::REAL_FLL) - BLOCK_ALIGN as u32) as u16;
    pub const MAX_ALLOC_SIZE: u16 = {
        let step = 1 << (Self::REAL_FLL - SLL_LOG2 - 1);
        let step = if step <= 4 { 4 } else { step };

        ((1u32 << Self::REAL_FLL) - step) as u16
    };
}

pub const MAX_USABLE_SIZE: u16 = u16::MAX & !0b11;

pub const MAX_POOL_SIZE: usize =
    Offset::MAX_UNCOMPRESSED_VALUE +
    FreeBlock::HEADER_SIZE as usize +
    BLOCK_ALIGN as usize // anchor
;

// check documented values
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_alloc_size() {
        assert_eq!(60, Header::<1>::MAX_ALLOC_SIZE);
        assert_eq!(124, Header::<2>::MAX_ALLOC_SIZE);
        assert_eq!(248, Header::<3>::MAX_ALLOC_SIZE);
        assert_eq!(496, Header::<4>::MAX_ALLOC_SIZE);
        assert_eq!(992, Header::<5>::MAX_ALLOC_SIZE);
        assert_eq!(1_984, Header::<6>::MAX_ALLOC_SIZE);
        assert_eq!(3_968, Header::<7>::MAX_ALLOC_SIZE);
        assert_eq!(7_936, Header::<8>::MAX_ALLOC_SIZE);
        assert_eq!(15_872, Header::<9>::MAX_ALLOC_SIZE);
        assert_eq!(31_744, Header::<10>::MAX_ALLOC_SIZE);
        assert_eq!(63_488, Header::<11>::MAX_ALLOC_SIZE);
    }

    #[test]
    fn header_size() {
        assert_eq!(36, Header::<1>::HEADER_SIZE);
        assert_eq!(72, Header::<2>::HEADER_SIZE);
        assert_eq!(104, Header::<3>::HEADER_SIZE);
        assert_eq!(140, Header::<4>::HEADER_SIZE);
        assert_eq!(172, Header::<5>::HEADER_SIZE);
        assert_eq!(208, Header::<6>::HEADER_SIZE);
        assert_eq!(240, Header::<7>::HEADER_SIZE);
        assert_eq!(276, Header::<8>::HEADER_SIZE);
        assert_eq!(308, Header::<9>::HEADER_SIZE);
        assert_eq!(344, Header::<10>::HEADER_SIZE);
        assert_eq!(376, Header::<11>::HEADER_SIZE);
    }
}
