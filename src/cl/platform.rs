use opencl::cl::*;
use opencl::cl::ll::*;
use cl::device::Device;
use std::ptr;
use std::iter::repeat;
use libc;
use cl::OpenClError;
use opencl::cl::CLStatus::*;
use regex::Regex;
use std::rc::Rc;

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

#[derive(Debug)]
pub struct Platform {
    id: cl_platform_id
}

impl Platform {
    pub fn from_platform_id(id: cl_platform_id) -> Platform {
        Platform { id: id }
    }

    pub fn get_devices_query(&self, query: &DeviceQuery) -> Vec<Rc<Device>> {
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
                    if regex.is_match(&d.get_name().unwrap_or("".to_string())) {
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

    pub fn get_devices(&self) -> Vec<Rc<Device>>
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
        self.profile_info(CL_PLATFORM_NAME).unwrap_or("Unknwon".to_string())
    }

    pub fn version(&self) -> String
    {
        self.profile_info(CL_PLATFORM_VERSION).unwrap_or("Unknwon".to_string())
    }

    pub fn profile(&self) -> String
    {
        self.profile_info(CL_PLATFORM_PROFILE).unwrap_or("Unknwon".to_string())
    }

    pub fn vendor(&self) -> String
    {
        self.profile_info(CL_PLATFORM_VENDOR).unwrap_or("Unknwon".to_string())
    }

    pub fn extensions(&self) -> String
    {
        self.profile_info(CL_PLATFORM_EXTENSIONS).unwrap_or("Unknwon".to_string())
    }

    fn get_devices_internal(&self, dtype: cl_device_type) -> Vec<Rc<Device>>
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
            ids.iter().map(|id| { Rc::new(Device::from_device_id(*id)) }).collect()
        }
    }
}