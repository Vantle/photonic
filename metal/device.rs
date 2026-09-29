#![allow(unsafe_code)]

use crate::failure::Failure;
use crate::operand::{Operand, Product};
use crate::plain::Plain;
use std::ffi::{CStr, c_char, c_void};
use std::marker::PhantomData;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
struct Object(*mut c_void);

impl Object {
    const NONE: Self = Self(std::ptr::null_mut());

    fn null(self) -> bool {
        self.0.is_null()
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct Size {
    width: u64,
    height: u64,
    depth: u64,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct Range {
    location: u64,
    length: u64,
}

#[link(name = "objc")]
unsafe extern "C" {
    fn objc_getClass(name: *const c_char) -> Object;
    fn sel_registerName(name: *const c_char) -> *mut c_void;
    fn objc_msgSend();
    fn objc_autoreleasePoolPush() -> *mut c_void;
    fn objc_autoreleasePoolPop(pool: *mut c_void);
    fn objc_release(object: Object);
}

unsafe extern "C" {
    fn dlopen(path: *const c_char, mode: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

const LAZY: i32 = 1;
const GLOBAL: i32 = 8;
const FLOAT: u32 = 0x1000_0000 | 32;
const ENCODING: u64 = 4;
const COMPLETED: u64 = 4;
const READ: u64 = 1;
const WRITE: u64 = 2;
// Metal 3's GPU family, the first whose kernels reach memory through device addresses.
const FAMILY: i64 = 5001;

// Every call goes through objc_msgSend cast to the exact signature of the selector it names,
// which is how the Objective-C runtime expects to be called from C.
macro_rules! send {
    ($receiver:expr, $selector:literal $(, $argument:expr => $kind:ty)* ; $result:ty) => {{
        let function: unsafe extern "C" fn(Object, *mut c_void $(, $kind)*) -> $result = unsafe {
            std::mem::transmute::<unsafe extern "C" fn(), _>(objc_msgSend)
        };
        let selector = unsafe { sel_registerName($selector.as_ptr()) };
        unsafe { function($receiver, selector $(, $argument)*) }
    }};
}

fn class(name: &CStr) -> Object {
    unsafe { objc_getClass(name.as_ptr()) }
}

fn text(string: Object) -> String {
    if string.null() {
        return String::new();
    }
    let pointer = send!(string, c"UTF8String"; *const c_char);
    if pointer.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(pointer) }
        .to_string_lossy()
        .into_owned()
}

fn describe(error: Object) -> String {
    if error.null() {
        return "no description".to_owned();
    }
    text(send!(error, c"localizedDescription"; Object))
}

struct Handle(Object);

// Handles own a +1 reference to devices, queues, buffers, libraries and pipeline states,
// which Metal documents as safe to use from any thread; command buffers and encoders stay
// inside one Command, whose raw-pointer marker makes it neither Send nor Sync.
unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe { objc_release(self.0) };
    }
}

impl Handle {
    fn new(object: Object) -> Option<Self> {
        (!object.null()).then_some(Self(object))
    }

    fn retain(object: Object) -> Option<Self> {
        (!object.null()).then(|| Self(send!(object, c"retain"; Object)))
    }
}

struct Pool(*mut c_void);

impl Pool {
    fn new() -> Self {
        Self(unsafe { objc_autoreleasePoolPush() })
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        unsafe { objc_autoreleasePoolPop(self.0) };
    }
}

fn string(value: &str) -> Handle {
    let allocated = send!(class(c"NSString"), c"alloc"; Object);
    Handle(send!(
        allocated,
        c"initWithBytes:length:encoding:",
        value.as_ptr().cast::<c_void>() => *const c_void,
        value.len() as u64 => u64,
        ENCODING => u64;
        Object
    ))
}

pub struct Device {
    handle: Handle,
    queue: Handle,
    name: String,
}

impl Device {
    pub fn open() -> Result<Self, Failure> {
        let metal = unsafe {
            dlopen(
                c"/System/Library/Frameworks/Metal.framework/Metal".as_ptr(),
                LAZY | GLOBAL,
            )
        };
        if metal.is_null() {
            return Err(Failure::Unavailable("Metal is not installed".to_owned()));
        }
        let shader = unsafe {
            dlopen(
                c"/System/Library/Frameworks/MetalPerformanceShaders.framework/MetalPerformanceShaders"
                    .as_ptr(),
                LAZY | GLOBAL,
            )
        };
        if shader.is_null() {
            return Err(Failure::Unavailable(
                "Metal Performance Shaders are not installed".to_owned(),
            ));
        }
        let symbol = unsafe { dlsym(metal, c"MTLCreateSystemDefaultDevice".as_ptr()) };
        if symbol.is_null() {
            return Err(Failure::Unavailable(
                "MTLCreateSystemDefaultDevice is missing".to_owned(),
            ));
        }
        let create: unsafe extern "C" fn() -> Object = unsafe { std::mem::transmute(symbol) };
        let handle = Handle::new(unsafe { create() })
            .ok_or_else(|| Failure::Unavailable("no Metal device".to_owned()))?;
        let queue = Handle::new(send!(handle.0, c"newCommandQueue"; Object))
            .ok_or_else(|| Failure::Unavailable("no command queue".to_owned()))?;
        let name = {
            let _pool = Pool::new();
            text(send!(handle.0, c"name"; Object))
        };
        Ok(Self {
            handle,
            queue,
            name,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    // Whether kernels on the device can reach memory through device addresses.
    pub fn addressing(&self) -> bool {
        send!(self.handle.0, c"supportsFamily:", FAMILY => i64; i8) != 0
    }

    pub fn memory<Element: Plain>(&self, count: usize) -> Result<Memory, Failure> {
        let mut memory = self.space::<Element>(count)?;
        memory.edit::<u32>().fill(0);
        Ok(memory)
    }

    // Memory left as the device hands it out, for kernels that write it before anything reads it;
    // large buffers skip the host's pass over every byte.
    pub fn space<Element: Plain>(&self, count: usize) -> Result<Memory, Failure> {
        let length = count * std::mem::size_of::<Element>();
        let handle = Handle::new(send!(
            self.handle.0,
            c"newBufferWithLength:options:",
            length.max(16) as u64 => u64,
            0 => u64;
            Object
        ))
        .ok_or(Failure::Allocation { length })?;
        Ok(Memory { handle, length })
    }

    pub fn upload<Element: Plain>(&self, value: &[Element]) -> Result<Memory, Failure> {
        let mut memory = self.space::<Element>(value.len())?;
        memory.edit::<Element>().copy_from_slice(value);
        Ok(memory)
    }

    pub fn library(&self, source: &str) -> Result<Library, Failure> {
        let _pool = Pool::new();
        let code = string(source);
        let mut error = Object::NONE;
        let result = send!(
            self.handle.0,
            c"newLibraryWithSource:options:error:",
            code.0 => Object,
            Object::NONE => Object,
            &raw mut error => *mut Object;
            Object
        );
        Handle::new(result)
            .map(|handle| Library { handle })
            .ok_or_else(|| Failure::Compile(describe(error)))
    }

    pub fn kernel(&self, library: &Library, entry: &str) -> Result<Kernel, Failure> {
        let _pool = Pool::new();
        let label = string(entry);
        let function = Handle::new(
            send!(library.handle.0, c"newFunctionWithName:", label.0 => Object; Object),
        )
        .ok_or_else(|| Failure::Kernel(format!("{entry} is missing")))?;
        let mut error = Object::NONE;
        let handle = Handle::new(send!(
            self.handle.0,
            c"newComputePipelineStateWithFunction:error:",
            function.0 => Object,
            &raw mut error => *mut Object;
            Object
        ))
        .ok_or_else(|| Failure::Kernel(format!("{entry}: {}", describe(error))))?;
        Ok(Kernel {
            width: send!(handle.0, c"threadExecutionWidth"; u64) as usize,
            capacity: send!(handle.0, c"maxTotalThreadsPerThreadgroup"; u64) as usize,
            handle,
        })
    }

    pub fn command(&self) -> Result<Command<'_>, Failure> {
        let _pool = Pool::new();
        let buffer = Handle::retain(send!(self.queue.0, c"commandBuffer"; Object))
            .ok_or_else(|| Failure::Execution("no command buffer".to_owned()))?;
        Ok(Command {
            device: self,
            buffer,
            encoder: None,
            retained: Vec::new(),
            reached: Vec::new(),
            local: PhantomData,
        })
    }
}

pub struct Memory {
    handle: Handle,
    length: usize,
}

// The buffers use shared storage, so contents points at length bytes that stay valid while
// the buffer lives, and the borrow of self keeps any Command that writes them from running.
impl Memory {
    // Where kernels find the memory when they reach it through an address rather than a binding.
    pub fn address(&self) -> u64 {
        send!(self.handle.0, c"gpuAddress"; u64)
    }

    pub fn length(&self) -> usize {
        self.length
    }

    pub fn view<Element: Plain>(&mut self) -> &[Element] {
        let pointer = send!(self.handle.0, c"contents"; *mut c_void);
        unsafe {
            std::slice::from_raw_parts(
                pointer.cast::<Element>(),
                self.length / std::mem::size_of::<Element>(),
            )
        }
    }

    pub fn edit<Element: Plain>(&mut self) -> &mut [Element] {
        let pointer = send!(self.handle.0, c"contents"; *mut c_void);
        unsafe {
            std::slice::from_raw_parts_mut(
                pointer.cast::<Element>(),
                self.length / std::mem::size_of::<Element>(),
            )
        }
    }
}

pub struct Library {
    handle: Handle,
}

pub struct Kernel {
    handle: Handle,
    width: usize,
    capacity: usize,
}

impl Kernel {
    pub fn width(&self) -> usize {
        self.width
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    // The threads of a group for a grid of threads: a row of as many as the kernel takes, or, for
    // a grid of rows, a SIMD group's width of columns and as many rows as fit.
    pub fn group(&self, thread: [usize; 3]) -> [usize; 3] {
        if thread[1] == 1 {
            return [self.capacity.min(thread[0]).max(1), 1, 1];
        }
        let width = self.width.min(thread[0]).max(1);
        let height = (self.capacity / width).min(thread[1]).max(1);
        [width, height, 1]
    }
}

pub struct Command<'device> {
    device: &'device Device,
    buffer: Handle,
    encoder: Option<Handle>,
    retained: Vec<Handle>,
    reached: Vec<Object>,
    local: PhantomData<*const ()>,
}

impl Drop for Command<'_> {
    fn drop(&mut self) {
        self.close();
    }
}

impl<'device> Command<'device> {
    fn encoder(&mut self) -> Result<Object, Failure> {
        if let Some(encoder) = &self.encoder {
            return Ok(encoder.0);
        }
        let _pool = Pool::new();
        let encoder = Handle::retain(send!(self.buffer.0, c"computeCommandEncoder"; Object))
            .ok_or_else(|| Failure::Execution("no compute encoder".to_owned()))?;
        let id = encoder.0;
        Self::resident(id, &self.reached);
        self.encoder = Some(encoder);
        Ok(id)
    }

    fn resident(encoder: Object, memory: &[Object]) {
        if memory.is_empty() {
            return;
        }
        send!(
            encoder,
            c"useResources:count:usage:",
            memory.as_ptr() => *const Object,
            memory.len() as u64 => u64,
            READ | WRITE => u64;
            ()
        );
    }

    // Lets every dispatch encoded after this read and write memories that kernels reach through
    // device addresses rather than bindings.
    pub fn reach(&mut self, memory: impl IntoIterator<Item = &'device Memory>) {
        let object = memory
            .into_iter()
            .map(|entry| entry.handle.0)
            .collect::<Vec<_>>();
        if let Some(encoder) = &self.encoder {
            Self::resident(encoder.0, &object);
        }
        self.reached.extend(object);
    }

    fn close(&mut self) {
        if let Some(encoder) = self.encoder.take() {
            send!(encoder.0, c"endEncoding"; ());
        }
    }

    pub fn dispatch(
        &mut self,
        kernel: &Kernel,
        memory: &[&'device Memory],
        constant: &[u8],
        thread: [usize; 3],
        group: [usize; 3],
    ) -> Result<(), Failure> {
        if thread.contains(&0) {
            return Ok(());
        }
        let encoder = self.encoder()?;
        send!(encoder, c"setComputePipelineState:", kernel.handle.0 => Object; ());
        for (index, entry) in memory.iter().enumerate() {
            send!(
                encoder,
                c"setBuffer:offset:atIndex:",
                entry.handle.0 => Object,
                0 => u64,
                index as u64 => u64;
                ()
            );
        }
        if !constant.is_empty() {
            send!(
                encoder,
                c"setBytes:length:atIndex:",
                constant.as_ptr().cast::<c_void>() => *const c_void,
                constant.len() as u64 => u64,
                memory.len() as u64 => u64;
                ()
            );
        }
        let size = |value: [usize; 3]| Size {
            width: value[0] as u64,
            height: value[1] as u64,
            depth: value[2] as u64,
        };
        send!(
            encoder,
            c"dispatchThreads:threadsPerThreadgroup:",
            size(thread) => Size,
            size(group) => Size;
            ()
        );
        Ok(())
    }

    fn matrix(operand: &Operand<'_>) -> Option<Handle> {
        let descriptor = send!(
            class(c"MPSMatrixDescriptor"),
            c"matrixDescriptorWithRows:columns:rowBytes:dataType:",
            operand.row as u64 => u64,
            operand.column as u64 => u64,
            (operand.column * 4) as u64 => u64,
            FLOAT => u32;
            Object
        );
        let allocated = send!(class(c"MPSMatrix"), c"alloc"; Object);
        Handle::new(send!(
            allocated,
            c"initWithBuffer:offset:descriptor:",
            operand.memory.handle.0 => Object,
            (operand.offset * 4) as u64 => u64,
            descriptor => Object;
            Object
        ))
    }

    pub fn multiply(&mut self, product: Product<'device>) -> Result<(), Failure> {
        let _pool = Pool::new();
        let fits = |operand: &Operand<'_>| {
            (operand.offset + operand.row * operand.column) * 4 <= operand.memory.length
        };
        if !fits(&product.left) || !fits(&product.right) || !fits(&product.result) {
            return Err(Failure::Execution("a matrix exceeds its memory".to_owned()));
        }
        let (row, column) = (product.result.row, product.result.column);
        let interior = if product.left.transpose {
            product.left.row
        } else {
            product.left.column
        };
        if row == 0 || column == 0 || interior == 0 {
            return Ok(());
        }
        self.close();
        let (Some(left), Some(right), Some(result)) = (
            Self::matrix(&product.left),
            Self::matrix(&product.right),
            Self::matrix(&product.result),
        ) else {
            return Err(Failure::Execution("could not describe a matrix".to_owned()));
        };
        let allocated = send!(class(c"MPSMatrixMultiplication"), c"alloc"; Object);
        let kernel = Handle::new(send!(
            allocated,
            c"initWithDevice:transposeLeft:transposeRight:resultRows:resultColumns:interiorColumns:alpha:beta:",
            self.device.handle.0 => Object,
            product.left.transpose => bool,
            product.right.transpose => bool,
            row as u64 => u64,
            column as u64 => u64,
            interior as u64 => u64,
            f64::from(product.alpha) => f64,
            f64::from(product.beta) => f64;
            Object
        ))
        .ok_or_else(|| Failure::Execution("could not create a multiplication".to_owned()))?;
        send!(
            kernel.0,
            c"encodeToCommandBuffer:leftMatrix:rightMatrix:resultMatrix:",
            self.buffer.0 => Object,
            left.0 => Object,
            right.0 => Object,
            result.0 => Object;
            ()
        );
        self.retained.extend([left, right, result, kernel]);
        Ok(())
    }

    // Encodes a blit command in an encoder of its own, so it runs after every dispatch before it
    // and before every dispatch after it.
    fn blit(&mut self, encode: impl FnOnce(Object)) -> Result<(), Failure> {
        let _pool = Pool::new();
        self.close();
        let encoder = Handle::retain(send!(self.buffer.0, c"blitCommandEncoder"; Object))
            .ok_or_else(|| Failure::Execution("no blit encoder".to_owned()))?;
        encode(encoder.0);
        send!(encoder.0, c"endEncoding"; ());
        Ok(())
    }

    // Copies the first count elements of one memory to the start of another.
    pub fn copy<Element: Plain>(
        &mut self,
        source: &'device Memory,
        target: &'device Memory,
        count: usize,
    ) -> Result<(), Failure> {
        let length = count * std::mem::size_of::<Element>();
        if length > source.length || length > target.length {
            return Err(Failure::Execution("a copy exceeds its memory".to_owned()));
        }
        if length == 0 {
            return Ok(());
        }
        self.blit(|encoder| {
            send!(
                encoder,
                c"copyFromBuffer:sourceOffset:toBuffer:destinationOffset:size:",
                source.handle.0 => Object,
                0 => u64,
                target.handle.0 => Object,
                0 => u64,
                length as u64 => u64;
                ()
            );
        })
    }

    // Sets every byte of a memory to zero on the device.
    pub fn clear(&mut self, memory: &'device Memory) -> Result<(), Failure> {
        if memory.length == 0 {
            return Ok(());
        }
        self.blit(|encoder| {
            send!(
                encoder,
                c"fillBuffer:range:value:",
                memory.handle.0 => Object,
                Range {
                    location: 0,
                    length: memory.length as u64,
                } => Range,
                0 => u8;
                ()
            );
        })
    }

    pub fn run(mut self) -> Result<(), Failure> {
        let _pool = Pool::new();
        self.close();
        send!(self.buffer.0, c"commit"; ());
        send!(self.buffer.0, c"waitUntilCompleted"; ());
        if send!(self.buffer.0, c"status"; u64) == COMPLETED {
            return Ok(());
        }
        Err(Failure::Execution(describe(
            send!(self.buffer.0, c"error"; Object),
        )))
    }
}
