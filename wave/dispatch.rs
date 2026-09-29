use crate::setting::BAND;
use metal::device::Kernel;

// The threads of a group for kernels that work in whole SIMD groups: one SIMD group sums the
// others' running sums, so a group holds at most as many SIMD groups as a SIMD group has lanes,
// and at most BAND, whose sums the kernels' scratch keeps.
pub fn whole(kernel: &Kernel) -> usize {
    let width = kernel.width().max(1);
    let most = kernel.capacity().min(BAND * width).min(width * width);
    (most / width * width).max(width)
}
