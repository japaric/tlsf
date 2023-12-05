#![no_main]

use core::mem::MaybeUninit;

use libfuzzer_sys::arbitrary::Arbitrary;
use libfuzzer_sys::{arbitrary, fuzz_target};
use tlsf::Tlsf;

fuzz_target!(|length: Length| {
    let mut tlsf = Tlsf::<2>::empty();
    let len = usize::from(length.main) + usize::from(length.extra);
    let mut memory = vec![MaybeUninit::uninit(); len];
    tlsf.initialize(&mut memory);
});

#[derive(Arbitrary, Debug)]
struct Length {
    main: u16,
    extra: u8,
}
