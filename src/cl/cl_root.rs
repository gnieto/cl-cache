use cl::platform::Platform;
use regex::Regex;
use std::rc::Rc;
use opencl::cl::*;
use opencl::cl::ll::*;
use std::ptr;
use std::iter::repeat;
use std;
use cl::OpenClError;
use opencl::cl::CLStatus::*;

pub struct ClRoot;

impl ClRoot {
	pub fn get_platform(pq: &PlatformQuery) -> Option<Rc<Platform>> {
		match *pq {
			PlatformQuery::Default => Self::platform_from_index(0),
			PlatformQuery::Index(i) => Self::platform_from_index(i),
			PlatformQuery::Regexp(ref r) => Self::platform_from_regexp(r),
		}
	}

	fn platform_from_index(index: usize) -> Option<Rc<Platform>> {
		let platforms = Self::get_platforms().unwrap();

		Self::platform_from_idx(&platforms, index)
	}

	fn platform_from_regexp(regex: &Regex) -> Option<Rc<Platform>> {
		let platforms = Self::get_platforms().unwrap();

		let index = platforms.iter().position(|ref x| regex.is_match(&x.name()));

		match index {
			None => None,
			Some(_) => Some(platforms[0].clone())
		}
	}

	fn get_platforms() -> Result<Vec<Rc<Platform>>, OpenClError> {
		let mut num_platforms = 0 as cl_uint;

        unsafe
        {
            let guard = platforms_mutex.lock();
            let status = clGetPlatformIDs(0,
                ptr::null_mut(),
                (&mut num_platforms)
            );

            if status != CL_SUCCESS as cl_int {
                return Err(OpenClError::new("Could not get the number of platforms".to_string(), status));
            }
            // unlock this before the check in case the check fails

            let mut ids: Vec<cl_device_id> = repeat(0 as cl_device_id)
                .take(num_platforms as usize).collect();

            let status = clGetPlatformIDs(num_platforms,
                ids.as_mut_ptr(),
                (&mut num_platforms)
            );

            if status != CL_SUCCESS as cl_int {
                return Err(OpenClError::new("Could not get the platforms".to_string(), status));
            }

            let _ = guard;

            Ok(ids.iter().map(|id| Rc::new(Platform::from_platform_id(*id))).collect())
        }
	}

	fn platform_from_idx(platforms: &Vec<Rc<Platform>>, index: usize) -> Option<Rc<Platform>> {
		if platforms.len() > 0 {
			Some(platforms[index].clone())
		} else {
			None
		}		
	}
}

#[derive(Debug)]
pub enum PlatformQuery {
	Default,
	Index(usize),
	Regexp(Regex),
}


// This mutex is used to work around weak OpenCL implementations.
// On some implementations concurrent calls to clGetPlatformIDs
// will cause the implantation to return invalid status.
static mut platforms_mutex: std::sync::StaticMutex = std::sync::MUTEX_INIT;