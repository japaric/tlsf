#![no_main]
#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec;
use core::mem::MaybeUninit;

use defmt::*;
use {defmt_rtt as _, panic_probe as _};

const NUM_WORDS: usize = 1024 / 4;

#[derive(Format)]
struct Zst;

#[cortex_m_rt::entry]
fn main() -> ! {
    static mut MEMORY: [MaybeUninit<u32>; NUM_WORDS] = [MaybeUninit::uninit(); NUM_WORDS];

    heap::initialize(MEMORY);

    let xs = vec![1, 2, 3];
    info!("easy as {}", xs.as_slice());
    drop(xs);

    info!("boxed {}", *Box::new(Zst));

    thumbv7m::exit()
}

mod heap {
    use core::alloc::{GlobalAlloc, Layout};
    use core::mem::MaybeUninit;
    use core::ptr::{self, NonNull};

    use defmt::*;
    use spin::mutex::SpinMutex;
    use tlsf::Tlsf;

    #[global_allocator]
    static HEAP: Heap = Heap {
        inner: SpinMutex::new(Tlsf::empty()),
    };

    struct Heap {
        inner: SpinMutex<Tlsf<'static, 2>>,
    }

    pub fn initialize(memory: &'static mut [MaybeUninit<u32>]) {
        HEAP.inner.lock().initialize(memory)
    }

    unsafe impl GlobalAlloc for Heap {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let mut tlsf = self.inner.lock();

            log_stats(&tlsf);

            let ptr = tlsf
                .memalign(layout)
                .map(|nn| nn.as_mut_ptr().cast())
                .unwrap_or(ptr::null_mut());

            debug!(
                "alloc(Layout {{ size: {}, align: {} }} -> {})",
                layout.size(),
                layout.align(),
                ptr
            );

            log_stats(&tlsf);

            ptr
        }

        unsafe fn dealloc(&self, ptr: *mut u8, _: Layout) {
            if let Some(nn) = NonNull::new(ptr) {
                let mut tlsf = self.inner.lock();

                log_stats(&tlsf);

                debug!("free({})", nn);

                tlsf.free(nn.cast());

                log_stats(&tlsf);
            }
        }
    }

    fn log_stats(tlsf: &Tlsf<2>) {
        let mut total_used = 0;
        let mut used_count = 0;
        let mut total_free = 0;
        let mut free_count = 0;
        for block in tlsf.blocks() {
            if block.is_free() {
                free_count += 1;
                total_free += block.usable_size();
            } else {
                used_count += 1;
                total_used += block.usable_size();
            }
        }

        trace!(
            "{}B of used memory across {} blocks; {}B of free memory across {} blocks",
            total_used,
            used_count,
            total_free,
            free_count
        );
    }

    unsafe impl Sync for Heap {}
}
