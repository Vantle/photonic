use serde::Serialize;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[derive(Default)]
pub struct Allocator {
    allocation: AtomicUsize,
    reallocation: AtomicUsize,
    deallocation: AtomicUsize,
    allocated: AtomicUsize,
    released: AtomicUsize,
    retained: AtomicUsize,
    peak: AtomicUsize,
}

impl Allocator {
    pub const fn new() -> Self {
        Self {
            allocation: AtomicUsize::new(0),
            reallocation: AtomicUsize::new(0),
            deallocation: AtomicUsize::new(0),
            allocated: AtomicUsize::new(0),
            released: AtomicUsize::new(0),
            retained: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
        }
    }

    fn grow(&self, size: usize) {
        self.allocated.fetch_add(size, Ordering::Relaxed);
        let retained = self.retained.fetch_add(size, Ordering::Relaxed);
        self.peak
            .fetch_max(retained.wrapping_add(size), Ordering::Relaxed);
    }

    fn shrink(&self, size: usize) {
        self.released.fetch_add(size, Ordering::Relaxed);
        self.retained.fetch_sub(size, Ordering::Relaxed);
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            allocation: self.allocation.load(Ordering::Relaxed),
            reallocation: self.reallocation.load(Ordering::Relaxed),
            deallocation: self.deallocation.load(Ordering::Relaxed),
            allocated: self.allocated.load(Ordering::Relaxed),
            released: self.released.load(Ordering::Relaxed),
            retained: self.retained.load(Ordering::Relaxed),
            peak: self.peak.load(Ordering::Relaxed),
        }
    }
}

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let address = unsafe { System.alloc(layout) };
        if !address.is_null() {
            self.allocation.fetch_add(1, Ordering::Relaxed);
            self.grow(layout.size());
        }
        address
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let address = unsafe { System.alloc_zeroed(layout) };
        if !address.is_null() {
            self.allocation.fetch_add(1, Ordering::Relaxed);
            self.grow(layout.size());
        }
        address
    }

    unsafe fn realloc(&self, address: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let address = unsafe { System.realloc(address, layout, size) };
        if !address.is_null() {
            self.reallocation.fetch_add(1, Ordering::Relaxed);
            if size >= layout.size() {
                self.grow(size - layout.size());
            } else {
                self.shrink(layout.size() - size);
            }
        }
        address
    }

    unsafe fn dealloc(&self, address: *mut u8, layout: Layout) {
        self.deallocation.fetch_add(1, Ordering::Relaxed);
        self.shrink(layout.size());
        unsafe { System.dealloc(address, layout) };
    }
}

struct Snapshot {
    allocation: usize,
    reallocation: usize,
    deallocation: usize,
    allocated: usize,
    released: usize,
    retained: usize,
    peak: usize,
}

#[derive(Serialize)]
pub struct Measurement {
    pub allocation: usize,
    pub reallocation: usize,
    pub deallocation: usize,
    pub allocated: usize,
    pub released: usize,
    pub retained: i128,
    pub peak: usize,
}

#[global_allocator]
static ALLOCATOR: Allocator = Allocator::new();
static ACTIVE: AtomicBool = AtomicBool::new(false);

struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {
        ACTIVE.store(false, Ordering::Relaxed);
    }
}

pub fn measure<Value>(run: impl FnOnce() -> Value) -> (Value, Measurement) {
    assert!(
        !ACTIVE.swap(true, Ordering::Relaxed),
        "overlapping allocation measurement"
    );
    let guard = Guard;
    let before = ALLOCATOR.snapshot();
    ALLOCATOR.peak.store(before.retained, Ordering::Relaxed);
    let value = run();
    let after = ALLOCATOR.snapshot();
    drop(guard);
    (
        value,
        Measurement {
            allocation: after.allocation.wrapping_sub(before.allocation),
            reallocation: after.reallocation.wrapping_sub(before.reallocation),
            deallocation: after.deallocation.wrapping_sub(before.deallocation),
            allocated: after.allocated.wrapping_sub(before.allocated),
            released: after.released.wrapping_sub(before.released),
            retained: after.retained as i128 - before.retained as i128,
            peak: after.peak.saturating_sub(before.retained),
        },
    )
}

#[cfg(test)]
#[path = "test/allocation.rs"]
mod test;
