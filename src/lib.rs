#![feature(static_mutex)]
#![feature(cstr_to_str)]

extern crate opencl;
extern crate crypto;
extern crate libc;
extern crate regex;

pub mod cache;
pub mod cl;

use cache::{Cache, CacheError};
use cache::disk::FileSystemCache;
use std::cell::RefCell;
use std::ffi::CStr;
use std::rc::Rc;
use opencl::cl::*;
use cl::device::Device;
use cl::program::Program;
use cl::context::Context;
use std::sync::Arc;
use std::ptr;
// use std::ffi::CStr;

thread_local!(static CACHE_CONT: RefCell<CacheContainer> = RefCell::new(CacheContainer::new()));

struct CacheContainer {
	caches: Vec<Arc<RefCell<Cache>>>,
}

impl CacheContainer {
	pub fn new() -> CacheContainer {
		CacheContainer {
			caches: Vec::new(),
		}
	}

	pub fn put(&mut self, cache: Arc<RefCell<Cache>>) -> u32 {
		self.caches.push(cache.clone());

		self.caches.len() as u32 - 1
	}

	pub fn get(&self, index: u32) -> Option<Arc<RefCell<Cache>>> {
		if (index as usize) < self.caches.len() {
			Some(self.caches[index as usize].clone())
		} else {
			None
		}
	}
}

#[no_mangle]
pub extern "C" fn cl_cache_create_fs(path: *const libc::c_char) -> i32 {
	// Put a mutex here
	let path = unsafe {CStr::from_ptr(path).to_string_lossy().into_owned()};
	let fs_cache = FileSystemCache::new(path.to_string());
	if fs_cache.is_none() {
		return -1;
	}

	let backend = Box::new(fs_cache.unwrap());
	let cache = Cache::new(backend);

	add_cache(RefCell::new(cache))
}

#[no_mangle]
pub extern "C" fn cl_cache_get(
	cache_id: i32,
	source: *const libc::c_char,
	num_devices: u8,
	devices: *const libc::c_void,
	context: *const libc::c_void
) -> *mut cl_program
{
	let source_str = unsafe{ CStr::from_ptr(source).to_str() };
	if source_str.is_err() {
		return ptr::null_mut();
	}

	let source_cstr = source_str.unwrap();
	
	let cache_result = get_cache(cache_id as usize);
	if cache_result.is_none() {
		return ptr::null_mut();
	}
	let cache = cache_result.unwrap();

	let devices_vec = get_devices_vector(num_devices, devices);
	let context = Context::from_id(context as cl_context);

 	let get_result = cache.borrow_mut().get(
 		&source_cstr,
 		&devices_vec,
 		&context
 	);

 	return_from_program_result(get_result)
}

#[no_mangle]
pub extern "C" fn cl_cache_get_with_tag(
	cache_id: i32,
	tag: *const libc::c_char,
	num_devices: u8,
	devices: *const libc::c_void,
	context: *const libc::c_void
) -> *mut cl_program
{
	let tag_str = unsafe{ CStr::from_ptr(tag).to_str() };
	if tag_str.is_err() {
		return ptr::null_mut();
	}

	let tag_cstr = tag_str.unwrap();
	let cache_result = get_cache(cache_id as usize);
	if cache_result.is_none() {
		return ptr::null_mut();
	}
	let cache = cache_result.unwrap();
	let devices_vec = get_devices_vector(num_devices, devices);
	let context = Context::from_id(context as cl_context);

 	let get_result = cache.borrow_mut().get_with_tag(
 		&tag_cstr,
 		&devices_vec,
 		&context
 	);

 	return_from_program_result(get_result)
}

#[no_mangle]
pub extern "C" fn cl_cache_put_with_tag(
	cache_id: i32,
	tag: *const libc::c_char,
	num_devices: u8,
	devices: *const libc::c_void,
	program: *const libc::c_void
) -> i32
{
	let tag_str = unsafe{ CStr::from_ptr(tag).to_str() };
	if tag_str.is_err() {
		return 0;
	}

	let tag_cstr = tag_str.unwrap();
	
	let cache_result = get_cache(cache_id as usize);
	if cache_result.is_none() {
		return 0;
	}
	let cache = cache_result.unwrap();

	let devices_vec = get_devices_vector(num_devices, devices);
	let program = Program::from_cl_program(program as cl_program);

 	let put_result = cache.borrow_mut().put_with_tag(
 		&tag_cstr,
 		&devices_vec,
 		&program
 	);

 	if put_result.is_err() {
 		0
 	} else { 	
 		1
 	}
}

#[no_mangle]
pub extern "C" fn cl_cache_get_with_options(
	cache_id: i32,
	source: *const libc::c_char,
	num_devices: u8,
	devices: *const libc::c_void,
	context: *const libc::c_void,
	options: *const libc::c_char
) -> *mut cl_program
{
	let source_str = unsafe{ CStr::from_ptr(source).to_str() };
	if source_str.is_err() {
		return ptr::null_mut();
	}
	let source_cstr = source_str.unwrap();

	let option_str = unsafe{ CStr::from_ptr(options).to_str() };
	if option_str.is_err() {
		return ptr::null_mut();
	}
	let options_cstr = option_str.unwrap();

	let cache = get_cache(cache_id as usize).unwrap();
	
	let context = Context::from_id(context as cl_context);
	let devices_vec = get_devices_vector(num_devices, devices);

 	let get_result = cache.borrow_mut().get_with_options(
 		&source_cstr,
 		&devices_vec,
 		&context,
 		&options_cstr,
 	);

 	return_from_program_result(get_result)
}

fn get_devices_vector(num_devices: u8, devices: *const libc::c_void) -> Vec<Rc<Device>> {
	let devices = unsafe {std::slice::from_raw_parts(devices as *const cl_device_id, std::mem::size_of::<cl_device_id>() * num_devices as usize)};
	let mut devices_vec = Vec::new();

	for i in (0..num_devices) {
		devices_vec.push(Rc::new(Device::from_device_id(devices[i as usize])));
	}

	devices_vec
}

fn return_from_program_result(result: Result<Program, CacheError>) -> *mut cl_program {
	if result.is_err() {
 		let p: *mut cl_program = ptr::null_mut();
 		
 		p
 	} else { 	
 		let program = result.unwrap();

 		program.get_id() as *mut *mut libc::c_void
 	}
}

fn add_cache(cache: RefCell<Cache>) -> i32 {
	CACHE_CONT.with(|ref_caches| {
		let ref mut caches = *ref_caches.borrow_mut();

		caches.put(Arc::new(cache)) as i32
	})
}

fn get_cache<'a>(index: usize) -> Option<Arc<RefCell<Cache>>> {
	CACHE_CONT.with(|ref_caches| {
		let ref caches = *ref_caches.borrow();

		caches.get(index as u32)
	})
}
