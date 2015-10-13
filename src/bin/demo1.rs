extern crate clcache;

use clcache::cache::Cache;
use clcache::cache::disk::FileSystemCache;
use clcache::cl::device::Device;
use clcache::cl::context::Context;
use clcache::cl::platform::*;
use clcache::cl::cl_root::*;
use clcache::cl::command_queue::CommandQueue;
use clcache::cl::kernel::*;
use clcache::cl::buffer::{InputBuffer, OutputBuffer};
use std::rc::Rc;

pub fn main() {
	let mut cache = Cache::new(Box::new(FileSystemCache::new("/tmp/test_demo1/".to_string()).unwrap()));
	let (ctx, devices) = get_context();

	let program = cache.get(
		get_demo_code(),
		&devices,
		&ctx
	).unwrap();

	let a = vec![9i32, -5, 8, 3, 4, 5, 6, 7];
	let b = vec![7i32, 6, 5, 4, 3, 2, 1, 0];

	let cq = CommandQueue::new(&ctx, &devices[0]).unwrap();
	let kernel = Kernel::create(&program, "vector_add").unwrap();

	let a_bf = InputBuffer::new(&ctx, &a).unwrap();
	let b_bf = InputBuffer::new(&ctx, &b).unwrap();
	let c_bf = OutputBuffer::<i32>::new(&ctx, a.len()).unwrap();

	cq.write(&a_bf).unwrap();
	cq.write(&b_bf).unwrap();

	kernel.set_arg(0, &a_bf).unwrap();
	kernel.set_arg(1, &b_bf).unwrap();
	kernel.set_arg(2, &c_bf).unwrap();

	let ws = WorkSize1D::new(8);
	cq.sync_work(&kernel, &ws).unwrap();
	let out = cq.read(&c_bf).unwrap();
	
	let expected = vec![16i32, 1, 13, 7, 7, 7, 7];
	if !check_result(&expected, &out) {
		println!("Vectors are not equal {:?} {:?}", expected, out);
	} else {
		println!("Vectors are equal");
	}
}

pub fn get_demo_code() -> &'static str {
	"__kernel void vector_add(__global const long *A, __global const long *B, __global long *C) {
                int i = get_global_id(0);
                C[i] = A[i] + B[i];
    }"
}

fn get_context() -> (Context, Vec<Rc<Device>>) {
	let pq = PlatformQuery::Index(0);
    let platform = ClRoot::get_platform(&pq).unwrap();

    let dq = DeviceQuery::Type(DeviceType::All);
    let devices = platform.get_devices_query(&dq);

    // TODO: Avoid this clones
    (Context::from_devices(&devices), devices.clone())
}

fn check_result<T>(expected: &Vec<T>, got: &Vec<T>) -> bool where T: Eq {
	let mut valid = true;

	for (i, _) in expected.iter().enumerate() {
		if expected[i] != got[i] {
			valid = false;
			break;
		}
	}

	valid
}