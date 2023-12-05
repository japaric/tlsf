#![no_main]
#![no_std]

use core::alloc::Layout;
use core::array;
use core::mem::MaybeUninit;
use core::ptr::NonNull;

use cortex_m::peripheral::DWT;
use cortex_m::Peripherals;
use defmt::*;
use rand_core::{RngCore, SeedableRng};
use rand_xorshift::XorShiftRng;
use tlsf::Tlsf;
use {defmt_rtt as _, panic_probe as _};

const MAX_ALLOC_SIZE: usize = 124;
const NUM_WORDS: usize = 64 * 1024 / 4;
const RNG_SEED: u128 = 244905504285122192707088002030163722993;

#[cortex_m_rt::entry]
fn main() -> ! {
    info!("START");

    let mut periph = Peripherals::take().unwrap();
    periph.DCB.enable_trace();
    periph.DWT.set_cycle_count(0);
    periph.DWT.enable_cycle_counter();

    let mut tlsf = Tlsf::<2>::empty();
    tlsf.initialize(unsafe { &mut MEMORY });

    let mut rng = XorShiftRng::from_seed(RNG_SEED.to_le_bytes());

    let min_layout = Layout::new::<u8>();
    let mut allocs: [_; 2 * 1024] = array::from_fn(|_| None);
    let mut count = 0;
    loop {
        let size = (rng.next_u32() as usize) % MAX_ALLOC_SIZE;
        let align = 1 << (rng.next_u32() as u8 % 6);
        let layout = Layout::from_size_align(size, align).unwrap();

        let before = DWT::cycle_count();
        let mut res = tlsf.memalign(layout);
        let after = DWT::cycle_count();

        let elapsed = after.wrapping_sub(before);
        if let Some(alloc) = res.take() {
            allocs[count] = Some(alloc);
            count += 1;
            log_ok(elapsed);
        } else {
            log_fail(elapsed);

            // the preceding `alloc` can fail due to alignment requirements; do the smallest
            // allocation possible instead which should always work as long as we are not OOM
            let before = DWT::cycle_count();
            res = tlsf.memalign(min_layout);
            let after = DWT::cycle_count();

            let elapsed = after.wrapping_sub(before);
            if let Some(alloc) = res.take() {
                allocs[count] = Some(alloc);
                count += 1;
                log_ok(elapsed)
            } else {
                log_fail(elapsed);

                break;
            }
        }
    }

    allocs[..count].reverse();

    for opt in &mut allocs[..count] {
        let alloc = opt.take().unwrap();

        let before = DWT::cycle_count();
        unsafe { tlsf.free(NonNull::from(alloc).cast()) };
        let after = DWT::cycle_count();

        info!("FREE {}", after.wrapping_sub(before));
    }

    info!("EXIT");
    thumbv7m::exit()
}

fn log_ok(elapsed: u32) {
    info!("MEMALIGN OK {}", elapsed)
}

fn log_fail(elapsed: u32) {
    info!("MEMALIGN FAIL {}", elapsed)
}

#[link_section = ".uninit.0"]
static mut MEMORY: [MaybeUninit<u32>; NUM_WORDS] = [MaybeUninit::uninit(); NUM_WORDS];
