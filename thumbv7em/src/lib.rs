#![no_std]

use cortex_m::asm;

pub fn exit() -> ! {
    loop {
        asm::bkpt()
    }
}
