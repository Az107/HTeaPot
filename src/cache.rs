// Written by Alberto Ruiz, 2024-11-05
// 
// Config module: handles application configuration setup and parsing.
// This module defines structs and functions to load and validate
// configuration settings from files, environment variables, or other sources.

use std::collections::HashMap;
use std::time;
use std::time::SystemTime;

/// A simple in-memory cache with TTL (time-to-live) support.
///
/// This cache stores byte arrays (`Vec<u8>`) along with an expiration timestamp.
/// When a cached entry is fetched, the TTL is validated. If it's expired, the
/// item is removed and `None` is returned.
///
/// Note: Currently not generic, but could be extended in the future to support
/// other data types.
///
/// # Example
/// ```
/// let mut cache = Cache::new(60); // 60 seconds TTL
/// cache.set("hello".into(), vec![1, 2, 3]);
/// let data = cache.get("hello".into());
/// assert!(data.is_some());
/// ```
pub struct Cache {
    // TODO: consider make it generic
    // The internal store: (data, expiration timestamp)
    data: HashMap<String, (Vec<u8>, u64)>,
    max_ttl: u64,
}

impl Cache {
    /// Creates a new `Cache` with the specified TTL in seconds.
    pub fn new(max_ttl: u64) -> Self {
        Cache {
            data: HashMap::new(),
            max_ttl,
        }
    }

    /// Creates a new `Cache` with the specified TTL in seconds.
    fn validate_ttl(&self, ttl: u64) -> bool {
        let now = SystemTime::now();
        let since_epoch = now
            .duration_since(time::UNIX_EPOCH)
            .expect("Time went backwards");
        let secs = since_epoch.as_secs();
        secs < ttl
    }

    /// Computes the expiration timestamp for a new cache entry.
    fn get_ttl(&self) -> u64 {
        let now = SystemTime::now();
        let since_epoch = now
            .duration_since(time::UNIX_EPOCH)
            .expect("Time went backwards");
        let secs = since_epoch.as_secs();
        secs + self.max_ttl
    }

    /// Stores data in the cache with the given key and a TTL.
    pub fn set(&mut self, key: String, data: Vec<u8>) {
        self.data.insert(key, (data, self.get_ttl()));
    }

    /// Retrieves data from the cache if it exists and hasn't expired.
    ///
    /// Removes and returns `None` if the TTL has expired.
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

        // Alternative implementation using pattern matching
        // This is a more idiomatic way to handle the Option type in Rust.
        // match self.data.get(&key) {
        //     Some((data, ttl)) if self.validate_ttl(*ttl) => Some(data.clone()),
        //     Some(_) => {
        //         self.data.remove(&key);
        //         None
        //     }
        //     None => None,
        // }
    }
}
