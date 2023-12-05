use core::ptr::NonNull;
use core::{fmt, mem};

use super::common::Header;

#[derive(Debug)]
pub struct UsedBlock<'a> {
    header: &'a UsedHeader,
}

impl<'a> UsedBlock<'a> {
    pub const HEADER_SIZE: u8 = mem::size_of::<UsedHeader>() as u8;

    pub(super) unsafe fn from_ptr(ptr: NonNull<UsedHeader>) -> Self {
        Self {
            header: ptr.as_ref(),
        }
    }

    pub(super) fn header_ptr(&self) -> NonNull<UsedHeader> {
        NonNull::from(self.header)
    }

    pub(super) fn header_addr(&self) -> usize {
        self.header as *const UsedHeader as usize
    }
}

#[repr(C)]
#[repr(align(4))]
pub struct UsedHeader {
    common: Header,
}

impl fmt::Debug for UsedHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Header")
            .field("usable_size", &self.common.usable_size())
            .field("is_last_phys_block", &self.common.is_last_phys_block())
            .field("prev_phys_block", &self.common.get_prev_phys_block())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // check the value specified in the user-facing API docs
    #[test]
    fn header_size() {
        assert_eq!(4, mem::size_of::<UsedHeader>());
    }
}
