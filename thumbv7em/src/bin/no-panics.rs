#![no_main]
#![no_std]

use core::panic::PanicInfo;

use tlsf::Tlsf;

#[no_mangle]
fn _start() -> [usize; 3] {
    [
        Tlsf::<2>::free as usize,
        Tlsf::<2>::initialize as usize,
        Tlsf::<2>::memalign as usize,
    ]
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    extern "C" {
        #[link_name = "this program contains panicking branches"]
        fn panic() -> !;
    }
    unsafe { panic() }
}
