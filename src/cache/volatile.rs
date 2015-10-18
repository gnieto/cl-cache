use std::collections::BTreeMap;
use cache::{CacheBackend, KeyError};

pub struct Volatile {
	map: BTreeMap<String, Vec<u8>>,
}

impl Volatile {
	pub fn new() -> Volatile {
		Volatile {
			map: BTreeMap::new(),
		}
	}
}

impl CacheBackend for Volatile {
    fn get(&self, key: &String) -> Result<Vec<u8>, KeyError> {
        let content = self.map.get(key);

        match content {
        	None => Err(KeyError::KeyNotFound),
        	Some(binary) => Ok(binary.clone()),
        }
    }

    fn put(&mut self, key: &String, payload: &Vec<u8>) -> Result<(), KeyError> {
    	self.map.insert(key.clone(), payload.clone());

    	Ok(())
    }
}

#[cfg(test)]
mod test {
	use super::*;
	use cache::CacheBackend;

	#[test]
	fn it_returns_none_on_empty_cache() {
		let c = Volatile::new();
		assert!(c.get(&("test".to_string())).is_err());
	}
}