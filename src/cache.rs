// Written by Alberto Ruiz, 2024-11-05
// Config module: handles application configuration setup and parsing.
// This module defines structs and functions to load and validate
// configuration settings from files, environment variables, or other sources.
use std::collections::HashMap;
use std::time;
use std::time::SystemTime;

pub struct Cache {
    //TODO: consider make it generic
    data: HashMap<String, (Vec<u8>, u64)>,
    max_ttl: u64,
}

impl Cache {
    pub fn new(max_ttl: u64) -> Self {
        Cache {
            data: HashMap::new(),
            max_ttl,
        }
    }

    fn validate_ttl(&self, ttl: u64) -> bool {
        let now = SystemTime::now();
        let since_epoch = now
            .duration_since(time::UNIX_EPOCH)
            .expect("Time went backwards");
        let secs = since_epoch.as_secs();
        secs < ttl
    }

    fn get_ttl(&self) -> u64 {
        let now = SystemTime::now();
        let since_epoch = now
            .duration_since(time::UNIX_EPOCH)
            .expect("Time went backwards");
        let secs = since_epoch.as_secs();
        secs + self.max_ttl
    }

    pub fn set(&mut self, key: String, data: Vec<u8>) {
        self.data.insert(key, (data, self.get_ttl()));
    }

    pub fn get(&mut self, key: String) -> Option<Vec<u8>> {
        let r = self.data.get(&key);
        if r.is_some() {
            let (data, ttl) = r.unwrap();
            if self.validate_ttl(*ttl) {
                Some(data.clone())
            } else {
                self.data.remove(&key);
                None
            }
        } else {
            None
        }
    }
}
