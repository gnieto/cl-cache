// use std::collections::BTreeMap;
// use cache::Cache;

/*pub struct CacheRepository {
    caches: BTreeMap<u8, Box<Cache>>,
    counter: u8,
}

impl CacheRepository {
    pub fn new() -> CacheRepository {
        CacheRepository {
            caches: BTreeMap::<u8, Box<Cache>>::new(),
            counter: 0,
        }
    }

    pub fn add(&mut self, cache: Box<Cache>) -> Option<u8> {
        if self.counter >= 255 {
            return None
        }

        let original_counter = self.counter;
        let result = self.caches.insert(self.counter, cache);
        self.counter = self.counter + 1;

        Some(original_counter)
    }

    pub fn get(&self, id: u8) -> Option<&Box<Cache>> {
        self.caches.get(&id)
    }
}

#[cfg(test)]
pub mod test{
    use super::*;
    use cache::Cache;
    use opencl::hl::Program;

    struct TestCache;

    impl Cache for TestCache {
        fn get(&self, program: Program) -> Option<Program> {None}
        fn put(&mut self, program: Program) {}
    }


    #[test]
    fn it_can_add_new_caches() {
        let mut cr = CacheRepository::new();
        assert_eq!(0, cr.add(Box::new(TestCache)).unwrap());
        assert_eq!(1, cr.add(Box::new(TestCache)).unwrap());

    }

    #[test]
    fn it_can_borrow_a_cache() { 
        let mut cr = CacheRepository::new();
        cr.add(Box::new(TestCache));

        let cache = cr.get(0);
    }

    #[test]
    fn it_can_not_add_more_than_u8_caches()
    {
        let mut cr = CacheRepository::new();

        for x in 0..255 {
            let result = cr.add(Box::new(TestCache));
            match result {
                Some(_) => (),
                None => panic!("Should have some cache"),
            }
        }

        cr.add(Box::new(TestCache));
    }

    #[test]
    fn it_returns_none_if_trying_to_get_a_non_existing_cache()
    {
        let mut cr = CacheRepository::new();
  
        let result = cr.get(1);
        match result {
            Some(_) => panic!("Expected 'None' result"),
            _ => ()
        }
    }
}*/
