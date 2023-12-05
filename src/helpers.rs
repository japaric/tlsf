#![allow(missing_docs)] // "pub" API but only for internal use (e.g. fuzzing)

use core::alloc::Layout;
#[cfg(not(fuzzing))]
use core::mem;
use core::mem::MaybeUninit;
use core::slice;

#[cfg(not(fuzzing))]
use crate::block::{Anchor, FreeBlock, Offset};
use crate::consts;

#[cfg(not(fuzzing))]
pub struct FreeBlocks<'a> {
    pub anchor: Anchor<'a>,
    prev_phys_block: Option<Offset>,
    offset: Offset,
    remaining: usize,
    yielded_last: bool,
}

#[cfg(not(fuzzing))]
impl<'a> FreeBlocks<'a> {
    pub fn new(memory: &'a mut [MaybeUninit<u32>]) -> Self {
        let len = memory.len() * mem::size_of::<u32>();
        let anchor = Anchor::new(memory);
        Self {
            anchor,
            prev_phys_block: None,
            offset: unsafe { Offset::compress(4) },
            remaining: len - 4,
            yielded_last: false,
        }
    }

    pub fn next(&mut self, usable_size: u16, is_last_phys_block: bool) -> FreeBlock<'a> {
        assert!(!self.yielded_last, "already marked a block as the last one");

        let block = unsafe {
            self.anchor.create_free_block(
                self.offset,
                usable_size,
                is_last_phys_block,
                self.prev_phys_block,
            )
        };
        let total_size = block.total_size();
        self.prev_phys_block = Some(self.offset);
        self.offset = unsafe { self.offset.add(total_size) };
        self.remaining = self
            .remaining
            .checked_sub(total_size)
            .expect("exhausted remaining memory");

        self.yielded_last = is_last_phys_block;

        block
    }
}

#[cfg(not(fuzzing))]
impl Drop for FreeBlocks<'_> {
    fn drop(&mut self) {
        assert!(
            self.yielded_last,
            "did not mark a block as the last_phys_block"
        )
    }
}

pub struct Memory {
    ptr: *mut u8,
}

impl Memory {
    fn layout() -> Layout {
        Layout::from_size_align(consts::MAX_POOL_SIZE, 64 * 1024).unwrap()
    }

    pub fn new() -> Self {
        let ptr = unsafe { std::alloc::alloc(Self::layout()) };
        assert!(!ptr.is_null());
        Self { ptr }
    }

    pub fn bytes(&mut self) -> &mut [MaybeUninit<u32>] {
        unsafe { slice::from_raw_parts_mut(self.ptr.cast(), consts::MAX_POOL_SIZE / 4) }
    }
}

impl Drop for Memory {
    fn drop(&mut self) {
        unsafe { std::alloc::dealloc(self.ptr, Self::layout()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let _memory = Memory::new();
    }
}
