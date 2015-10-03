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
use clcache::cl::cl_root::ClRoot;
use clcache::cl::cl_root::PlatformQuery;
use clcache::cache::CacheError;
use regex::Regex;

use std::fs::*;
use std::fs;
use std::str;
use std::io::Read;
use std::io;
use std::result::Result;
use std::path::*;

use ansi_term::Colour::*;

pub fn main() {
	let yaml = load_yaml!("warmup.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let mut job = WarmupJob::from_arg_matches(&matches).unwrap();
    let job_result = job.execute();

    if job_result.is_err() {
    	println!("{} {}", Red.bold().paint("Job error:"), job_result.err().unwrap());
    }
}

struct WarmupJob {
	src: String,
	options: String,
	devices: Vec<Device>,
	context: Context,
	recursive: bool,
	extension: String,
	verbose: u8,
	cache: Cache,
}

impl WarmupJob {
	pub fn from_arg_matches(matches: &ArgMatches) -> Result<WarmupJob, String> {
		let out = matches.value_of("out_directory").unwrap_or(".").to_string(); 

		let platform_str = Self::get_platform_query(&matches);
		let opt_platform = ClRoot::get_platform(&platform_str);
		if opt_platform.is_none() {
			return Err(format!("No platform found with query: {:?}", platform_str));
		}

		let platform = opt_platform.unwrap();

		let devices_query = Self::get_device_query(&matches);
		let devices = platform.get_devices_query(&devices_query);
		if devices.len() == 0 {
			return Err(format!("No devices found with query: {:?}", devices_query));
		}

		let context = Context::from_devices(&devices);

		let job = WarmupJob {
			src: matches.value_of("src_directory").unwrap().to_string(),
			options: "".to_string(),
			devices: devices,
			context: context,
			extension: matches.value_of("extension").unwrap_or("cl").to_string(),
			recursive: matches.is_present("recursive"),
			verbose: matches.occurrences_of("verbosity"),
			cache: Cache::new(Box::new(FileSystemCache::new(out).unwrap())),
		};

		Ok(job)
	}

	pub fn execute(&mut self) -> Result<(), String> {
		let buffer = PathBuf::from(&self.src);
		let recursive = self.recursive;
	    let visit_result = self.visit_dirs(&buffer, recursive);
	    if visit_result.is_err() {
	    	println!("Some error occured visting dir: {}", Red.bold().paint(&buffer.to_str().unwrap()));

	    	if self.verbose > 0 {
	    		println!(
	    			"{}: {:?}",
	    			Red.bold().paint("Error"),
	    			visit_result.err()
	    		);
	    	}
	    }

		Ok(())
	}

	fn visit_path_buffer(&mut self, path: &PathBuf) {
		match path.extension() {
    		None => (),
    		Some(ext) => if ext == self.extension.as_str() {
				let mut file = File::open(path).unwrap();
				let mut buffer: Vec<u8> = Vec::new();
				file.read_to_end(&mut buffer).unwrap();

				let result = self.cache.get_with_options(
					unsafe {str::from_utf8_unchecked(buffer.as_slice())},
					&self.devices,
					&self.context,
					&self.options
				);

				match result {
					Ok(_) => println!("No errors getting from cache. File: {}", Green.bold().paint(path.to_str().unwrap())),
					Err(CacheError::CacheError) => println!("Some GENERIC error when getting from cache. File: {}", Red.bold().paint(path.to_str().unwrap())),
					Err(CacheError::ClBuildError(hm)) => {
						println!("Got some error compiling the program. File: {}", Red.bold().paint(path.to_str().unwrap()));
						for (device, log) in hm {
							if self.verbose > 0 {
								println!("Device: {}", Cyan.bold().paint(&device.get_name().unwrap()));
								println!("{}", log);
							}
						}
					}
					Err(CacheError::ClError(error)) => {
						println!("Got some error opencl-related on file: {}", Red.bold().paint(path.to_str().unwrap()));
						if self.verbose > 0 {
							println!("{:?}", error);
						}
					},
					Err(_) => {
						println!("Got some unknown error on file: {}", Red.bold().paint(path.to_str().unwrap()));
					}
				}
    		},
    	}
	}

	fn visit_dirs(&mut self, dir: &PathBuf, recursive: bool) -> io::Result<()> {
		let top = try!(fs::metadata(dir));

	    if top.is_dir() {
	        for entry in try!(fs::read_dir(dir)) {
	            let entry = try!(entry);
	            if try!(fs::metadata(entry.path())).is_dir() {
	            	if recursive {
	                	try!(self.visit_dirs(&entry.path(), recursive));
	            	}
	            } else {
	                self.visit_path_buffer(&entry.path());
	            }
	        }
	    } else {
	    	self.visit_path_buffer(&dir);
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

	fn get_platform_query(matches: &ArgMatches) -> PlatformQuery {
		let platform_str = matches.value_of("platform").unwrap_or("0");
		let x = platform_str.parse::<usize>();

		match x {
			Err(_) => {
				let regex = Regex::new(platform_str).unwrap();
				PlatformQuery::Regexp(regex.clone())
			},
			Ok(x) => {
				PlatformQuery::Index(x)
			}
		}
	}
}