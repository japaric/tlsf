#![allow(unstable_name_collisions)]

use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ptr::NonNull;

use sptr::Strict;

use super::common::Header;
use super::{Block, FreeBlock, Offset, UsedBlock};
use crate::consts;

#[derive(Clone, Copy, Debug)]
pub struct Anchor<'a> {
    ptr: NonNull<u32>,
    _lifetime: PhantomData<&'a mut [MaybeUninit<u32>]>,
}

impl<'a> Anchor<'a> {
    pub fn new(memory: &'a mut [MaybeUninit<u32>]) -> Self {
        unsafe {
            Anchor {
                ptr: NonNull::new_unchecked(memory.as_mut_ptr()).cast(),
                _lifetime: PhantomData,
            }
        }
    }

    /// # Safety
    /// - caller must perform bounds checking
    pub unsafe fn create_free_block(
        &self,
        at: Offset,
        usable_size: u16,
        is_last_phys_block: bool,
        prev_phys_block: Option<Offset>,
    ) -> FreeBlock<'a> {
        let ptr = self.resolve_offset(at).cast();
        FreeBlock::new(ptr, usable_size, is_last_phys_block, prev_phys_block)
    }

    /// # Safety
    /// - caller must perform bounds checking
    pub unsafe fn get_free_block(&self, at: Offset) -> FreeBlock<'a> {
        FreeBlock::from_ptr(self.resolve_offset(at).cast())
    }

    pub unsafe fn get_used_block(&self, body_ptr: NonNull<u32>) -> UsedBlock<'a> {
        // cannot go through offset because `body_ptr` can be outside the `Offset` range
        UsedBlock::from_ptr(
            NonNull::new_unchecked(self.ptr.as_ptr().with_addr(
                (body_ptr.as_ptr() as usize).wrapping_sub(UsedBlock::HEADER_SIZE.into()),
            ))
            .cast(),
        )
    }

    /// # Safety
    /// - caller must perform bounds checking
    pub unsafe fn block_at(&self, at: Offset) -> Block<'a> {
        Block::from_ptr(self.resolve_offset(at).cast())
    }

    /// # Safety
    /// - caller must ensure `block` is covered by this anchor
    #[allow(private_bounds)]
    pub unsafe fn next_phys_block(&self, block: &impl NextPhysBlock) -> Option<Block<'a>> {
        if block.is_last_phys_block() {
            return None;
        }

        let distance = usize::from(block.usable_size()) + usize::from(UsedBlock::HEADER_SIZE);
        let block = self.compute_offset(block.header_ptr().cast());
        let next = block.add(distance);
        Some(self.block_at(next))
    }

    pub unsafe fn prev_phys_block(&self, block: &FreeBlock<'a>) -> Option<Block<'a>> {
        let prev = block.get_prev_phys_block()?;
        Some(self.block_at(prev))
    }

    /// # Safety
    /// - `block` must be covered by `anchor`
    #[allow(private_bounds)]
    pub unsafe fn offset_of(&self, block: &impl OffsetOf) -> Offset {
        self.compute_offset(block.header_ptr().cast())
    }

    /// # Safety
    /// - `ptr` must lie ahead of the anchor
    /// - `ptr` must lie within `1 << 18` bytes of the anchor
    unsafe fn compute_offset(&self, ptr: NonNull<u32>) -> Offset {
        let anchor = self.ptr.as_ptr().addr();
        let offset = ptr.as_ptr().addr().wrapping_sub(anchor);
        unsafe { Offset::compress(offset) }
    }

    /// # Safety
    /// - `offset` must have been produced using `compute_offset` on this anchor
    unsafe fn resolve_offset(&self, offset: Offset) -> NonNull<u32> {
        NonNull::new_unchecked(self.ptr.as_ptr().add(offset.get()))
    }

    unsafe fn with_addr(&self, addr: usize) -> NonNull<u32> {
        NonNull::new_unchecked(self.ptr.as_ptr().with_addr(addr))
    }
}

impl<'a> FreeBlock<'a> {
    pub unsafe fn into_used(self, anchor: Anchor<'a>) -> &'a mut [MaybeUninit<u32>] {
        let size = usize::from(self.usable_size());

        self.header_ptr().cast::<Header>().as_ref().set_free(false);

        let data_ptr = anchor
            .with_addr(
                self.header_addr()
                    .wrapping_add(UsedBlock::HEADER_SIZE.into()),
            )
            .as_ptr();

        core::slice::from_raw_parts_mut(data_ptr.cast(), size >> consts::BLOCK_ALIGN_LOG2)
    }
}

impl<'a> UsedBlock<'a> {
    pub unsafe fn into_free(self, anchor: Anchor<'a>) -> FreeBlock<'a> {
        self.header_ptr().cast::<Header>().as_ref().set_free(true);

        let free = FreeBlock::from_ptr(anchor.with_addr(self.header_addr()).cast());
        free.clear_next_free();
        free.clear_prev_free();
        free
    }
}

impl<'a> Block<'a> {
    pub(crate) unsafe fn try_into_free(self, anchor: Anchor<'a>) -> Option<FreeBlock<'a>> {
        if self.is_free() {
            Some(FreeBlock::from_ptr(
                anchor.with_addr(self.header_addr()).cast(),
            ))
        } else {
            None
        }
    }
}

trait NextPhysBlock {
    fn header_ptr(&self) -> NonNull<Header>;
    fn is_last_phys_block(&self) -> bool;
    fn usable_size(&self) -> u16;
}

trait OffsetOf {
    fn header_ptr(&self) -> NonNull<Header>;
}

impl OffsetOf for FreeBlock<'_> {
    fn header_ptr(&self) -> NonNull<Header> {
        Self::header_ptr(self).cast()
    }
}

impl OffsetOf for Block<'_> {
    fn header_ptr(&self) -> NonNull<Header> {
        Self::header_ptr(self).cast()
    }
}

impl NextPhysBlock for Block<'_> {
    fn is_last_phys_block(&self) -> bool {
        Self::is_last_phys_block(self)
    }

    fn usable_size(&self) -> u16 {
        Self::usable_size(self)
    }

    fn header_ptr(&self) -> NonNull<Header> {
        Self::header_ptr(self)
    }
}

impl NextPhysBlock for FreeBlock<'_> {
    fn is_last_phys_block(&self) -> bool {
        Self::is_last_phys_block(self)
    }

    fn usable_size(&self) -> u16 {
        Self::usable_size(self)
    }

    fn header_ptr(&self) -> NonNull<Header> {
        Self::header_ptr(self).cast()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn furthest_free_block_into_used() {
        let mut memory = [MaybeUninit::uninit(); 65 * 1024];
        let anchor = Anchor::new(&mut memory[..]);
        unsafe {
            let free = anchor.create_free_block(Offset::max(), 4, true, None);
            let used = free.into_used(anchor);
            used.iter_mut().for_each(|mu| {
                mu.write(!0);
            });
        }
    }
}
