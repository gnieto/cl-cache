use opencl::cl::*;
use opencl::cl::ll::*;
use std::ptr;
use libc;
use std::iter::repeat;
use opencl::cl::CLStatus::*;
use std::cmp::Eq;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Device {
    id: cl_device_id,
}

impl Device {
    pub fn from_device_id(id: cl_device_id) -> Device {
        Device {
            id: id,
        }
    }

    pub fn get_name(&self) -> Option<String> {
		self.profile_info(CL_DEVICE_NAME)
    }

    pub fn get_id(&self) -> cl_device_id {
    	self.id
    }

    fn profile_info(&self, name: cl_device_info) -> Option<String>
	{
		unsafe {
			let mut size = 0 as libc::size_t;
			let status = clGetDeviceInfo(
				self.id,
				name,
				0,
				ptr::null_mut(),
				&mut size
			);

			if status != CL_SUCCESS as cl_int {
				return None;
			}

			let mut buf : Vec<u8> = repeat(0u8).take(size as usize).collect();

			let status = clGetDeviceInfo(
				self.id,
				name,
				size,
				buf.as_mut_ptr() as *mut libc::c_void,
				ptr::null_mut()
			);

			if status != CL_SUCCESS as cl_int {
				return None;
			}

			Some(String::from_utf8_unchecked(buf))
		}
	}
}