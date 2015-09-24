pub mod volatile;
pub mod disk;
pub mod repository;

use cl::device::Device;
use cl::context::Context;
use cl::program::Program;
use cl::OpenClError;
use std::collections::HashMap;
use crypto::digest::Digest;
use crypto::sha2::Sha256;

pub struct Cache {
    backend: Box<CacheBackend>,
    digester: Box<Digest>,
}

impl Cache {
    pub fn new(backend: Box<CacheBackend>) -> Cache {
        Cache {
            backend: backend,
            digester: Box::new(Sha256::new()),
        }
    }

    pub fn get(&mut self, source: &str, devices: &Vec<Device>, ctx: &Context) -> Option<Program> {
        // TODO: Avoid all this clones by creating smarter data structures
        // The code is copying a lot of times the buffers with the binaries
        let source_str = source.to_string();
        let mut binaries_hash: HashMap<Device, Vec<u8>>  = HashMap::new();
        let mut non_build_devices = Vec::new();
        let mut keys = Vec::new();

        for device in devices {
            let key = self.build_cache_key(&device, &source_str);

            let cache_result = self.backend.get(&key);
            match cache_result {
                None => {
                    non_build_devices.push(device.clone());
                    keys.push(key.clone());
                },
                Some(binary) => {binaries_hash.insert(device.clone(), binary);},
            }
        }

        let compilation_result = self.compile_program(&mut binaries_hash, &source, &ctx, &non_build_devices, &keys);
        if compilation_result.is_err() {
            return None;
        }

        let mut final_binaries = Vec::new();
        for device in devices {
            final_binaries.push(binaries_hash[device].clone());
        }

        Program::from_binary(ctx, devices, &final_binaries)
    }

    fn compile_program(&mut self, binaries_hash: &mut HashMap<Device, Vec<u8>>, source: &str, ctx: &Context, devices: &Vec<Device>, keys: &Vec<String>) -> Result<Vec<Vec<u8>>, OpenClError> {
        let program = try!{Program::from_source(ctx, source)};
        try!{program.build(&devices)};
        let binaries = try!{program.get_binaries()};

        for (idx, device) in devices.iter().enumerate() {
            let binary = binaries[idx].clone();
            self.backend.put(&keys[idx], &binary);
            binaries_hash.insert(device.clone(), binary);
        }

        Ok(Vec::new())
    }

    fn build_cache_key(&mut self, device: &Device, source: &String) -> String {
        self.digester.reset();
        let device_name = device.get_name().unwrap();
        let content_to_hash = source.clone() + &(*device_name);
        self.digester.input_str(&content_to_hash);

        self.digester.result_str()
    }
}

pub trait CacheBackend {
    fn get(&self, key: &String) -> Option<Vec<u8>>;
    fn put(&mut self, key: &String, payload: &Vec<u8>);
}

#[cfg(test)]
pub mod test {
    use super::*;
    use cl::context::Context;
    use cl::platform::Platform;
    use cl::program::Program;
    use cl::device::Device;

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
            map(|x| c.build_cache_key(&x, &src.to_string())).
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

    fn create_cache_dummy_backend() -> Cache {
        Cache::new(Box::new(DummyCacheBackend))
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
