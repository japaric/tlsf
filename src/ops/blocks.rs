#[cfg(test)]
use crate::block::FreeBlock;
use crate::block::{Anchor, Offset};
use crate::header::Header;
use crate::{Block, Tlsf};

impl<const FLL: usize> Tlsf<'_, FLL> {
    /// Returns an iterator over all the memory blocks managed by the allocator
    ///
    /// The iteration order is from lowest memory address to highest memory address
    ///
    /// While the iterator is in scope it's not possible to request memory or return memory to the
    /// allocator
    pub fn blocks(&self) -> Blocks {
        if let Some(anchor) = self.anchor {
            self.header.blocks(anchor)
        } else {
            Blocks {
                anchor: Anchor::new(&mut []),
                current: None,
            }
        }
    }

    #[cfg(test)]
    pub fn free_blocks(&self) -> Vec<FreeBlock> {
        let Some(anchor) = self.anchor else {
            return vec![];
        };

        unsafe { self.header.free_blocks(anchor) }
    }
}

impl<const FLL: usize> Header<FLL> {
    fn blocks<'a>(&self, anchor: Anchor<'a>) -> Blocks<'a> {
        Blocks {
            anchor,
            current: Some(unsafe { Offset::compress(4) }),
        }
    }

    #[cfg(test)]
    pub unsafe fn free_blocks<'a>(&self, anchor: Anchor<'a>) -> Vec<FreeBlock<'a>> {
        self.blocks(anchor)
            .filter_map(|block| unsafe { block.try_into_free(anchor) })
            .collect()
    }
}

pub struct Blocks<'a> {
    anchor: Anchor<'a>,
    current: Option<Offset>,
}

impl<'a> Iterator for Blocks<'a> {
    type Item = Block<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let offset = self.current?;
        unsafe {
            let block = self.anchor.block_at(offset);
            let next = self.anchor.next_phys_block(&block);
            self.current = next.map(|block| self.anchor.offset_of(&block));
            Some(block)
        }
    }
}

#[cfg(test)]
mod tests {
    use core::alloc::Layout;
    use core::mem::MaybeUninit;
    use core::ptr::NonNull;

    use super::*;

    #[test]
    fn it_works() {
        let mut tlsf = Tlsf::<1>::empty();
        let mut memory = [MaybeUninit::uninit(); 7];
        tlsf.initialize(&mut memory);

        let blocks = tlsf.blocks().collect::<Vec<_>>();
        assert_eq!(1, blocks.len());
        assert_eq!(24, blocks[0].total_size());

        let min_layout = Layout::new::<u8>();
        let first = tlsf.memalign(min_layout).unwrap();
        assert_eq!(1, first.len());

        let blocks = tlsf.blocks().collect::<Vec<_>>();
        assert_eq!(2, blocks.len());
        let [a, b] = blocks.try_into().unwrap();

        // OK to mutate memory previously returned by the allocator while the block _headers_ are
        // in scope
        first.iter_mut().for_each(|mu| {
            mu.write(!0);
        });

        assert!(!a.is_free());
        assert_eq!(4, a.usable_size());

        assert!(b.is_free());
        assert_eq!(16, b.total_size());

        let second = tlsf.memalign(min_layout).unwrap();
        assert_eq!(1, second.len());

        let blocks = tlsf.blocks().collect::<Vec<_>>();
        assert_eq!(3, blocks.len());
        let [a, b, c] = blocks.try_into().unwrap();

        first.iter_mut().for_each(|mu| {
            mu.write(!0);
        });
        second.iter_mut().for_each(|mu| {
            mu.write(!0);
        });

        assert!(!a.is_free());
        assert_eq!(4, a.usable_size());

        assert!(!b.is_free());
        assert_eq!(4, b.usable_size());

        assert!(c.is_free());
        assert_eq!(8, c.total_size());

        unsafe { tlsf.free(NonNull::from(first).cast()) }

        let blocks = tlsf.blocks().collect::<Vec<_>>();
        assert_eq!(3, blocks.len());
        let [a, b, c] = blocks.try_into().unwrap();

        second.iter_mut().for_each(|mu| {
            mu.write(!0);
        });

        assert!(a.is_free());
        assert_eq!(4, a.usable_size());

        assert!(!b.is_free());
        assert_eq!(4, b.usable_size());

        assert!(c.is_free());
        assert_eq!(8, c.total_size());
    }

    #[test]
    fn no_anchor() {
        let mut tlsf = Tlsf::<1>::empty();
        let mut memory = [MaybeUninit::uninit(); 2];
        tlsf.initialize(&mut memory);

        assert!(tlsf.blocks().next().is_none());
    }
}
