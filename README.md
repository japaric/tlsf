An implementation of the Two-Level Segregated Fit (TLSF) allocator with optimized memory
footprint

# Design features

- The `alloc` and `dealloc` [^1] operations execute in bounded constant time (`O(1)`)
- Good-fit strategy
- Immediate coalescing adds predictability and reduces fragmentation

For more details check the papers linked in the 'References' section

[^1]: in the worst case scenario, the `realloc` operation involves a `memcpy` operation, which executes in linear time (`O(N)`)

# Implementation features

- Reduced memory footprint compared to the original TLSF thanks to pointer compression
  (`usize` -> `u16`)
- Free of panicking branches when optimized, even when debug-assertions are enabled
- Adheres to strict provenance as checked by `miri`

# Rejected features

- A `GlobalAlloc` implementation.

Rationale: it can be implemented on top of this library and every implementation needs to make
app-specific decisions like synchronization and whether to "forbid" `realloc`-like operations which
don't have bounded execution time by always triggering an OOM for them.

Those decisions are best left to the application author.

# Limitations

- Can manage only up to 256 KiB of _contiguous_ memory
- Can only allocate sizes of up to 62 KiB

# Examples

The allocator manages mutably borrowed memory; the memory can even be stack allocated.

```
use core::alloc::Layout;
use core::mem::MaybeUninit;

use tlsf::Tlsf;

let mut tlsf = Tlsf::<1>::empty();
let mut memory = [MaybeUninit::uninit(); 256];
tlsf.initialize(&mut memory);

let alloc: &mut [MaybeUninit<u32>] = tlsf.memalign(Layout::new::<u32>()).unwrap();
assert!(alloc.len() >= 1);
alloc.iter_mut().for_each(|mu| { mu.write(42); });
```

The allocator tracks the lifetime of the initial memory pool and allocations cannot outlive it. This
code does not compile

```compile_fail
use core::alloc::Layout;
use core::mem::MaybeUninit;

use tlsf::Tlsf;

let mut tlsf = Tlsf::<1>::empty();

{
   let memory = [MaybeUninit::uninit(); 256];
   tlsf.initialize(&mut memory); //~ error: `memory` does not live long enough
   // `memory` goes out of scope here
}

let alloc = tlsf.memalign(Layout::new::<u64>());
```

Due to this lifetime constraint, usage with `#[global_allocator]` requires that the initial memory
pool has `'static` lifetime. An example `GlobalAlloc` implementation can be found in the `thumbv7em`
directory of this project's repository.

# Parameters

The TLSF allocator has 2 parameters: FL and SL (see linked paper for further details). This
implementation hard codes SL to `16`. FL can controlled via the `FLL` type parameter of the `Tlsf`
type. The table below shows the possible values of `FLL` and its effect on the allocator

| `FLL` | FL  | `MAX_ALLOC_SIZE` | `HEADER_SIZE` |
| ----- | --- | ---------------- | ------------- |
| 1     | 6   | 60 B             | 36 B          |
| 2     | 7   | 124 B            | 72 B          |
| 3     | 8   | 248 B            | 104 B         |
| 4     | 9   | 496 B            | 140 B         |
| 5     | 10  | 992 B            | 172 B         |
| 6     | 11  | 1,984 B          | 208 B         |
| 7     | 12  | 3,968 B          | 240 B         |
| 8     | 13  | 7,936 B          | 276 B         |
| 9     | 14  | 15,872 B         | 308 B         |
| 10    | 15  | 31,744 B         | 344 B         |
| 11    | 16  | 63,488 B         | 376 B         |

Requesting more than `MAX_ALLOC_SIZE` bytes of memory from the allocator will always result in an
OOM condition. Note that the effective value of `MAX_ALLOC_SIZE` is reduced when alignments greater
than 4 are requested via `memalign` due to potential padding needed to meet the alignment
requirement.

`HEADER_SIZE` is the fixed memory overhead of the allocator. There's a 4 or 8 byte of overhead for
each memory block managed by the allocator.

# Performance

Benchmark configuration

- rustc: 1.74.0
- target: `thumbv7em-none-eabi`
- profile: release (opt-level=3, lto='fat', codegen-units=1)
- FLL: 2

## Binary ("Flash") size

~1,650 B (`.text` section)

Measured using

```ignore
#![no_main]
#![no_std]

#[no_mangle]
fn _start() -> [usize; 3] {
    [
        Tlsf::<2>::free as usize,
        Tlsf::<2>::initialize as usize,
        Tlsf::<2>::memalign as usize,
    ]
}
```

```text
$ size -A binary
section           size     addr
.ARM.exidx          16    65748
.text             1650   131300
.debug_aranges       0        0
```

## Execution time

workload: N random-sized (< `MAX_ALLOC_SIZE`) allocations with random alignments (<= 32 B) until OOM
followed by N deallocations in reverse order

| Operation         | min | max |
| ----------------- | --- | --- |
| `memalign` (ALL)  | 66  | 407 |
| `memalign` (FAIL) | 66  | 125 |
| `memalign` (OK)   | 193 | 407 |
| `free`            | 96  | 337 |

"FAIL" indicates that `memalign` returned `None`; "OK" indicates that it returned `Some`; "ALL"
accounts for both cases.

![][histograms]
![](/images/histograms.svg)

The code used to make the measurements can be found in the `thumbv7em` directory of the project's
repository.

# References

The design of the TLSF allocator is described and discussed in the following papers

- "A constant-time dynamic storage allocator for real-time systems."
- "Implementation of a constant time dynamic storage allocator."
- "TLSF: A new dynamic memory allocator for real-time systems."

which are available at <http://www.gii.upv.es/tlsf/main/docs.html>
