use crate::failure::Failure;
use crate::runtime::{Command, Device, Kernel, Memory};

const SOURCE: &str = include_str!("kernel.metal");
pub const TILE: usize = 32;

pub struct Pipeline {
    pub embed: Kernel,
    pub norm: Kernel,
    pub bias: Kernel,
    pub activate: Kernel,
    pub attend: Kernel,
    pub gather: Kernel,
    pub point: Kernel,
    pub clear: Kernel,
    pub derive: Kernel,
    pub column: Kernel,
    pub unnorm: Kernel,
    pub renorm: Kernel,
    pub unattend: Kernel,
    pub release: Kernel,
    pub spread: Kernel,
    pub unpoint: Kernel,
    pub unembed: Kernel,
    pub square: Kernel,
    pub adam: Kernel,
}

impl Pipeline {
    pub fn new(device: &Device, dimension: usize) -> Result<Self, Failure> {
        let library = device.library(&format!(
            "#define TILE {TILE}\n#define DIMENSION {dimension}\n{SOURCE}"
        ))?;
        let kernel = |entry: &str| device.kernel(&library, entry);
        Ok(Self {
            embed: kernel("embed")?,
            norm: kernel("norm")?,
            bias: kernel("bias")?,
            activate: kernel("activate")?,
            attend: kernel("attend")?,
            gather: kernel("gather")?,
            point: kernel("point")?,
            clear: kernel("clear")?,
            derive: kernel("derive")?,
            column: kernel("column")?,
            unnorm: kernel("unnorm")?,
            renorm: kernel("renorm")?,
            unattend: kernel("unattend")?,
            release: kernel("release")?,
            spread: kernel("spread")?,
            unpoint: kernel("unpoint")?,
            unembed: kernel("unembed")?,
            square: kernel("square")?,
            adam: kernel("adam")?,
        })
    }
}

fn group(kernel: &Kernel, thread: [usize; 3]) -> [usize; 3] {
    if thread[1] == 1 {
        return [kernel.capacity().min(thread[0]).max(1), 1, 1];
    }
    let width = kernel.width().min(thread[0]).max(1);
    let height = (kernel.capacity() / width).min(thread[1]).max(1);
    [width, height, 1]
}

fn constant(shape: &[usize]) -> Vec<u8> {
    shape
        .iter()
        .flat_map(|value| (*value as u32).to_le_bytes())
        .collect()
}

pub fn dispatch<'memory>(
    command: &mut Command<'memory>,
    kernel: &Kernel,
    memory: &[&'memory Memory],
    shape: &[usize],
    thread: [usize; 3],
) {
    command.dispatch(
        kernel,
        memory,
        &constant(shape),
        thread,
        group(kernel, thread),
    );
}

pub fn tiled<'memory>(
    command: &mut Command<'memory>,
    kernel: &Kernel,
    memory: &[&'memory Memory],
    shape: &[usize],
    tile: [usize; 2],
) {
    command.dispatch(
        kernel,
        memory,
        &constant(shape),
        [tile[0] * TILE, tile[1], 1],
        [TILE, 1, 1],
    );
}
