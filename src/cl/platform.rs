use opencl::cl::*;
use opencl::cl::ll::*;
use cl::device::Device;
use std::ptr;
use std::iter::repeat;
use libc;
use std;
use cl::OpenClError;
use opencl::cl::CLStatus::*;
use regex::Regex;

// Almost copy of the platfrom from opencl_rust

#[derive(Copy, Clone, Debug)]
pub enum DeviceType {
      CPU,
      GPU,
      All,
}

impl DeviceType {
    pub fn convert_device_type(&self) -> cl_device_type {
        match *self {
            DeviceType::CPU => CL_DEVICE_TYPE_CPU,
            DeviceType::GPU => CL_DEVICE_TYPE_GPU | CL_DEVICE_TYPE_ACCELERATOR,
            DeviceType::All => CL_DEVICE_TYPE_ALL
        }        
    }
}

#[derive(Debug)]
pub enum DeviceQuery {
    Index(usize),
    Type(DeviceType),
    Regexp(Regex),
}

#[derive(Clone, Debug)]
pub struct Platform {
    id: cl_platform_id
}

impl Platform {
    pub fn all() -> Result<Vec<Platform>, OpenClError> {
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

            Ok(ids.iter().map(|id| { Platform { id: *id } }).collect())
        }
    }

    pub fn get_devices_query(&self, query: &DeviceQuery) -> Vec<Device> {
        match *query {
            DeviceQuery::Index(i) => {
                let all_devices = self.get_devices();
                let mut out_device = Vec::new();
                out_device.push(all_devices[i].clone());

                out_device
            },
            DeviceQuery::Regexp(ref regex) => {
                let all_devices = self.get_devices();
                let mut out_devices = Vec::new();

                for d in all_devices.iter() {
                    if regex.is_match(&d.get_name().unwrap()) {
                        out_devices.push(d.clone());
                    }
                }

                out_devices
            },
            DeviceQuery::Type(device_type) => {
                self.get_devices_internal(device_type.convert_device_type())
            },
        }
    }

    pub fn get_devices(&self) -> Vec<Device>
    {
        self.get_devices_internal(CL_DEVICE_TYPE_ALL)
    }

    fn profile_info(&self, name: cl_platform_info) -> Result<String, OpenClError>
    {
        unsafe {
            let mut size = 0 as libc::size_t;

            let status = clGetPlatformInfo(self.id,
                name,
                0,
                ptr::null_mut(),
                &mut size
            );

            if status != CL_SUCCESS as cl_int {
                return Err(OpenClError::new("Could not get size from platfrom info".to_string(), status));
            }

            let mut buf : Vec<u8>
                = repeat(0u8).take(size as usize).collect();

            let status = clGetPlatformInfo(self.id,
                name,
                size,
                buf.as_mut_ptr() as *mut libc::c_void,
                ptr::null_mut()
            );

            if status != CL_SUCCESS as cl_int {
                return Err(
                    OpenClError::new(
                        format!("Could not get {} from platfrom info", name),
                        status
                    )
                );
            }
            

            Ok(String::from_utf8_unchecked(buf))
        }
    }

    pub fn get_id(&self) -> cl_platform_id {
        self.id
    }

    pub fn name(&self) -> String
    {
        self.profile_info(CL_PLATFORM_NAME).unwrap()
    }

    pub fn version(&self) -> String
    {
        self.profile_info(CL_PLATFORM_VERSION).unwrap()
    }

    pub fn profile(&self) -> String
    {
        self.profile_info(CL_PLATFORM_PROFILE).unwrap()
    }

    pub fn vendor(&self) -> String
    {
        self.profile_info(CL_PLATFORM_VENDOR).unwrap()
    }

    pub fn extensions(&self) -> String
    {
        self.profile_info(CL_PLATFORM_EXTENSIONS).unwrap()
    }

    pub fn from_platform_id(id: cl_platform_id) -> Platform {
        Platform { id: id }
    }

    fn get_devices_internal(&self, dtype: cl_device_type) -> Vec<Device>
    {
        unsafe
        {
            let mut num_devices = 0;

            clGetDeviceIDs(self.id, dtype, 0, ptr::null_mut(),
                           (&mut num_devices));

            let mut ids: Vec<cl_device_id> = repeat(0 as cl_device_id)
                .take(num_devices as usize).collect();
            clGetDeviceIDs(self.id, dtype, ids.len() as cl_uint,
                           ids.as_mut_ptr(), (&mut num_devices));
            ids.iter().map(|id| { Device::from_device_id(*id) }).collect()
        }
    }
}

// This mutex is used to work around weak OpenCL implementations.
// On some implementations concurrent calls to clGetPlatformIDs
// will cause the implantation to return invalid status.
static mut platforms_mutex: std::sync::StaticMutex = std::sync::MUTEX_INIT;