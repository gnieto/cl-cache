use cache::CacheBackend;
use std::fs::*;
use std::io::Write;
use std::io::Read;
use std::io::Result;

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

	pub fn create_dir(path: &String) -> Result<()> {
		Ok(try!{create_dir_all(path)})
	}

	fn get_path(&self, key: &String) -> String {
    	format!("{}/{}.clbin", self.path.clone(), key)
    }
}

impl CacheBackend for FileSystemCache {
    fn get(&self, key: &String) -> Option<Vec<u8>> {
        let final_path = self.get_path(&key);
     	let file_open_result = File::open(final_path);

     	if file_open_result.is_err() {
     		return None;
     	}

     	let mut file = file_open_result.unwrap();
     	let mut buffer: Vec<u8> = Vec::new();
     	let result = file.read_to_end(&mut buffer);

     	match result {
     		Ok(_) => Some(buffer),
     		Err(_) => None,
     	}
    }

    fn put(&mut self, key: &String, payload: &Vec<u8>) {
    	let final_path = self.get_path(&key);
    	let file_descriptor = File::create(final_path);

        if file_descriptor.is_err() {
            return;
        }

        let mut f = file_descriptor.unwrap();
    	f.write_all(payload).unwrap();
    }
}