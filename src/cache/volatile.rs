use std::collections::BTreeMap;
use cache::CacheBackend;

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
    fn get(&self, key: &String) -> Option<Vec<u8>> {
        let content = self.map.get(key);

        match content {
        	None => None,
        	Some(binary) => Some(binary.clone()),
        }
    }

    fn put(&mut self, key: &String, payload: &Vec<u8>) {
    	self.map.insert(key.clone(), payload.clone());
    }
}

#[cfg(test)]
mod test {
	use super::*;
	use cache::CacheBackend;

	#[test]
	fn it_returns_none_on_empty_cache() {
		let c = Volatile::new();
		assert!(c.get(&("test".to_string())).is_none());
	}
}