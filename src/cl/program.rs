use opencl::cl::*;
use opencl::cl::ll::*;
use cl::context::Context;
use cl::device::Device;
use std::ffi::CString;
use opencl::cl::CLStatus::*;
use std::ptr;
use std::mem;
use libc;
use std::iter::repeat;
use cl::OpenClError;

#[derive(Debug)]
pub struct Program {
    prg: cl_program,
}

impl Drop for Program
{
	fn drop(&mut self) {
		unsafe {
			clReleaseProgram(self.prg);
		}
	}
}

impl Program {
    pub fn from_cl_program(prg: cl_program) -> Program {
        Program {
        	prg: prg,
        }
    }

    pub fn from_source(ctx: &Context, src: &str) -> Result<Program, OpenClError> {
    	let src = CString::new(src).unwrap();
		let mut status: cl_int = 0;

		unsafe {
			let program = clCreateProgramWithSource(
				ctx.get_id(),
				1,
				&src.as_ptr(),
				ptr::null(),
				(&mut status)
			);

			if status != CL_SUCCESS as cl_int {
				return Err(OpenClError::new("Could not create program from source".to_string(), status));
			}

			Ok(Program{prg: program})
		}
    }

    pub fn from_binary(ctx: &Context, devices: &Vec<Device>, binaries: &Vec<Vec<u8>>) -> Result<Program, OpenClError> {
    	if devices.len() == 0 {
    		return Err(OpenClError::from_string("Can't create program without devices".to_string()));
    	}

    	if binaries.iter().any(|x| x.len() == 0) {
    		return Err(OpenClError::from_string("Binaries need to have some size".to_string()));
    	}

    	unsafe {
    		let devices_ids: Vec<cl_device_id> = devices.iter().map(|x| x.get_id()).collect();
    		let binary_sizes: Vec<u64> = binaries.iter().map(|x| x.len() as u64).collect();
    		let binary_ptrs: Vec<*const u8> = binaries.iter().map(|x| x.as_ptr() as *const u8).collect();

    		let mut errcode: cl_int = 0;
    		let program = clCreateProgramWithBinary(
    			ctx.get_id(),
    			devices.len() as u32,
    			devices_ids.as_ptr() as *const *mut libc::c_void,
    			binary_sizes.as_ptr() as *const u64,
    			binary_ptrs.as_ptr() as *const *const u8,
    			ptr::null_mut(),
    			&mut errcode
    		);

    		if errcode != CL_SUCCESS as cl_int {
    			return Err(OpenClError::new("Error creating program from binaries".to_string(), errcode));
    		}

    		Ok(Program::from_cl_program(program))
    	}
    }

    pub fn get_id(&self) -> cl_program {
    	self.prg
    }

    pub fn build(&self, devices: &Vec<Device>) -> Result<(), OpenClError> {
    	self.build_with_options(devices, "")
    }

    pub fn build_with_options(&self, devices: &Vec<Device>, options: &str) -> Result<(), OpenClError> {
    	unsafe
		{
			let option_ptr = if options.len() > 0 {
				options.as_ptr() as *const i8
			} else {
				ptr::null()
			};
			

			let ret = clBuildProgram(
				self.prg,
				devices.len() as u32,
				devices.as_ptr() as *const *mut libc::c_void,
				option_ptr,
				mem::transmute(ptr::null::<fn()>()),
				ptr::null_mut()
			);

			if ret != CL_SUCCESS as cl_int {
				return Err(OpenClError::new("Some error ocurred while compiling the program".to_string(), ret));
			}
		}

		Ok(())
    }

    pub fn get_log(&self, device: &Device) -> Result<String, OpenClError> {
    	unsafe {
    		let device_id = device.get_id();

			// Get the build log.
			let mut size = 0 as libc::size_t;
			let status = clGetProgramBuildInfo(
				self.prg,
				device_id,
				CL_PROGRAM_BUILD_LOG,
				0,
				ptr::null_mut(),
				(&mut size)
			);

			if status != CL_SUCCESS as cl_int {
				return Err(OpenClError::new("Could not get build log size".to_string(), status));
			}
		
			let mut buf : Vec<u8> = repeat(0u8).take(size as usize).collect();
			let status = clGetProgramBuildInfo(
				self.prg,
				device_id,
				CL_PROGRAM_BUILD_LOG,
				buf.len() as libc::size_t,
				buf.as_mut_ptr() as *mut libc::c_void,
				ptr::null_mut()
			);

			// TODO: Check status
			let log = String::from_utf8_lossy(&buf[..]);
			if status == CL_SUCCESS as cl_int {
				Ok(log.into_owned())
			} else {
				return Err(OpenClError::new("Could not get build log".to_string(), status));
			}
		}
    }

    pub fn get_binaries(&self) -> Result<Vec<Vec<u8>>, OpenClError> {
		let num_devices = self.get_num_devices();

		if num_devices == 0 {
			return Err(OpenClError::from_string("Current program does not have any device".to_string()));
		}

		let sizes = self.get_binary_sizes(num_devices);
		let sizes_amount = sizes.len();
		let mut buffer = Self::get_binaries_buffer(sizes);

    	unsafe {
    		let buffer_size = mem::size_of::<&libc::c_void>() * sizes_amount;
    		let mut buffer_ptrs: Vec<*mut u8> = buffer.iter_mut().map(|x| x.as_mut_ptr()).collect();

    		let errcode = clGetProgramInfo(
    			self.prg,
    		 	CL_PROGRAM_BINARIES,
    		 	buffer_size as u64,
    		 	buffer_ptrs.as_mut_ptr() as *mut libc::c_void,
    		 	ptr::null_mut()
    		);

    		if errcode != CL_SUCCESS as cl_int {
    			return Err(OpenClError::new("Could not get binaries".to_string(), errcode));
    		}
		}

		Ok(buffer)
    }

    pub fn get_source(&self) -> Result<String, OpenClError> {
    	let ss = try!{self.get_source_size()};

    	// The source code size returned will be the concatenation of all the source + the null char
    	// so, source code size will be allways at least 1. If it's 1, program has been
    	// created from binaries
    	if ss <= 1 {
    		return Err(OpenClError::from_string("This program has not source".to_string()))
    	}

    	let mut source = String::with_capacity(ss as usize);

    	unsafe {
			let errcode = clGetProgramInfo(
    			self.prg,
    			CL_PROGRAM_SOURCE,
    			ss,
    			source.as_mut_vec().as_mut_ptr() as *mut libc::c_void,
    			ptr::null_mut()
    		);

			if errcode != CL_SUCCESS as cl_int {
				return Err(OpenClError::new("Could not get program source".to_string(), errcode));
			}
    	}

    	Ok(source)
    }

    pub fn get_devices(&self) -> Result<Vec<Device>, OpenClError> {
    	let num_devices = self.get_num_devices();

    	unsafe {
    		let mut raw_devices: Vec<cl_device_id> = 
    			repeat(0 as cl_device_id)
    			.take(num_devices as usize)
    			.collect();

    		let vec_size = mem::size_of::<cl_device_id>() * num_devices as usize;
    		let errcode = clGetProgramInfo(
    			self.prg,
    			CL_PROGRAM_DEVICES,
    			vec_size as u64,
    			raw_devices.as_mut_ptr() as *mut libc::c_void,
    			ptr::null_mut()
    		);

    		if errcode != CL_SUCCESS as cl_int {
    			return Err(OpenClError::new("Could not get program devices".to_string(), errcode));
    		}

    		let devices : Vec<Device> = raw_devices.
				iter().
				map(|x| Device::from_device_id(*x)).
				collect();

			Ok(devices)
    	}
    }

    fn get_source_size(&self) -> Result<u64, OpenClError> {
    	let mut source_size: u64 = 0;

    	unsafe {
    		let errcode = clGetProgramInfo(
    			self.prg,
    			CL_PROGRAM_SOURCE,
    			0,
    			ptr::null_mut(),
    			&mut source_size 
    		);

    		if errcode != (CL_SUCCESS as cl_int) {
    			return Err(OpenClError::new("Could not retrieve source size".to_string(), errcode));
    		}
    	}

    	Ok(source_size)
    }

    fn get_binaries_buffer(sizes: Vec<libc::size_t>) -> Vec<Vec<u8>> {
    	let mut buffer = Vec::new();

    	for size in sizes {
    		let current_buffer: Vec<u8> =
    			repeat(0u8)
    			.take(size as usize)
    			.collect();

    		buffer.push(current_buffer);
    	}

    	buffer
    }

    fn get_num_devices(&self) -> cl_uint {
    	let mut num_devices : cl_uint = 0;

    	unsafe {
    		clGetProgramInfo(
    			self.prg,
    			CL_PROGRAM_NUM_DEVICES,
    			mem::size_of::<cl_uint>() as libc::size_t,
    			(&mut num_devices as *mut cl_uint) as *mut libc::c_void,
    			ptr::null_mut()
    		);
    	}

    	num_devices
    }

    fn get_binary_sizes(&self, num_devices: cl_uint) -> Vec<libc::size_t> {
    	let mut sizes : Vec<libc::size_t> =
    			repeat(0 as libc::size_t)
				.take(num_devices as usize)
				.collect();

    	unsafe {
			let vec_size: u64 = (mem::size_of::<libc::size_t>() as u64) * (num_devices as u64);

    		clGetProgramInfo(
    			self.prg,
    		 	CL_PROGRAM_BINARY_SIZES,
    		 	vec_size,
    		 	sizes.as_mut_ptr() as *mut libc::c_void,
    		 	ptr::null_mut()
    		);
    	}

    	sizes
    }
}
