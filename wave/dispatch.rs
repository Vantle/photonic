use metal::device::Kernel;

// The threads of a group for kernels that need no whole SIMD groups: as many as the kernel takes,
// and no more than there are threads.
pub fn group(kernel: &Kernel, thread: usize) -> [usize; 3] {
    [kernel.capacity().min(thread).max(1), 1, 1]
}

// The threads of a group for kernels that work in whole SIMD groups, at most as many groups as a
// SIMD group has lanes.
pub fn whole(kernel: &Kernel) -> usize {
    let lanes = kernel.width().max(1);
    let most = kernel.capacity().min(1024).min(32 * lanes);
    (most / lanes * lanes).max(lanes)
}
