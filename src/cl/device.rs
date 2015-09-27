use opencl::cl::*;
use opencl::cl::ll::*;
use std::ptr;
use libc;
use std::iter::repeat;
use opencl::cl::CLStatus::*;
use std::cmp::Eq;
use cl::OpenClError;
use std::mem;

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

    pub fn get_vendor_id(&self) -> Result<cl_uint, OpenClError> {
    	self.profile_info_scalar::<cl_uint>(CL_DEVICE_VENDOR_ID)
    }

    pub fn get_id(&self) -> cl_device_id {
    	self.id
    }

    pub fn get_platform_id(&self) -> Result<cl_platform_id, OpenClError> {
    	unsafe {
			let mut result = 0 as cl_platform_id;
			// println!("Result pointer {:p}; value: {:?}", &result, *result);

			let status = clGetDeviceInfo(
				self.id,
				CL_DEVICE_PLATFORM,
				mem::size_of::<cl_platform_id>() as u64,
				(&mut result as *mut cl_platform_id) as *mut libc::c_void,
				ptr::null_mut()
			);

			if status != CL_SUCCESS as cl_int {
				return Err(OpenClError::new("Could not get the profile info".to_string(), status));
			}

			Ok(result)
		}
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

	fn profile_info_scalar<T>(&self, name: cl_device_info) -> Result<T, OpenClError> where T: Default {
		unsafe {
			let mut result = Default::default();

			let status = clGetDeviceInfo(
				self.id,
				name,
				mem::size_of::<T>() as u64,
				(&mut result as *mut T) as *mut libc::c_void,
				ptr::null_mut()
			);

			if status != CL_SUCCESS as cl_int {
				return Err(OpenClError::new("Could not get the profile info".to_string(), status));
			}

			Ok(result)
		}
	}
}