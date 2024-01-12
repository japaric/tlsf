#![no_main]

use core::num::NonZeroU16;

use libfuzzer_sys::fuzz_target;
use tlsf::{Memory, Tlsf};

fuzz_target!(|data: Vec<NonZeroU16>| {
    let mut tlsf = Tlsf::<11>::empty();
    let mut memory = Memory::new();
    tlsf.initialize(memory.bytes());

    for size in data {
        let size = size.into();
        tlsf.malloc(size);
    }
});
