use opencl::cl::*;
use opencl::cl::ll::*;
use cl::program::Program;
use cl::OpenClError;
use opencl::cl::CLStatus::*;
use std::ffi::CString;
use cl::buffer::KernelArgument;
use libc;
use std::ptr;
use std::ops::Drop;

pub struct Kernel {
	kernel: cl_kernel,
}

impl Drop for Kernel {
	fn drop(&mut self) {
		unsafe {
			clReleaseKernel(self.kernel);
		}
	}
}

impl Kernel {
	pub fn from_id(kernel: cl_kernel) -> Kernel {
		unsafe {
			// Increase ref-count to be able to implement Drop
			clRetainKernel(kernel);
		}

		Kernel {
			kernel: kernel,
		}
	}

	pub fn get_id(&self) -> cl_kernel {
		self.kernel
	}

	pub fn create(program: &Program, kernel_name: &str) -> Result<Kernel, OpenClError> {
		unsafe {
			let kernel_name = try!{CString::new(kernel_name)};
			let mut status = 0;

			let kernel = clCreateKernel(
				program.get_id(),
				kernel_name.as_ptr(),
				(&mut status)
			);

			if status != CL_SUCCESS as cl_int {
				return Err(
					OpenClError::new(
						"Could not create kernel from program".to_string(),
						status
					)
				);
			}

			Ok(Self::from_id(kernel))
		}
	}

	pub fn set_arg(&self, index: u32, arg: &KernelArgument) -> Result<(), OpenClError> {
		unsafe {
			let status = clSetKernelArg(
				self.get_id(),
				index as cl_uint,
				arg.get_size() as u64,
				arg.as_buffer_ptr() as *const libc::c_void
			);

			if status != CL_SUCCESS as cl_int {
				return Err(OpenClError::new("Could not set kernel argument".to_string(), status));
			}

			Ok(())
		}
	}
}

pub trait WorkSizeable {
	fn get_dim(&self) -> usize;
	fn get_global(&self) -> *const usize;
	fn get_local(&self) -> *const usize;
	fn get_offset(&self) -> *const usize;
}

pub struct WorkSize1D {
	global: [usize; 1],
	local: Option<[usize; 1]>,
	offset: Option<[usize; 1]>,
}

impl WorkSize1D {
	pub fn new(global: usize) -> WorkSize1D {
		WorkSize1D {
			global: [global],
			local: None,
			offset: None,
		}
	}

	pub fn offset(mut self, offset: usize) -> WorkSize1D {
		self.offset = Some([offset]);
		self
	}

	pub fn local(mut self, local: usize) -> WorkSize1D {
		self.local = Some([local]);
		self
	}
}

impl WorkSizeable for WorkSize1D {
	fn get_dim(&self) -> usize {
		1
	}

	fn get_global(&self) -> *const usize {
		self.global.as_ptr()
	}

	fn get_local(&self) -> *const usize {
		match self.local {
			None => ptr::null(),
			Some(l) => l.as_ptr(),
		}
	}

	fn get_offset(&self) -> *const usize {
		match self.offset {
			None => ptr::null(),
			Some(l) => l.as_ptr(),
		}
	}
}

pub struct WorkSize2D {
	global: [usize; 2],
	local: Option<[usize; 2]>,
	offset: Option<[usize; 2]>,
}

impl WorkSize2D {
	pub fn new(global_x: usize, global_y: usize) -> WorkSize2D {
		WorkSize2D {
			global: [global_x, global_y],
			local: None,
			offset: None,
		}
	}

	pub fn offset(mut self, offset_x: usize, offset_y: usize) -> WorkSize2D {
		self.offset = Some([offset_x, offset_y]);
		self
	}

	pub fn local(mut self, local_x: usize, local_y: usize) -> WorkSize2D {
		self.local = Some([local_x, local_y]);
		self
	}
}

impl WorkSizeable for WorkSize2D {
	fn get_dim(&self) -> usize {
		2
	}

	fn get_global(&self) -> *const usize {
		self.global.as_ptr()
	}

	fn get_local(&self) -> *const usize {
		match self.local {
			None => ptr::null(),
			Some(l) => l.as_ptr(),
		}
	}

	fn get_offset(&self) -> *const usize {
		match self.offset {
			None => ptr::null(),
			Some(l) => l.as_ptr(),
		}
	}
}

pub struct WorkSize3D {
	global: [usize; 3],
	local: Option<[usize; 3]>,
	offset: Option<[usize; 3]>,
}

impl WorkSize3D {
	pub fn new(global_x: usize, global_y: usize, global_z: usize) -> WorkSize3D {
		WorkSize3D {
			global: [global_x, global_y, global_z],
			local: None,
			offset: None,
		}
	}

	pub fn offset(mut self, offset_x: usize, offset_y: usize, offset_z: usize) -> WorkSize3D {
		self.offset = Some([offset_x, offset_y, offset_z]);
		self
	}

	pub fn local(mut self, local_x: usize, local_y: usize, local_z: usize) -> WorkSize3D {
		self.local = Some([local_x, local_y, local_z]);
		self
	}
}

impl WorkSizeable for WorkSize3D {
	fn get_dim(&self) -> usize {
		3
	}

	fn get_global(&self) -> *const usize {
		self.global.as_ptr()
	}

	fn get_local(&self) -> *const usize {
		match self.local {
			None => ptr::null(),
			Some(l) => l.as_ptr(),
		}
	}

	fn get_offset(&self) -> *const usize {
		match self.offset {
			None => ptr::null(),
			Some(l) => l.as_ptr(),
		}
	}
}