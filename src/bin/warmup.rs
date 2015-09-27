#![feature(convert)]
extern crate clcache;
#[macro_use] extern crate clap;
extern crate ansi_term;

use clap::App;
use clcache::cache::Cache;
use clcache::cache::disk::FileSystemCache;
use clcache::cl::device::Device;
use clcache::cl::context::Context;
use clcache::cl::platform::Platform;
use clcache::cache::CacheError;

use std::fs::*;
use std::fs;
use std::str;
use std::io::Read;
use std::io::Result;
use std::path::*;

use ansi_term::Colour::*;

pub fn main() {
	let yaml = load_yaml!("warmup.yml");
    let matches = App::from_yaml(yaml).get_matches();
    let src_dir = matches.value_of("src_directory").unwrap().to_string();
    let out_dir = matches.value_of("out_directory").unwrap().to_string();
    let recursive = matches.is_present("recursive");
    let extension = matches.value_of("extension").unwrap_or("cl");

	let mut cache = Cache::new(Box::new(FileSystemCache::new(out_dir).unwrap()));
    let (ctx, devices) = get_context();


    let mut entry_cb = move |path: &PathBuf| {
    	match path.extension() {
    		None => (),
    		Some(ext) => if ext == extension {
				let mut file = File::open(path).unwrap();
				let mut buffer: Vec<u8> = Vec::new();
				file.read_to_end(&mut buffer).unwrap();

				let result = cache.get_with_options(
					unsafe {str::from_utf8_unchecked(buffer.as_slice())},
					&devices,
					&ctx,
					""
				);

				match result {
					Ok(_) => println!("No errors getting from cache. File: {}", Green.bold().paint(path.to_str().unwrap())),
					Err(CacheError::CacheError) => println!("Some GENERIC error when getting from cache. File: {}", Red.bold().paint(path.to_str().unwrap())),
					Err(CacheError::ClBuildError(hm)) => {
						println!("Got some error compiling the program. File: {}", Red.bold().paint(path.to_str().unwrap()));
						for (device, log) in hm {
							println!("Device: {}", Cyan.bold().paint(&device.get_name().unwrap()));
							println!("{}", log);
						}
					}
					Err(CacheError::ClError(error)) => {
						println!("Got some error opencl-related on file: {}", Red.bold().paint(path.to_str().unwrap()));
						println!("{:?}", error);
					},
					Err(_) => {
						println!("Got some unknown error on file: {}", Red.bold().paint(path.to_str().unwrap()));
					}
				}
    		},
    	}
    };

    visit_dirs(&PathBuf::from(src_dir), recursive, &mut entry_cb).unwrap();

    
}

fn visit_dirs<'a>(dir: &PathBuf, recursive: bool, cb: &mut FnMut(&PathBuf)) -> Result<()> {
	let top = try!(fs::metadata(dir));

    if top.is_dir() {
        for entry in try!(fs::read_dir(dir)) {
            let entry = try!(entry);
            if try!(fs::metadata(entry.path())).is_dir() {
            	if recursive {
                	try!(visit_dirs(&entry.path(), recursive, cb));
            	}
            } else {
                cb(&entry.path());
            }
        }
    } else {
    	cb(&dir);
    }

    Ok(())
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