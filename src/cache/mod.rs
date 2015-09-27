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

    pub fn get(&mut self, source: &str, devices: &Vec<Device>, ctx: &Context) -> Result<Program, CacheError> {
        self.get_with_options(&source, &devices, &ctx, "")
    }

    pub fn get_with_tag(&mut self, tag: &str, devices: &Vec<Device>, ctx: &Context) -> Result<Program, CacheError> {
        let mut binaries: Vec<Vec<u8>> = Vec::new();

        for device in devices {
            let key = self.key_hasher.get_tag_key(&device, &tag);
            let cache_result = self.backend.get(&key);

            match cache_result {
                None => {
                    return Err(CacheError::NotAllBinariesLoaded(devices.clone()));
                },
                Some(binary) => {
                    binaries.push(binary);
                },
            }
        }

        self.get_program_from_binaries(&ctx, &devices, &binaries)
    }

    pub fn put_with_tag(&mut self, tag: &str, devices: &Vec<Device>, program: &Program) -> Result<(), CacheError> {
        let binaries = program.get_binaries().unwrap();
        for (idx, b) in binaries.iter().enumerate() {
            if b.len() == 0 {
                return Err(CacheError::NeedBinaryProgram(devices[idx].clone()))
            }
        }

        for (idx, d) in devices.iter().enumerate() {
            self.backend.put(
                &self.key_hasher.get_tag_key(&d, &tag),
                &binaries[idx]
            );
        }

        Ok(())
    }

    pub fn get_with_options(&mut self, source: &str, devices: &Vec<Device>, ctx: &Context, options: &str) -> Result<Program, CacheError> {
        // TODO: Avoid all this clones by creating smarter data structures
        // The code is copying a lot of times the buffers with the binaries
        let source_str = source.to_string();
        let mut binaries_hash: HashMap<Device, Vec<u8>>  = HashMap::new();
        let mut non_build_devices = Vec::new();
        let mut keys = Vec::new();

        for device in devices {
            let key = self.build_cache_key(&device, &source_str, &options.to_string());

            let cache_result = self.backend.get(&key);
            match cache_result {
                None => {
                    non_build_devices.push(device.clone());
                    keys.push(key.clone());
                },
                Some(binary) => {
                    binaries_hash.insert(device.clone(), binary);
                },
            }
        }

        if non_build_devices.len() > 0 {
            let compilation_result = self.compile_program(&mut binaries_hash, &source, &options, &ctx, &non_build_devices, &keys);
            if compilation_result.is_err() {
                return Err(compilation_result.err().unwrap());
            }
        }

        let mut final_binaries = Vec::new();
        for device in devices {
            final_binaries.push(binaries_hash[device].clone());
        }

        self.get_program_from_binaries(&ctx, &devices, &final_binaries)
    }

    fn get_program_from_binaries(&self, ctx: &Context, devices: &Vec<Device>, binaries: &Vec<Vec<u8>>) -> Result<Program, CacheError> {
        let program = Program::from_binary(ctx, devices, &binaries);
        match program  {
            Err(_) => {
                println!("Could not get program from binary");
                Err(CacheError::CacheError)
            },
            Ok(p) => {
                let build_result = p.build(&devices);
                if build_result.is_err() {
                    Err(CacheError::ClError(build_result.err().unwrap()))
                } else {
                    Ok(p)
                }
            }
        }
    }

    fn compile_program(&mut self, binaries_hash: &mut HashMap<Device, Vec<u8>>, source: &str, options: &str, ctx: &Context, devices: &Vec<Device>, keys: &Vec<String>) -> Result<(), CacheError> {
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

        for (idx, device) in devices.iter().enumerate() {
            let binary = binaries[idx].clone();
            self.backend.put(&keys[idx], &binary);
            binaries_hash.insert(device.clone(), binary);
        }

        Ok(())
    }

    fn get_build_logs(&self, program: &Program, devices: &Vec<Device>) -> HashMap<Device, String> {
        let mut log_map: HashMap<Device, String> = HashMap::new();

        for device in devices {
            log_map.insert(device.clone(), program.get_log(&device).unwrap());
        }

        log_map
    }

    fn build_cache_key(&mut self, device: &Device, source: &String, options: &String) -> String {
        let key = self.key_hasher.get_key(&device, &source, &options);
        println!("Key {}", key);

        key
    }
}

#[derive(Debug)]
pub enum CacheError {
    ClBuildError(HashMap<Device, String>),
    ClError(OpenClError),
    NotAllBinariesLoaded(Vec<Device>),
    NeedBinaryProgram(Device),
    CacheError,
}

impl From<OpenClError> for CacheError {
    fn from(error: OpenClError) -> Self {
        CacheError::ClError(error)
    }
}

pub trait CacheBackend {
    fn get(&self, key: &String) -> Option<Vec<u8>>;
    fn put(&mut self, key: &String, payload: &Vec<u8>);
}

pub trait KeyHasher {
    fn get_key(&mut self, device: &Device, source: &String, options: &String) -> String;
    fn get_tag_key(&mut self, device: &Device, tag: &str) -> String;
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
    fn get_key(&mut self, device: &Device, source: &String, options: &String) -> String {
        self.digester.reset();
        let device_name = device.get_name().unwrap();
        let platform_id = device.get_platform_id().unwrap();
        let platform = Platform::from_platform_id(platform_id);
        let platform_name = platform.name();
        let platform_version = platform.version();
        let content_to_hash = source.clone() + &(*device_name) + &(*platform_name) + &(*platform_version) + &options;
        self.digester.input_str(&content_to_hash);

        self.digester.result_str()
    }

    fn get_tag_key(&mut self, device: &Device, tag: &str) -> String {
        self.digester.reset();
        let device_name = device.get_name().unwrap();
        let platform_id = device.get_platform_id().unwrap();
        let platform = Platform::from_platform_id(platform_id);
        let platform_name = platform.name();
        let platform_version = platform.version();
        let content_to_hash = "".to_string() + &(*device_name) + &(*platform_name) + &(*platform_version);
        self.digester.input_str(&content_to_hash);

        tag.to_string().clone() + &self.digester.result_str()
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use cl::context::Context;
    use cl::platform::Platform;
    use cl::program::Program;
    use cl::device::Device;
    use cache::volatile::Volatile;

    struct DummyCacheBackend;

    impl CacheBackend for DummyCacheBackend {
        fn get(&self, _: &String) -> Option<Vec<u8>> {
            None
        }

        fn put(&mut self, _: &String, _: &Vec<u8>) {}
    }

    #[test]
    pub fn it_can_create_a_cache() {
        let _ = create_cache_dummy_backend();
    }

    #[test]
    fn it_does_not_create_same_hash_with_same_source_but_distinct_device() {
        let mut c = create_cache_dummy_backend();
        let src = get_demo_source();
        let (ctx, devices) = get_context();

        let program = Program::from_source(&ctx, src).unwrap();
        program.build(&devices).unwrap();
        assert!(devices.len() >= 2);

        let keys: Vec<String> = devices.
            iter().
            map(|x| c.build_cache_key(&x, &src.to_string(), &"".to_string())).
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
        let mut c = create_cache_dummy_backend();
        let src = get_demo_source();
        let (_, devices) = get_context();

        let device = &devices[0];
        let key_wo_options = c.build_cache_key(&device, &src.to_string(), &"".to_string());
        let key_with_options = c.build_cache_key(&device, &src.to_string(), &"-D test=2".to_string());

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
        let mut c = create_cache_volatile_backend();
        let src = get_demo_source();
        let (ctx, devices) = get_context();        
        let prg_a = Program::from_source(&ctx, &src).unwrap();
        let prg_b = Program::from_source(&ctx, &src).unwrap();
        prg_a.build(&devices).unwrap();
        prg_b.build_with_options(&devices, "-D test=22").unwrap();

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

    fn get_context() -> (Context, Vec<Device>) {
        let platforms = Platform::all().unwrap();

        if platforms.len() == 0 {
            panic!("There is no OpenCL platform");
        }

        let devices = platforms[0].get_devices();

        // TODO: Avoid this clones
        (Context::from_devices(devices.clone()), devices.clone())
    }
}
