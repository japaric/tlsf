#![no_main]

use core::alloc::Layout;

use libfuzzer_sys::fuzz_target;
use tlsf::{Memory, Tlsf};

fuzz_target!(|data: Vec<(u16, u8)>| {
    let mut tlsf = Tlsf::<11>::empty();
    let mut memory = Memory::new();
    tlsf.initialize(memory.bytes());

    for (align, size) in data {
        let size = size.into();
        let align = 1usize.wrapping_shl(align.into());
        if let Ok(layout) = Layout::from_size_align(size, align) {
            tlsf.memalign(layout);
        }
    }
});
