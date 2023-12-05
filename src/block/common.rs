use core::cell::Cell;
use core::fmt;
use core::ptr::NonNull;

use super::Offset;
#[cfg(any(fuzzing, test))]
use crate::consts;
#[allow(unused_imports)] // used by API docs
use crate::Tlsf;

/// A memory block managed by a [`Tlsf`] allocator
///
/// This object only grants access to the metadata of the memory block and does not overlap with
/// memory returned by [`Tlsf::memalign`]
pub struct Block<'a> {
    header: &'a Header,
}

impl Block<'_> {
    /// Returns the usable size of the memory block in bytes
    pub fn usable_size(&self) -> u16 {
        self.header.usable_size()
    }

    /// Returns `true` if the block is currently "free"
    ///
    /// A free block is "owned" by the allocator (as in the allocator has unique access to it) and
    /// can be used to fulfill allocation requests
    pub fn is_free(&self) -> bool {
        self.header.is_free()
    }

    /// Returns `true` if the block is currently in "use"
    ///
    /// A used block has been lent (e.g. via [`Tlsf::memalign`]) to a caller and the caller has
    /// unique access to it
    pub fn is_used(&self) -> bool {
        !self.header.is_free()
    }

    pub(super) unsafe fn from_ptr(ptr: NonNull<Header>) -> Self {
        Self {
            header: ptr.as_ref(),
        }
    }

    pub(super) fn header_addr(&self) -> usize {
        self.header as *const Header as usize
    }

    #[cfg(test)]
    pub(crate) fn total_size(&self) -> usize {
        use super::UsedBlock;

        usize::from(self.usable_size()) + usize::from(UsedBlock::HEADER_SIZE)
    }
    pub(super) fn is_last_phys_block(&self) -> bool {
        self.header.is_last_phys_block()
    }

    pub(crate) fn set_prev_phys_block(&self, offset: Offset) {
        self.header.set_prev_phys_block(offset);
    }

    pub(super) fn header_ptr(&self) -> NonNull<Header> {
        NonNull::from(self.header)
    }
}

impl fmt::Debug for Block<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Block")
            .field("is_free", &self.is_free())
            .field("usable_size", &self.usable_size())
            .finish()
    }
}
#[repr(C)]
#[repr(align(4))]
pub(super) struct Header {
    size_free_last: Cell<u16>,
    prev_phys_block: Cell<Option<Offset>>,
}

impl Header {
    const FREE_BIT: u16 = 1 << 0;
    const LAST_PHYS_BLOCK_BIT: u16 = 1 << 1;
    const SIZE_MASK: u16 = !(Self::FREE_BIT | Self::LAST_PHYS_BLOCK_BIT);

    pub fn new(
        usable_size: u16,
        is_free: bool,
        is_last_phys_block: bool,
        prev_phys_block: Option<Offset>,
    ) -> Self {
        #[cfg(any(fuzzing, test))]
        debug_assert_eq!(0, usable_size % u16::from(consts::BLOCK_ALIGN));

        let mut size_free_last = usable_size;
        if is_free {
            size_free_last |= Self::FREE_BIT;
        }
        if is_last_phys_block {
            size_free_last |= Self::LAST_PHYS_BLOCK_BIT;
        }

        Self {
            size_free_last: Cell::new(size_free_last),
            prev_phys_block: Cell::new(prev_phys_block),
        }
    }

    pub fn usable_size(&self) -> u16 {
        self.size_free_last.get() & Self::SIZE_MASK
    }

    pub fn is_free(&self) -> bool {
        self.size_free_last.get() & Self::FREE_BIT != 0
    }

    pub fn set_free(&self, free: bool) {
        update(&self.size_free_last, |old| {
            if free {
                old | Self::FREE_BIT
            } else {
                old & !Self::FREE_BIT
            }
        });
    }

    pub unsafe fn set_usable_size(&self, new_usable_size: u16) {
        #[cfg(any(fuzzing, test))]
        debug_assert_eq!(0, new_usable_size % 4);

        update(&self.size_free_last, |old| {
            let free_last = old & !Self::SIZE_MASK;
            new_usable_size | free_last
        });
    }

    pub fn is_last_phys_block(&self) -> bool {
        self.size_free_last.get() & Self::LAST_PHYS_BLOCK_BIT != 0
    }

    pub fn set_last_phys_block(&self) {
        update(&self.size_free_last, |old| old | Self::LAST_PHYS_BLOCK_BIT);
    }

    pub fn clear_last_phys_block(&self) {
        update(&self.size_free_last, |old| old & !Self::LAST_PHYS_BLOCK_BIT);
    }

    pub fn get_prev_phys_block(&self) -> Option<Offset> {
        self.prev_phys_block.get()
    }

    pub fn set_prev_phys_block(&self, prev_phys_block: Offset) {
        self.prev_phys_block.set(Some(prev_phys_block));
    }
}

fn update<T>(cell: &Cell<T>, f: impl FnOnce(T) -> T)
where
    T: Copy,
{
    let old = cell.get();
    let new = f(old);
    cell.set(new);
}
