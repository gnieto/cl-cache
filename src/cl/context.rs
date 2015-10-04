use opencl::cl::*;
use opencl::cl::ll::*;
use cl::device::Device;
use std::mem;
use std::ptr;
use libc;
use std::ops::Drop;

#[derive(Debug)]
pub struct Context {
    ctx: cl_context,
}

impl Context {
	pub fn from_devices(devices: &Vec<Device>) -> Context {
		unsafe {
			let mut errcode = 0;

			let ctx = clCreateContext(
				ptr::null(),
				devices.len() as u32,
				devices.as_ptr() as *const *mut libc::c_void,
				mem::transmute(ptr::null::<fn()>()),
				ptr::null_mut(),
				&mut errcode
			);

			Context {
				ctx: ctx,
			}
		}
	}

	pub fn get_id(&self) -> cl_context {
		self.ctx
	}
}

impl Drop for Context {
	fn drop(&mut self) {
		unsafe {
			clReleaseContext(self.ctx);
		}
	}
}

#[cfg(test)]
pub mod test {
    use cl::platform::*;
    use cl::cl_root::*;
    use super::*;

    #[test]
    pub fn it_can_create_a_context_from_devices() {
    	let pq = PlatformQuery::Index(0);
        let platform = ClRoot::get_platform(&pq).unwrap();

        let dq = DeviceQuery::Type(DeviceType::All);
        let devices = platform.get_devices_query(&dq);

    	let ctx = Context::from_devices(&devices);
    	println!("Context: {:?}", ctx);
    }
}	