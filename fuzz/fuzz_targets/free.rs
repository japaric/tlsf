#![no_main]

use core::alloc::Layout;
use core::mem;
use core::num::NonZeroU16;
use core::ptr::NonNull;

use libfuzzer_sys::arbitrary::Arbitrary;
use libfuzzer_sys::{arbitrary, fuzz_target};
use tlsf::{Memory, Tlsf};

fuzz_target!(|actions: Vec<Action>| {
    let mut tlsf = Tlsf::<11>::empty();
    let mut memory = Memory::new();
    tlsf.initialize(memory.bytes());

    let mut allocs = vec![];
    let mut allocated = 0;
    for action in actions {
        match action {
            Action::Malloc { size } => {
                if let Some(alloc) = tlsf.malloc(size) {
                    allocated += mem::size_of_val(alloc);

                    allocs.push(alloc);

                    check_stats(&tlsf, allocated, allocs.len());
                }
            }

            Action::Memalign { size, align_log } => {
                if let Some(align) = 1usize.checked_shl(align_log.into()) {
                    let size = size.into();

                    if let Ok(layout) = Layout::from_size_align(size, align) {
                        if let Some(alloc) = tlsf.memalign(layout) {
                            allocated += mem::size_of_val(alloc);

                            allocs.push(alloc);

                            check_stats(&tlsf, allocated, allocs.len());
                        }
                    }
                }
            }

            Action::Free { index } => {
                if !allocs.is_empty() && index < allocs.len() {
                    let alloc = allocs.remove(index);
                    allocated -= mem::size_of_val(alloc);

                    unsafe { tlsf.free(NonNull::from(alloc).cast()) }

                    check_stats(&tlsf, allocated, allocs.len());
                }
            }
        }
    }
});

// sanity check that the "statistics" API matches reality
fn check_stats(tlsf: &Tlsf<11>, allocated: usize, allocs_len: usize) {
    let mut count = 0;
    let mut used = 0;
    for block in tlsf.blocks() {
        if block.is_used() {
            used += usize::from(block.usable_size());
            count += 1;
        }
    }
    assert_eq!(allocs_len, count);
    assert_eq!(allocated, used);
}

#[derive(Arbitrary, Debug)]
enum Action {
    Free { index: usize },
    Malloc { size: NonZeroU16 },
    Memalign { size: u16, align_log: u8 },
}
