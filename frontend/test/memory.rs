use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Counter;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let address = unsafe { System.alloc(layout) };
        if !address.is_null() {
            let live = LIVE.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PEAK.fetch_max(live, Ordering::Relaxed);
        }
        address
    }

    unsafe fn dealloc(&self, address: *mut u8, layout: Layout) {
        unsafe { System.dealloc(address, layout) };
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
    }
}

#[global_allocator]
static COUNTER: Counter = Counter;

fn start() -> usize {
    let live = LIVE.load(Ordering::Relaxed);
    PEAK.store(live, Ordering::Relaxed);
    live
}

// Pest keeps two 40-byte tokens for every node while it parses, which bounds the peak from below;
// the tree and the lowered program must stay within a small multiple of the source beside it.
#[test]
fn memory() {
    for (source, tree, peak, program) in [
        ("A, ".repeat(1_000_000), 25, 100, 20),
        (format!("{}A", "A.".repeat(1_000_000)), 20, 80, 16),
        ("[A] B, ".repeat(300_000), 35, 140, 80),
    ] {
        let base = start();
        let parsed = frontend::parser::parse(&source).unwrap();
        let kept = LIVE.load(Ordering::Relaxed) - base;
        drop(parsed);
        let base = start();
        let lowered = frontend::lowering::parse(&source).unwrap();
        let highest = PEAK.load(Ordering::Relaxed) - base;
        let result = LIVE.load(Ordering::Relaxed) - base;
        drop(lowered);
        let length = source.len();
        assert!(kept <= tree * length, "tree {kept} for {length} bytes");
        assert!(
            highest <= peak * length,
            "peak {highest} for {length} bytes"
        );
        assert!(
            result <= program * length,
            "program {result} for {length} bytes"
        );
    }
}
