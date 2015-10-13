use opencl::cl::*;
use opencl::cl::ll::*;
use cl::context::Context;
use cl::device::Device;
use cl::kernel::*;
use cl::buffer::{InputBuffer, OutputBuffer};
use cl::OpenClError;
use opencl::cl::CLStatus::*;
use std::ptr;
use std::iter::repeat;
use libc;
use std::ops::Drop;

pub struct CommandQueue {
	queue: cl_command_queue,
}

impl CommandQueue {
	pub fn from_command_queue_id(queue_id: cl_command_queue) -> CommandQueue {
		unsafe {
			// Increase ref-count to be able to implement Drop
			clRetainCommandQueue(queue_id);
		}

		CommandQueue {
			queue: queue_id,
		}
	}

	pub fn new(ctx: &Context, device: &Device) -> Result<CommandQueue, OpenClError> {
		let mut status: cl_int = 0;

		unsafe {
			let queue_id = clCreateCommandQueue(
				ctx.get_id(),
				device.get_id(),
				0,
				&mut status
			);

			if status != CL_SUCCESS as cl_int {
				return Err(OpenClError::new("Could not create program from source".to_string(), status));
			}

			Ok(CommandQueue::from_command_queue_id(queue_id))
		}
	}

	pub fn get_id(&self) -> cl_command_queue {
		self.queue
	}

	pub fn write<'a, T>(&self, buffer: &InputBuffer<'a, T>) -> Result<(), OpenClError> {
		unsafe {
			let status = clEnqueueWriteBuffer(
				self.get_id(),
				buffer.get_id(),
				CL_TRUE,
				0,
				buffer.get_buffer_size() as u64,
				buffer.as_ptr() as *const libc::c_void,
				0,
				ptr::null_mut(),
				ptr::null_mut()
			);

			if status != CL_SUCCESS as cl_int {
				return Err(OpenClError::new("Could not write buffer".to_string(), status));
			}
		}

		Ok(())
	}

	pub fn read<T>(&self, buffer: &OutputBuffer<T>) -> Result<Vec<T>, OpenClError> where T: Clone + Default {
		let mut result : Vec<T> = repeat(Default::default()).take(buffer.len() as usize).collect();
		
		unsafe {
			let status = clEnqueueReadBuffer(
				self.get_id(),
				buffer.get_id(),
				CL_TRUE,
				0,
				buffer.get_buffer_size() as u64,
				result.as_mut_ptr() as *mut libc::c_void,
				0,
				ptr::null_mut(),
				ptr::null_mut()
			);

			if status != CL_SUCCESS as cl_int {
				return Err(OpenClError::new("Could not read buffer".to_string(), status));
			}
		}

		Ok(result)
	}

	pub fn sync_work(&self, kernel: &Kernel, ws: &WorkSizeable) -> Result<(), OpenClError> {
		unsafe {
			let status = clEnqueueNDRangeKernel(
				self.get_id(),
				kernel.get_id(),
				ws.get_dim() as u32,
				ws.get_offset() as *const u64,
				ws.get_global() as *const u64,
				ws.get_local() as *const u64,
				0,
				ptr::null(),
				ptr::null_mut()
			);

			if status != CL_SUCCESS as cl_int {
				return Err(OpenClError::new("Could not queue nd kernel".to_string(), status));
			}
		}

		Ok(())
	}
}

impl Drop for CommandQueue {
	fn drop(&mut self) {
		unsafe {
			clReleaseCommandQueue(self.queue);
		}
	}
}