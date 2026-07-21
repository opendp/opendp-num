use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicUsize, Ordering},
};

use dashu::float::{FBig, round::mode::Up};

struct CountingAllocator;

static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

fn add_allocation(size: usize) {
    let current = CURRENT.fetch_add(size, Ordering::Relaxed) + size;
    PEAK.fetch_max(current, Ordering::Relaxed);
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            add_allocation(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        CURRENT.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, old: Layout, new_size: usize) -> *mut u8 {
        let new_pointer = unsafe { System.realloc(pointer, old, new_size) };
        if !new_pointer.is_null() {
            if new_size >= old.size() {
                add_allocation(new_size - old.size());
            } else {
                CURRENT.fetch_sub(old.size() - new_size, Ordering::Relaxed);
            }
        }
        new_pointer
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn main() {
    fn measure(value: f64) -> usize {
        let input = FBig::<Up>::try_from(value)
            .unwrap()
            .with_precision(2)
            .value();
        PEAK.store(CURRENT.load(Ordering::Relaxed), Ordering::Relaxed);
        let result = input.exp_m1();
        assert!(!result.repr().is_infinite());
        PEAK.load(Ordering::Relaxed)
    }

    let positive_peak = measure(100_000_000.0);
    let negative_peak = measure(-100_000_000.0);

    if cfg!(debug_assertions) {
        assert!(
            positive_peak > 10 * 1024 * 1024,
            "positive debug peak was only {positive_peak} bytes"
        );
        assert!(
            negative_peak > 10 * 1024 * 1024,
            "negative debug peak was only {negative_peak} bytes"
        );
    } else {
        assert!(
            positive_peak < 1024 * 1024,
            "positive release peak was {positive_peak} bytes"
        );
        assert!(
            negative_peak < 1024 * 1024,
            "negative release peak was {negative_peak} bytes"
        );
    }
    println!(
        "DASHU-027 reproduced: exp_m1(±1e8) profile={} positive_peak_heap_bytes={positive_peak} negative_peak_heap_bytes={negative_peak}",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
}
