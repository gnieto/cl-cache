#![feature(convert)]
extern crate clcache;
#[macro_use] extern crate clap;
extern crate ansi_term;
extern crate regex;
extern crate yaml_rust;

use clap::App;
use clap::ArgMatches;
use clcache::cache::Cache;
use clcache::cache::disk::FileSystemCache;
use clcache::cl::device::Device;
use clcache::cl::platform::DeviceQuery;
use clcache::cl::platform::DeviceType;
use clcache::cl::context::Context;
use clcache::cl::program::Program;
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
use std::path::PathBuf;
use yaml_rust::{YamlLoader, Yaml, ScanError};
use std::rc::Rc;
use std::fmt::{Display, Formatter, Error};
use std::ffi::OsStr;

use ansi_term::Colour::*;

pub fn main() {
	let yaml = load_yaml!("warmup_clap.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let mut jobs = if let Some(yaml) = matches.value_of("yaml") {
		let possible_jobs = load_from_yaml(yaml);

    	if possible_jobs.is_err() {
    		println!("Could not decode the target YAML: {}", yaml);
    		println!("Reason: {}", possible_jobs.err().unwrap());
    		return
    	}

    	possible_jobs.unwrap()
    } else {
    	let mut jobs = Vec::new();

    	if let Ok(job) = WarmupJob::from_decoder(Box::new(&matches)) {
			jobs.push(job);
    	}

	    jobs
    };

    while let Some(mut job) = jobs.pop() {
    	if job.name.is_some() {
    		let job_name = job.name.clone().unwrap();
    		println!("Executing job: {}", Cyan.bold().paint(&job_name));
    	}

    	let job_result = job.execute();

    	if job_result.is_err() {
    		println!(
    			"{} {:?}",
    			Red.bold().paint("Error: "),
    			job_result.err(),
    		);
    	}
    }
}

fn load_from_yaml(file: &str) -> Result<Vec<WarmupJob>, YamlError> {
	let mut file = try!{File::open(file)};
 	let mut file_str = Vec::new();

 	try!{file.read_to_end(&mut file_str)};
 	let str_result = unsafe{ String::from_utf8_unchecked(file_str) };
 	let ref yaml_result = try!{YamlLoader::load_from_str(&str_result)};
 	let doc = &yaml_result[0];

 	let empty_yaml_jobs = Vec::new();
 	let yaml_jobs = doc["jobs"].as_vec().unwrap_or(&empty_yaml_jobs);

	let mut jobs = Vec::new();

	for job_yaml in yaml_jobs {
		if job_yaml.as_hash().is_none() {
			return Err(YamlError::new("Jobs child is not a hash".to_string()));
		}

		for (job_name, job_hash) in job_yaml.as_hash().unwrap() {
			if job_hash["source"].is_badvalue() {
				return Err(YamlError::new("Some of the childs are not a hash or 'source' field not found".to_string()));
			}

			let maybe_job = WarmupJob::from_decoder(Box::new(job_hash));
			if maybe_job.is_ok() {
				let mut job = maybe_job.unwrap();

				if job_name.as_str().is_some() {
					job.set_name(job_name.as_str().unwrap().to_string());
				}
				
				jobs.push(job);
			}
		}
	}

	Ok(jobs)
}

#[derive(Debug)]
struct YamlError {
	error: String,
}

impl YamlError {
	pub fn new(str_error: String) -> YamlError {
		YamlError {
			error: str_error,
		}
	}
}

impl Display for YamlError {
	fn fmt(&self, formatter: &mut Formatter) -> Result<(), Error> {
		formatter.write_str(&self.error)
	}
}

impl From<std::io::Error> for YamlError {
	fn from(error: std::io::Error) -> YamlError {
		YamlError {
			error: format!{"Could not open file {}", error}
		}
	}
}

impl From<ScanError> for YamlError {
	fn from(error: ScanError) -> YamlError {
		YamlError {
			error: format!{"Error while decoding yaml file: {}", error}
		}
	}
}

struct WarmupJob {
	src: String,
	options: String,
	devices: Vec<Rc<Device>>,
	context: Context,
	recursive: bool,
	extension: String,
	verbose: u8,
	cache: Cache,
	tagged: Option<String>,
	pub name: Option<String>,
}

trait JobDecoder {
	fn get_input(&self) -> Option<String>;
	fn get_output(&self) -> String;
	fn get_options(&self) -> String;
	fn get_device_query(&self) -> DeviceQuery;
	fn get_platform_query(&self) -> PlatformQuery;
	fn get_recursive(&self) -> bool;
	fn get_extension(&self) -> String;
	fn get_verbose(&self) -> u8;
	fn get_tag(&self) -> Option<String>;
}

impl<'a, 'b> JobDecoder for ArgMatches<'a, 'b> {
	fn get_device_query(&self) -> DeviceQuery {
		let arg_present = (
			self.is_present("device_type"),
			self.is_present("device_index"),
			self.is_present("device_regexp"),
		);

		let dq = match arg_present {
			(true, _, _) => {
				let dtype = self.value_of("device_type").unwrap();

				let device_type = match dtype {
					"cpu" => DeviceType::CPU,
					"gpu" => DeviceType::GPU,
					"all" | _ => DeviceType::All,
				};

				 DeviceQuery::Type(device_type)
			},
			(_, true, _) => {
				let str_index = self.value_of("device_index").unwrap();
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
				if let Ok(regex) = Regex::new(self.value_of("device_regexp").unwrap()) {
					DeviceQuery::Regexp(regex.clone())	
				} else {
					DeviceQuery::Type(DeviceType::All)		
				}
				
			},
			_ => DeviceQuery::Type(DeviceType::All),
		};

		dq
	}

	fn get_input(&self) -> Option<String> {
		if self.is_present("src_directory") {
			Some(self.value_of("src_directory").unwrap().to_string())
		} else {
			None
		}
	}

	fn get_output(&self) -> String {
		self.value_of("out_directory").unwrap_or(".").to_string()
	}

	fn get_options(&self) -> String {
		self.value_of("options").unwrap_or("").to_string()
	}

	fn get_platform_query(&self) -> PlatformQuery {
		let platform_str = self.value_of("platform").unwrap_or("0");
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

	fn get_recursive(&self) -> bool {
		self.is_present("recursive")
	}

	fn get_extension(&self) -> String {
		self.value_of("extension").unwrap_or("cl").to_string()
	}

	fn get_verbose(&self) -> u8 {
		self.occurrences_of("verbosity")
	}

	fn get_tag(&self) -> Option<String> {
		None
	}
}

impl JobDecoder for Yaml {
	fn get_device_query(&self) -> DeviceQuery {
		let (device_type, device_index, device_regex) = (
			self["device_type"].as_str(),
			self["device_index"].as_i64(),
			self["device_regex"].as_str()
		);

		if device_type.is_some() {
			let dtype = match device_type {
				Some("cpu") => DeviceType::CPU,
				Some("gpu") => DeviceType::GPU,
				Some("all") | _ => DeviceType::All,
			};

			DeviceQuery::Type(dtype)
		} else if device_index.is_some() {
			let index =  match device_index {
				None => {
					println!("'device_index' not numeric: Using index 0");
					0
				},
				Some(i) => i as usize,
			};

			DeviceQuery::Index(index)
		} else if device_regex.is_some() {
			let regex = Regex::new(device_regex.unwrap()).unwrap();
			DeviceQuery::Regexp(regex.clone())
		} else {
			DeviceQuery::Index(0)
		}

	}

	fn get_input(&self) -> Option<String> {
		match self["source"].as_str() {
			None => None,
			Some(input) => Some(input.to_string()),
		}
	}

	fn get_output(&self) -> String {
		return self["output"].as_str().unwrap_or(".").to_string();
	}

	fn get_options(&self) -> String {
		return self["options"].as_str().unwrap_or("").to_string();
	}

	fn get_platform_query(&self) -> PlatformQuery {
		let index = self["platform_index"].as_i64().unwrap_or(0);
		let regex = self["platform_regex"].as_str();

		match regex {
			None => {
				PlatformQuery::Index(index as usize)
			},
			Some(x) => {
				let regex = Regex::new(x).unwrap();
				PlatformQuery::Regexp(regex.clone())
			}
		}
	}

	fn get_recursive(&self) -> bool {
		return self["recursive"].as_bool().unwrap_or(false);
	}

	fn get_extension(&self) -> String {
		return self["extension"].as_str().unwrap_or("cl").to_string();
	}

	fn get_verbose(&self) -> u8 {
		return self["options"].as_i64().unwrap_or(0) as u8;
	}

	fn get_tag(&self) -> Option<String> {
		if self["tag"].as_str().is_none() {
			None
		} else {
			Some(self["tag"].as_str().unwrap().to_string())
		}
	}
}

impl WarmupJob {
	pub fn set_name(&mut self, name: String) {
		self.name = Some(name);
	}

	pub fn from_decoder(decoder: Box<&JobDecoder>) -> Result<WarmupJob, String> {
		let out = decoder.get_output();
		let input = decoder.get_input();

		if input.is_none() {
			return Err("No input folder with OpenCl sources specified".to_string());
		}

		let platform_query = decoder.get_platform_query();
		let opt_platform = ClRoot::get_platform(&platform_query);

		if opt_platform.is_none() {
			return Err(format!("No platform found with query: {:?}", platform_query));
		}

		let platform = opt_platform.unwrap();
		let devices_query = decoder.get_device_query();
		let devices = platform.get_devices_query(&devices_query);
		if devices.len() == 0 {
			return Err(format!("No devices found with query: {:?}", devices_query));
		}

		let context = Context::from_devices(&devices);

		if let Some(cache_backend) = FileSystemCache::new(out) {
			let job = WarmupJob {
				src: input.unwrap(),
				options: decoder.get_options(),
				devices: devices,
				context: context,
				extension: decoder.get_extension(),
				recursive: decoder.get_recursive(),
				verbose: decoder.get_verbose(),
				cache: Cache::new(Box::new(cache_backend)),
				tagged: decoder.get_tag(),
				name: None,
			};

			Ok(job)
		} else {
			Err("Could not build a FileSystemCache".to_string())
		}
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
    	let extension = path.extension().unwrap_or(OsStr::new(""));
    	if extension != self.extension.as_str() {
    		return
    	}

    	let result = self.visit_matching_path(&path);
    	self.show_visit_result(&result, &path)

	}

	fn visit_matching_path(&mut self, path: &PathBuf) -> Result<Program, CacheError> {
		let mut file = try!{File::open(path)};
		let mut buffer: Vec<u8> = Vec::new();
		try!{file.read_to_end(&mut buffer)};
		let source = unsafe {str::from_utf8_unchecked(buffer.as_slice())};

		if self.tagged.is_none() {
			self.cache.get_with_options(
				source,
				&self.devices,
				&self.context,
				&self.options
			)
		} else {
	        let tag = self.tagged.clone().unwrap();
	        let program = try!{Program::from_source(&self.context, source)};
	        let build_result = if self.options.len() > 0 {
	        	program.build_with_options(&self.devices, &self.options)
	        } else {
	        	program.build(&self.devices)
	        };

	        if build_result.is_ok() {
	        	let put_result = self.cache.put_with_tag(&tag, &self.devices, &program);
	        	if put_result.is_ok() {
	        		Ok(program)
	        	} else {
	        		Err(put_result.err().unwrap())
	        	}
	        } else {
	        	Err(CacheError::ClError(build_result.err().unwrap()))
	        }
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

	fn show_visit_result(&self, result: &Result<Program, CacheError>, path: &PathBuf) {
		match result {
			&Ok(_) => println!("No errors getting from cache. File: {}", Green.bold().paint(path.to_str().unwrap())),
			&Err(CacheError::CacheError) => println!("Some GENERIC error when getting from cache. File: {}", Red.bold().paint(path.to_str().unwrap())),
			&Err(CacheError::ClBuildError(ref hm)) => {
				println!("Got some error compiling the program. File: {}", Red.bold().paint(path.to_str().unwrap()));
				for (device, log) in hm {
					if self.verbose > 0 {
						println!("Device: {}", Cyan.bold().paint(&device.get_name().unwrap()));
						println!("{}", log);
					}
				}
			}
			&Err(CacheError::ClError(ref error)) => {
				println!("Got some error opencl-related on file: {}", Red.bold().paint(path.to_str().unwrap()));
				if self.verbose > 0 {
					println!("{:?}", error);
				}
			},
			&Err(_) => {
				println!("Got some unknown error on file: {}", Red.bold().paint(path.to_str().unwrap()));
			}
		}
	}
}