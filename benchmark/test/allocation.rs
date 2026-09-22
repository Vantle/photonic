use super::Allocator;
use std::alloc::{GlobalAlloc, Layout};

#[test]
fn storage() {
    let allocator = Allocator::new();
    let layout = Layout::from_size_align(64, 64).unwrap();
    let address = unsafe { allocator.alloc_zeroed(layout) };
    assert!(!address.is_null());
    assert_eq!(address as usize % layout.align(), 0);
    assert!(
        unsafe { std::slice::from_raw_parts(address, layout.size()) }
            .iter()
            .all(|value| *value == 0)
    );
    unsafe { address.write_bytes(37, layout.size()) };
    let address = unsafe { allocator.realloc(address, layout, 256) };
    assert!(!address.is_null());
    assert_eq!(address as usize % layout.align(), 0);
    assert!(
        unsafe { std::slice::from_raw_parts(address, layout.size()) }
            .iter()
            .all(|value| *value == 37)
    );
    let layout = Layout::from_size_align(256, 64).unwrap();
    let address = unsafe { allocator.realloc(address, layout, 32) };
    assert!(!address.is_null());
    assert_eq!(address as usize % layout.align(), 0);
    assert!(
        unsafe { std::slice::from_raw_parts(address, 32) }
            .iter()
            .all(|value| *value == 37)
    );
    unsafe { allocator.dealloc(address, Layout::from_size_align(32, 64).unwrap()) };
    let snapshot = allocator.snapshot();
    assert_eq!(snapshot.allocation, 1);
    assert_eq!(snapshot.reallocation, 2);
    assert_eq!(snapshot.deallocation, 1);
    assert_eq!(snapshot.allocated, 256);
    assert_eq!(snapshot.released, 256);
    assert_eq!(snapshot.retained, 0);
    assert_eq!(snapshot.peak, 256);
}

#[test]
fn overlap() {
    let allocator = Allocator::new();
    let layout = Layout::from_size_align(73, 8).unwrap();
    let first = unsafe { allocator.alloc(layout) };
    assert!(!first.is_null());
    let second = unsafe { allocator.alloc(layout) };
    assert!(!second.is_null());
    assert_ne!(first, second);
    unsafe { allocator.dealloc(first, layout) };
    let snapshot = allocator.snapshot();
    assert_eq!(snapshot.retained, 73);
    assert_eq!(snapshot.peak, 146);
    unsafe { allocator.dealloc(second, layout) };
    let snapshot = allocator.snapshot();
    assert_eq!(snapshot.allocation, 2);
    assert_eq!(snapshot.reallocation, 0);
    assert_eq!(snapshot.deallocation, 2);
    assert_eq!(snapshot.allocated, snapshot.released);
    assert_eq!(snapshot.retained, 0);
}

#[test]
fn scope() {
    assert!(std::panic::catch_unwind(|| super::measure(|| panic!("interrupted"))).is_err());
    let (value, _) = super::measure(|| {
        assert!(std::panic::catch_unwind(|| super::measure(|| ())).is_err());
        7
    });
    assert_eq!(value, 7);
    assert_eq!(super::measure(|| 11).0, 11);
}
