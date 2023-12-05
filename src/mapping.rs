use crate::consts;
use crate::header::Header;

impl<const FLL: usize> Header<FLL> {
    pub fn mapping_insert(size: u16) -> Indices {
        let (fl, sl) = if size < consts::LOWER_SIZE_THRESHOLD {
            (0, (size >> consts::BLOCK_ALIGN_LOG2) as u8)
        } else if size > Self::UPPER_SIZE_THRESHOLD {
            (FLL as u8 - 1, consts::SLL - 1)
        } else {
            let mut fl = find_last_bit_set(size);
            let sl =
                size.wrapping_shr(fl.wrapping_sub(consts::SLL_LOG2).into()) as u8 & !consts::SLL;
            fl = fl.wrapping_sub(consts::MIN_FLL - 1);
            (fl, sl)
        };

        #[cfg(any(fuzzing, test))]
        debug_assert!(fl < Self::REAL_FLL);

        #[cfg(any(fuzzing, test))]
        debug_assert!(sl < consts::SLL);

        Indices { fl, sl }
    }

    /// # Safety
    /// - `size` <= MAX_ALLOC_SIZE
    pub unsafe fn mapping_search(mut size: u16) -> Indices {
        #[cfg(any(fuzzing, test))]
        debug_assert!(size <= Self::MAX_ALLOC_SIZE);

        if size >= consts::LOWER_SIZE_THRESHOLD {
            size = (size - 1).wrapping_add(
                1u16.wrapping_shl(
                    find_last_bit_set(size)
                        .wrapping_sub(consts::SLL_LOG2)
                        .into(),
                ),
            );
        }

        Self::mapping_insert(size)
    }
}

fn find_last_bit_set(num: u16) -> u8 {
    15u8.wrapping_sub(num.leading_zeros() as u8)
}

pub fn find_first_bit_set(num: u16) -> u8 {
    num.trailing_zeros() as u8
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(test, derive(Hash, Eq))]
pub struct Indices {
    /// First Level
    pub fl: u8,
    /// Second Level
    pub sl: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapping_insert() {
        // 0, 15: 60..64; 1, 0: 64..68
        assert_eq!(Indices { fl: 0, sl: 15 }, Header::<1>::mapping_insert(60));
        assert_eq!(Indices { fl: 0, sl: 15 }, Header::<1>::mapping_insert(64));
        assert_eq!(Indices { fl: 0, sl: 15 }, Header::<1>::mapping_insert(68));

        assert_eq!(Indices { fl: 0, sl: 15 }, Header::<3>::mapping_insert(60));
        assert_eq!(Indices { fl: 1, sl: 0 }, Header::<3>::mapping_insert(64));

        // 1, 15: 124..128; 2, 0: 128..136
        assert_eq!(Indices { fl: 1, sl: 15 }, Header::<3>::mapping_insert(124));
        assert_eq!(Indices { fl: 2, sl: 0 }, Header::<3>::mapping_insert(128));
        assert_eq!(Indices { fl: 2, sl: 0 }, Header::<3>::mapping_insert(132));

        // UPPER_SIZE_THRESHOLD = 124
        assert_eq!(Indices { fl: 1, sl: 14 }, Header::<2>::mapping_insert(120));
        assert_eq!(Indices { fl: 1, sl: 15 }, Header::<2>::mapping_insert(124));
        assert_eq!(Indices { fl: 1, sl: 15 }, Header::<2>::mapping_insert(128));
        assert_eq!(Indices { fl: 1, sl: 15 }, Header::<2>::mapping_insert(256));
    }

    #[test]
    fn mapping_search() {
        unsafe {
            // 0, 15: 60..64; 1, 0: 64..68
            assert_eq!(Indices { fl: 0, sl: 15 }, Header::<1>::mapping_search(60));
            assert_eq!(Indices { fl: 0, sl: 15 }, Header::<3>::mapping_search(60));
            assert_eq!(Indices { fl: 1, sl: 0 }, Header::<3>::mapping_search(64));

            // 1, 15: 124..128; 2, 0: 128..136
            assert_eq!(Indices { fl: 1, sl: 15 }, Header::<3>::mapping_search(124));
            assert_eq!(Indices { fl: 2, sl: 0 }, Header::<3>::mapping_search(128));
            assert_eq!(Indices { fl: 2, sl: 1 }, Header::<3>::mapping_search(132));

            // 1, 15: 124..128; 2, 0: 128..136
            assert_eq!(124, Header::<2>::MAX_ALLOC_SIZE);
            assert_eq!(Indices { fl: 1, sl: 14 }, Header::<2>::mapping_search(120));
            assert_eq!(Indices { fl: 1, sl: 15 }, Header::<2>::mapping_search(124));

            // 2, 14: 240..248; 2, 15: 248..256
            assert_eq!(248, Header::<3>::MAX_ALLOC_SIZE);
            assert_eq!(Indices { fl: 2, sl: 15 }, Header::<3>::mapping_search(244));
            assert_eq!(Indices { fl: 2, sl: 15 }, Header::<3>::mapping_search(248));
        }
    }
}
