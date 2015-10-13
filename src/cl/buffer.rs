use opencl::cl::*;
use opencl::cl::ll::*;
use cl::context::Context;
use cl::OpenClError;
use opencl::cl::CLStatus::*;
use std::mem;
use std::ptr;
use std::marker::PhantomData;
use std::ops::Drop;

pub trait KernelArgument {
	fn get_size(&self) -> usize;
	fn as_buffer_ptr(&self) -> *const cl_mem;
}

#[derive(Debug)]
pub struct RawBuffer {
	buffer: cl_mem,
}

impl RawBuffer {
	pub fn from_id(buffer: cl_mem) -> RawBuffer {
		unsafe {
			// Increase ref-count to be able to implement Drop
			clRetainMemObject(buffer);
		}

		RawBuffer {
			buffer: buffer,
		}
	}

	pub fn from_size(ctx: &Context, size: usize) -> Result<RawBuffer, OpenClError> {
		let mut errcode = 0;

		unsafe {
			let mem = clCreateBuffer (
				ctx.get_id(),
				0,
				size as u64,
				ptr::null_mut(),
				&mut errcode
			);

			if errcode != CL_SUCCESS as cl_int {
				return Err(OpenClError::new("Could not create buffer".to_string(), errcode));
			}

			Ok(Self::from_id(mem))
		}
	}

	pub fn get_id(&self) -> cl_mem {
		self.buffer
	}

	pub fn as_ptr(&self) -> *const cl_mem {
		&self.buffer
	}
}

impl Drop for RawBuffer {
	fn drop(&mut self) {
		unsafe {
			clReleaseMemObject(self.buffer);
		}
	}
}

// TODO: Change Vec for slice?
#[derive(Debug)]
pub struct InputBuffer<'a, T> where T: 'a {
	buffer: RawBuffer,
	vec: &'a Vec<T>,
}

impl <'a, T> InputBuffer<'a, T> where T: 'a {
	pub fn new(ctx: &Context, vec: &'a Vec<T>) -> Result<InputBuffer<'a, T>, OpenClError> {
		let raw = try!{RawBuffer::from_size(&ctx, Self::get_vector_size(vec))};

		let input_buffer = InputBuffer {
			buffer: raw,
			vec: vec,
		};

		Ok(input_buffer)
	}

	pub fn get_id(&self) -> cl_mem {
		self.buffer.get_id()
	}

	pub fn get_buffer_size(&self) -> usize {
		Self::get_vector_size(self.vec)
	}

	pub fn as_ptr(&self) -> *const T {
		self.vec.as_ptr()
	}

	pub fn get_vector_size(vec: &'a Vec<T>) -> usize {
		(mem::size_of::<T>() * vec.len())
	}
}

impl<'a, T> KernelArgument for InputBuffer<'a, T> {
	fn get_size(&self) -> usize {
		mem::size_of::<cl_mem>()
	}

	fn as_buffer_ptr(&self) -> *const cl_mem {
		self.buffer.as_ptr() 
	}
}

pub struct OutputBuffer<T> {
	raw: RawBuffer,
	amount: usize,
	phantom: PhantomData<T>,
}

impl<T> OutputBuffer<T> {
	pub fn new(ctx: &Context, amount: usize) -> Result<OutputBuffer<T>, OpenClError> {
		let size = mem::size_of::<T>() * amount;
		let raw = try!{RawBuffer::from_size(ctx, size)};

		let output = OutputBuffer {
			raw: raw,
			amount: amount,
			phantom: PhantomData,
		};

		Ok(output)
	}

	pub fn get_id(&self) -> cl_mem {
		self.raw.get_id()
	}

	pub fn len(&self) -> usize {
		self.amount
	}

	pub fn get_buffer_size(&self) -> usize {
		mem::size_of::<T>() * (self.amount as usize)
	}
}

impl<T> KernelArgument for OutputBuffer<T> {
	fn get_size(&self) -> usize {
		mem::size_of::<cl_mem>()
	}

	fn as_buffer_ptr(&self) -> *const cl_mem {
		self.raw.as_ptr() as *const cl_mem
	}
}