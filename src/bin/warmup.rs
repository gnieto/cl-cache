#![feature(convert)]
extern crate clcache;
#[macro_use] extern crate clap;
extern crate ansi_term;
extern crate regex;

use clap::App;
use clap::ArgMatches;
use clcache::cache::Cache;
use clcache::cache::disk::FileSystemCache;
use clcache::cl::device::Device;
use clcache::cl::platform::DeviceQuery;
use clcache::cl::platform::DeviceType;
use clcache::cl::context::Context;
use clcache::cl::platform::Platform;
use clcache::cl::cl_root::ClRoot;
use clcache::cl::cl_root::PlatformQuery;
use clcache::cache::CacheError;
use regex::Regex;

use std::fs::*;
use std::fs;
use std::str;
use std::io::Read;
use std::io::Result;
use std::path::*;
use std::str::FromStr;

use ansi_term::Colour::*;

struct WarmupJob {
	src: String,
	out: String,
	options: String,
	platform: String,
	devices: String,
	recursive: bool,
	extension: String,
	verbose: u8,
}

impl WarmupJob {
	pub fn new(src: String, out: String, options: String, platform: String, devices: String, extension: String) -> WarmupJob {
		WarmupJob {
			src: src,
			out: out,
			options: options,
			platform: platform,
			devices: devices,
			extension: extension,	
			recursive: false,
			verbose: 0,
		}
	}
}

pub fn main() {
	let yaml = load_yaml!("warmup.yml");
    let matches = App::from_yaml(yaml).get_matches();
    let src_dir = matches.value_of("src_directory").unwrap().to_string();
    let out_dir = matches.value_of("out_directory").unwrap_or(".").to_string();
    let recursive = matches.is_present("recursive");
    let extension = matches.value_of("extension").unwrap_or("cl");
    let verbosity_level = matches.occurrences_of("verbosity");
    let platform_str = matches.value_of("platform").unwrap_or("0");
    let device_query = get_device_query(&matches);

    let possible_platform = get_platform(platform_str);
    if possible_platform.is_none() {
    	println!(
    		"{}: No platform found with expression: {}",
    		Red.bold().paint("Error"),
    		Cyan.bold().paint(platform_str)
    	);

    	return
    }
    let platform = possible_platform.unwrap();

	let mut cache = Cache::new(Box::new(FileSystemCache::new(out_dir).unwrap()));
    let (ctx, devices) = get_context(&platform, &device_query);

    if devices.len() == 0 {
		println!(
    		"{} No devices found with query: {:?}",
    		Red.bold().paint("Error: "),
    		device_query
    	);

    	return
    }

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
							if verbosity_level > 0 {
								println!("Device: {}", Cyan.bold().paint(&device.get_name().unwrap()));
								println!("{}", log);
							}
						}
					}
					Err(CacheError::ClError(error)) => {
						println!("Got some error opencl-related on file: {}", Red.bold().paint(path.to_str().unwrap()));
						if verbosity_level > 0 {
							println!("{:?}", error);
						}
					},
					Err(_) => {
						println!("Got some unknown error on file: {}", Red.bold().paint(path.to_str().unwrap()));
					}
				}
    		},
    	}
    };

    let buffer = PathBuf::from(src_dir);
    let visit_result = visit_dirs(&buffer, recursive, &mut entry_cb);
    if visit_result.is_err() {
    	println!("Some error occured visting dir: {}", Red.bold().paint(&buffer.to_str().unwrap()));

    	if verbosity_level > 0 {
    		println!(
    			"{}: {:?}",
    			Red.bold().paint("Error"),
    			visit_result.err()
    		);
    	}
    }
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

fn get_device_query(matches: &ArgMatches) -> DeviceQuery {
	let arg_present = (
		matches.is_present("device_type"),
		matches.is_present("device_index"),
		matches.is_present("device_regexp"),
	);

	let dq = match arg_present {
		(true, _, _) => {
			let dtype = matches.value_of("device_type").unwrap();

			let device_type = match dtype {
				"cpu" => DeviceType::CPU,
				"gpu" => DeviceType::GPU,
				"all" | _ => DeviceType::All,
			};

			 DeviceQuery::Type(device_type)
		},
		(_, true, _) => {
			let str_index = matches.value_of("device_index").unwrap();
			let index =  match str_index.parse::<usize>() {
				Err(_) => {
					println!("'device_index' not numeric: Using index 0");
					0
				},
				Ok(i) => i,
			};

			DeviceQuery::Index(index)
		},
		(_, _, true) => {
			let regex = Regex::new(matches.value_of("device_regexp").unwrap()).unwrap();
			DeviceQuery::Regexp(regex.clone())
		},
		_ => DeviceQuery::Type(DeviceType::All),
	};

	dq
}

fn get_platform(str_plaform: &str) -> Option<Platform> {
	// let x: usize = from_str(str_plaform);
	let x = str_plaform.parse::<usize>();

	match x {
		Err(_) => {
			let regex = Regex::new(str_plaform).unwrap();
			let pq = PlatformQuery::Regexp(&regex);
			ClRoot::get_platform(&pq)
		},
		Ok(x) => {
			let pq = PlatformQuery::Index(x);
			ClRoot::get_platform(&pq)
		}
	}
}

fn get_context(platform: &Platform, device_query: &DeviceQuery) -> (Context, Vec<Device>) {
    let devices = platform.get_devices_query(&device_query);

    // TODO: Avoid this clones
    (Context::from_devices(devices.clone()), devices.clone())
}