use core::cell::Cell;
use core::ptr::NonNull;
use core::{fmt, mem};

use super::common::Header;
use super::used::UsedHeader;
use super::{Offset, UsedBlock};

#[derive(Debug)]
#[cfg_attr(test, derive(Clone))]
pub struct FreeBlock<'a> {
    header: &'a FreeHeader,
}

impl<'a> FreeBlock<'a> {
    pub const HEADER_SIZE: u8 = mem::size_of::<FreeHeader>() as u8;

    pub(super) unsafe fn new(
        ptr: NonNull<FreeHeader>,
        usable_size: u16,
        is_last_phys_block: bool,
        prev_phys_block: Option<Offset>,
    ) -> Self {
        #[cfg(any(fuzzing, test))]
        debug_assert!(
            usize::from(usable_size) + usize::from(UsedBlock::HEADER_SIZE)
                >= usize::from(FreeBlock::HEADER_SIZE)
        );

        // fully initialize the whole header
        unsafe {
            ptr.as_ptr().write(FreeHeader::new(
                usable_size,
                is_last_phys_block,
                prev_phys_block,
            ))
        }
        Self {
            header: ptr.as_ref(),
        }
    }

    pub(super) unsafe fn from_ptr(ptr: NonNull<FreeHeader>) -> Self {
        Self {
            header: ptr.as_ref(),
        }
    }

    pub(super) fn header_ptr(&self) -> NonNull<FreeHeader> {
        NonNull::from(self.header)
    }

    pub(super) fn header_addr(&self) -> usize {
        self.header as *const FreeHeader as usize
    }

    pub unsafe fn resize(&self, new_usable_size: u16) {
        self.header.common.set_usable_size(new_usable_size);
    }

    pub fn total_size(&self) -> usize {
        usize::from(self.usable_size()) + usize::from(UsedBlock::HEADER_SIZE)
    }

    pub fn body_ptr(&self) -> NonNull<u32> {
        unsafe {
            NonNull::new_unchecked(self.header_ptr().cast::<UsedHeader>().as_ptr().add(1)).cast()
        }
    }

    // header manipulation
    pub fn usable_size(&self) -> u16 {
        self.header.common.usable_size()
    }

    pub fn is_last_phys_block(&self) -> bool {
        self.header.common.is_last_phys_block()
    }

    pub fn set_last_phys_block(&self) {
        self.header.common.set_last_phys_block()
    }

    pub fn clear_last_phys_block(&self) {
        self.header.common.clear_last_phys_block()
    }

    pub fn get_prev_phys_block(&self) -> Option<Offset> {
        self.header.common.get_prev_phys_block()
    }

    // FreeBlock-specific header manipulation
    pub fn clear_next_free(&self) {
        self.header.next_free.set(None);
    }

    pub fn get_next_free(&self) -> Option<Offset> {
        self.header.next_free.get()
    }

    pub fn set_next_free(&self, offset: Offset) {
        self.header.next_free.set(Some(offset));
    }

    pub fn clear_prev_free(&self) {
        self.header.prev_free.set(None);
    }

    pub fn set_prev_free(&self, offset: Offset) {
        self.header.prev_free.set(Some(offset));
    }

    pub fn get_prev_free(&self) -> Option<Offset> {
        self.header.prev_free.get()
    }
}

#[repr(C)]
#[repr(align(4))]
pub struct FreeHeader {
    common: Header,
    next_free: Cell<Option<Offset>>,
    prev_free: Cell<Option<Offset>>,
}

impl FreeHeader {
    fn new(usable_size: u16, is_last_phys_block: bool, prev_phys_block: Option<Offset>) -> Self {
        Self {
            common: Header::new(usable_size, true, is_last_phys_block, prev_phys_block),
            next_free: Cell::new(None),
            prev_free: Cell::new(None),
        }
    }
}

impl fmt::Debug for FreeHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Header")
            .field("usable_size", &self.common.usable_size())
            .field("is_last_phys_block", &self.common.is_last_phys_block())
            .field("prev_phys_block", &self.common.get_prev_phys_block())
            .field("next_free", &self.next_free)
            .field("prev_free", &self.prev_free)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // check the value specified in the user-facing API docs
    #[test]
    fn header_size() {
        assert_eq!(8, mem::size_of::<FreeHeader>());
    }
}
