pub mod volatile;
pub mod disk;

use cl::device::Device;
use cl::context::Context;
use cl::program::Program;
use cl::platform::Platform;
use cl::OpenClError;
use std::collections::HashMap;
use crypto::digest::Digest;
use crypto::sha2::Sha256;
use std::rc::Rc;
use std::io::Error;

pub struct Cache {
    backend: Box<CacheBackend>,
    key_hasher: Box<KeyHasher>,
}

impl Cache {
    pub fn new(backend: Box<CacheBackend>) -> Cache {
        Cache {
            backend: backend,
            key_hasher: Box::new(DefaultHasher::new()),
        }
    }

    pub fn get(&mut self, source: &str, devices: &Vec<Rc<Device>>, ctx: &Context) -> Result<Program, CacheError> {
        self.get_with_options(&source, &devices, &ctx, "")
    }

    pub fn get_with_tag(&mut self, tag: &str, devices: &Vec<Rc<Device>>, ctx: &Context) -> Result<Program, CacheError> {
        let mut binaries: Vec<Vec<u8>> = Vec::new();

        for device in devices {
            let key = try!{self.key_hasher.get_tag_key(&device, &tag)};
            let cache_result = self.backend.get(&key);

            match cache_result {
                Err(_) => {
                    info!("Program not found for device: {:?}", device);
                    return Err(CacheError::NotAllBinariesLoaded(devices.clone()));
                },
                Ok(binary) => {
                    info!("Program found on cache for device: {:?}", device);
                    binaries.push(binary);
                },
            }
        }

        self.get_program_from_binaries(&ctx, &devices, &binaries)
    }

    pub fn put_with_tag(&mut self, tag: &str, devices: &Vec<Rc<Device>>, program: &Program) -> Result<(), CacheError> {
        let binaries = try!{program.get_binaries()};
        for (idx, b) in binaries.iter().enumerate() {
            if b.len() == 0 {
                return Err(CacheError::NeedBinaryProgram(devices[idx].clone()))
            }
        }

        for (idx, d) in devices.iter().enumerate() {
            let put_result = self.backend.put(
                &try!{self.key_hasher.get_tag_key(&d, &tag)},
                &binaries[idx]
            );

            if let Err(_) = put_result {
                return Err(CacheError::CacheError);
            }
        }

        Ok(())
    }

    pub fn get_with_options(&mut self, source: &str, devices: &Vec<Rc<Device>>, ctx: &Context, options: &str) -> Result<Program, CacheError> {
        // TODO: Avoid all this clones by creating smarter data structures
        // The code is copying a lot of times the buffers with the binaries
        let source_str = source.to_string();
        let mut binaries_hash: HashMap<Rc<Device>, Vec<u8>>  = HashMap::new();
        let mut non_build_devices = Vec::new();
        let mut keys = Vec::new();

        for device in devices {
            let key = try!{self.key_hasher.get_key(&device, &source_str, &options.to_string())};

            let cache_result = self.backend.get(&key);
            match cache_result {
                Err(_) => {
                    non_build_devices.push(device.clone());
                    keys.push(key.clone());
                },
                Ok(binary) => {
                    binaries_hash.insert(device.clone(), binary);
                },
            }
        }

        if non_build_devices.len() > 0 {
            try!{self.compile_program(&mut binaries_hash, &source, &options, &ctx, &non_build_devices, &keys)};
        }

        let mut final_binaries = Vec::new();
        for device in devices {
            final_binaries.push(binaries_hash[device].clone());
        }

        self.get_program_from_binaries(&ctx, &devices, &final_binaries)
    }

    fn get_program_from_binaries(&self, ctx: &Context, devices: &Vec<Rc<Device>>, binaries: &Vec<Vec<u8>>) -> Result<Program, CacheError> {
        let program = Program::from_binary(ctx, devices, &binaries);
        match program  {
            Err(cl_error) => {
                println!("Could not get program from binary");
                Err(CacheError::ClError(cl_error))
            },
            Ok(p) => {
                try!{p.build(&devices)};
                Ok(p)
            }
        }
    }

    fn compile_program(&mut self, binaries_hash: &mut HashMap<Rc<Device>, Vec<u8>>, source: &str, options: &str, ctx: &Context, devices: &Vec<Rc<Device>>, keys: &Vec<String>) -> Result<(), CacheError> {
        let program = try!{Program::from_source(ctx, source)};
        let build_result = if options.len() > 0 {
            program.build_with_options(&devices, &options)
        } else {
            program.build(&devices)
        };
        
        if build_result.is_err() {
            return Err(CacheError::ClBuildError(self.get_build_logs(&program, &devices)));
        }

        let binaries = try!{program.get_binaries()};

        if binaries.iter().any(|x| x.len() == 0) {
            return Err(CacheError::ClBuildError(self.get_build_logs(&program, &devices)));   
        }

        for (idx, device) in devices.iter().enumerate() {
            let binary = binaries[idx].clone();
            let put_result = self.backend.put(&keys[idx], &binary);

            if let Err(_) = put_result {
                return Err(CacheError::CacheError);
            }

            binaries_hash.insert(device.clone(), binary);
        }

        Ok(())
    }

    fn get_build_logs(&self, program: &Program, devices: &Vec<Rc<Device>>) -> HashMap<Rc<Device>, String> {
        let mut log_map: HashMap<Rc<Device>, String> = HashMap::new();

        for device in devices {
            if let Ok(log) = program.get_log(&device) {
                log_map.insert(device.clone(), log);
            }
        }

        log_map
    }
}

#[derive(Debug)]
pub enum CacheError {
    ClBuildError(HashMap<Rc<Device>, String>),
    ClError(OpenClError),
    NotAllBinariesLoaded(Vec<Rc<Device>>),
    NeedBinaryProgram(Rc<Device>),
    CacheError,
    IoError(Error),
}

impl From<OpenClError> for CacheError {
    fn from(error: OpenClError) -> Self {
        CacheError::ClError(error)
    }
}

impl From<Error> for CacheError {
    fn from(error: Error) -> Self {
        CacheError::IoError(error)
    }
}

#[derive(Debug)]
pub enum KeyError {
    IoError,
    KeyNotFound,
    InvalidContent,
}

impl From<Error> for KeyError {
    fn from(_: Error) -> Self {
        KeyError::IoError
    }
}


pub trait CacheBackend {
    fn get(&self, key: &String) -> Result<Vec<u8>, KeyError>;
    fn put(&mut self, key: &String, payload: &Vec<u8>) -> Result<(), KeyError>;
}

pub trait KeyHasher {
    fn get_key(&mut self, device: &Device, source: &String, options: &String) -> Result<String, CacheError>;
    fn get_tag_key(&mut self, device: &Device, tag: &str) -> Result<String, CacheError>;
}

pub struct DefaultHasher {
    digester: Box<Digest>,
}

impl DefaultHasher {
    pub fn new() -> DefaultHasher {
        DefaultHasher {
            digester: Box::new(Sha256::new()),
        }
    }
}

impl KeyHasher for DefaultHasher {
    fn get_key(&mut self, device: &Device, source: &String, options: &String) -> Result<String, CacheError> {
        self.digester.reset();
        let device_name = try!{device.get_name()};
        let platform_id = try!{device.get_platform_id()};
        let platform = Platform::from_platform_id(platform_id);
        let platform_name = platform.name();
        let platform_version = platform.version();
        let content_to_hash = source.clone() + &(*device_name) + &(*platform_name) + &(*platform_version) + &options;
        self.digester.input_str(&content_to_hash);

        Ok(self.digester.result_str())
    }

    fn get_tag_key(&mut self, device: &Device, tag: &str) -> Result<String, CacheError> {
        self.digester.reset();
        let device_name = try!{device.get_name()};
        let platform_id = try!{device.get_platform_id()};
        let platform = Platform::from_platform_id(platform_id);
        let platform_name = platform.name();
        let platform_version = platform.version();
        let content_to_hash = "".to_string() + &(*device_name) + &(*platform_name) + &(*platform_version);
        self.digester.input_str(&content_to_hash);

        Ok(tag.to_string().clone() + &self.digester.result_str())
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use cl::context::Context;
    use cl::program::Program;
    use cl::device::Device;
    use cache::volatile::Volatile;
    use cl::cl_root::*;
    use cl::platform::*;
    use std::rc::Rc;

    struct DummyCacheBackend;

    impl CacheBackend for DummyCacheBackend {
        fn get(&self, _: &String) -> Result<Vec<u8>, KeyError> {
            Err(KeyError::KeyNotFound)
        }

        fn put(&mut self, _: &String, _: &Vec<u8>) -> Result<(), KeyError> {
            Ok(())
        }
    }

    #[test]
    pub fn it_can_create_a_cache() {
        let _ = create_cache_dummy_backend();
    }

    #[test]
    fn it_does_not_create_same_hash_with_same_source_but_distinct_device() {
        let src = get_demo_source();
        let (ctx, devices) = get_context();
        let mut hasher = DefaultHasher::new();

        let program = Program::from_source(&ctx, src).unwrap();
        program.build(&devices).unwrap();
        assert!(devices.len() >= 2);

        let keys: Vec<String> = devices.
            iter().
            map(|x| hasher.get_key(&x, &src.to_string(), &"".to_string()).unwrap()).
            collect();

        let mut unique_keys = keys.clone();
        unique_keys.sort();
        unique_keys.dedup();

        assert!(keys.len() == unique_keys.len());
    }

    #[test]
    fn it_should_compile_for_all_devices_if_not_in_cache() {
        let mut c = create_cache_dummy_backend();
        let src = get_demo_source();
        let (ctx, devices) = get_context();

        c.get(&src, &devices, &ctx).unwrap();
    }

    #[test]
    fn it_creates_distinct_hashes_with_options_and_without() {
        let src = get_demo_source();
        let (_, devices) = get_context();
        let mut hasher = DefaultHasher::new();

        let device = &devices[0];
        let key_wo_options = hasher.get_key(&device, &src.to_string(), &"".to_string()).unwrap();
        let key_with_options = hasher.get_key(&device, &src.to_string(), &"-D test=2".to_string()).unwrap();

        assert!(key_wo_options != key_with_options)
    }

    #[test]
    fn it_can_not_put_with_tag_with_program_from_source() {
        let mut c = create_cache_dummy_backend();
        let src = get_demo_source();
        let (ctx, devices) = get_context();        
        let prg = Program::from_source(&ctx, &src).unwrap();

        let put_result = c.put_with_tag("test", &devices, &prg);
        match put_result {
            Err(CacheError::NeedBinaryProgram(_)) => (),
            _ => panic!("Needed a binary program"),
        }
    }

    #[test]
    fn it_can_put_with_tag_and_recover_a_progran_with_cached_contents() {
        let mut c = create_cache_volatile_backend();
        let src = get_demo_source();
        let (ctx, devices) = get_context();        
        let prg = Program::from_source(&ctx, &src).unwrap();
        prg.build(&devices).unwrap();

        c.put_with_tag("test", &devices, &prg).unwrap();
        
        let new_program = c.get_with_tag("test", &devices, &ctx).unwrap();
        let binaries = new_program.get_binaries().unwrap();
        if binaries.iter().all(|x| {x.len() > 0}) == false {
            panic!("All the binaries should have length >= 1")
        }
    }

    #[test]
    fn it_can_not_cache_same_program_with_distinct_options_and_same_tag() {
        /*let mut c = create_cache_volatile_backend();
        let src = get_demo_source();
        let (ctx, devices) = get_context();        
        let prg_a = Program::from_source(&ctx, &src).unwrap();
        let prg_b = Program::from_source(&ctx, &src).unwrap();
        prg_a.build(&devices).unwrap();
        prg_b.build_with_options(&devices, "-D test=22").unwrap();*/

        // TODO: Assert binaries are distinct
    }

    fn create_cache_dummy_backend() -> Cache {
        Cache::new(Box::new(DummyCacheBackend))
    }

    fn create_cache_volatile_backend() -> Cache {
        Cache::new(Box::new(Volatile::new()))
    }

    fn get_demo_source() -> &'static str {
        return "__kernel void vector_add(__global const long *A, __global const long *B, __global long *C) {
                    int i = get_global_id(0);
                    C[i] = A[i] + B[i];
        }";
    }

    fn get_context() -> (Context, Vec<Rc<Device>>) {
        let pq = PlatformQuery::Index(0);
        let platform = ClRoot::get_platform(&pq).unwrap();

        let dq = DeviceQuery::Type(DeviceType::All);
        let devices = platform.get_devices_query(&dq);

        // TODO: Avoid this clones
        (Context::from_devices(&devices), devices)
    }
}
