use core::ptr::NonNull;

use crate::Tlsf;

impl<'a, const FLL: usize> Tlsf<'a, FLL> {
    /// Returns the block of memory behind `ptr` to the allocator
    ///
    /// # Safety
    ///
    /// - `ptr` MUST denote a block of memory currently allocated via this allocator
    /// - `ptr` MUST no be freed more than once
    /// - `ptr` MUST not be used after it has been freed
    pub unsafe fn free(&mut self, ptr: NonNull<u32>) {
        let Some(anchor) = self.anchor else { return };
        let used = anchor.get_used_block(ptr);
        let free = used.into_free(anchor);
        self.header.coalesce(anchor, free);
    }
}

#[cfg(test)]
mod tests {
    use core::alloc::Layout;
    use core::mem::MaybeUninit;

    use super::*;
    use crate::block::{Anchor, Offset};
    use crate::helpers::Memory;

    #[test]
    fn it_works() {
        let mut tlsf = Tlsf::<1>::empty();
        let mut memory = [MaybeUninit::uninit(); 4];
        tlsf.initialize(&mut memory);

        let layout = Layout::new::<u32>();

        let alloc = tlsf.memalign(layout).unwrap();
        alloc.iter_mut().for_each(|mu| {
            mu.write(!0);
        });

        unsafe { tlsf.free(NonNull::from(alloc).cast()) }
    }

    #[test]
    fn furthest_block() {
        let mut tlsf = Tlsf::<1>::empty();
        let mut memory = Memory::new();
        let anchor = Anchor::new(memory.bytes());
        tlsf.anchor = Some(anchor);

        unsafe {
            let block = anchor.create_free_block(Offset::max(), 4, true, None);
            let alloc = block.into_used(anchor);
            tlsf.free(NonNull::from(alloc).cast());
        }
    }
}
