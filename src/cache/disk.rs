use cache::{CacheBackend, KeyError};
use std::fs::*;
use std::io::Write;
use std::io::Read;
use std::result::Result;
use std::io::Result as IoResult;

pub struct FileSystemCache {
	path: String,
}

impl FileSystemCache {
	pub fn new(path: String) -> Option<FileSystemCache> {
		// Check that route exists, and try to crate it
		let path_meta = metadata(&path);
		match path_meta {
			Err(_) => match Self::create_dir(&path) {
				Err(_) => return None,
				_ => (),
			},
			_ => (),
		}

		let cache = FileSystemCache {
			path: path,
		};

		Some(cache)
	}

	pub fn create_dir(path: &String) ->IoResult<()> {
		Ok(try!{create_dir_all(path)})
	}

	fn get_path(&self, key: &String) -> String {
    	format!("{}/{}.clbin", self.path.clone(), key)
    }
}

impl CacheBackend for FileSystemCache {
    fn get(&self, key: &String) -> Result<Vec<u8>, KeyError> {
        let final_path = self.get_path(&key);
     	let mut file = try!{File::open(final_path)};     
     	let mut buffer: Vec<u8> = Vec::new();
     	try!{file.read_to_end(&mut buffer)};

     	Ok(buffer)
    }

    fn put(&mut self, key: &String, payload: &Vec<u8>) -> Result<(), KeyError> {
    	let final_path = self.get_path(&key);
    	let mut f = try!{File::create(final_path)};
    	try!{f.write_all(payload)};

        Ok(())
    }
}