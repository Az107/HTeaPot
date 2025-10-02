use std::hash::{Hash, Hasher};
use std::{
    collections::{HashMap, hash_map},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone)]
struct CaseInsensitiveString(String);

impl PartialEq for CaseInsensitiveString {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl Eq for CaseInsensitiveString {}

impl Hash for CaseInsensitiveString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for b in self.0.bytes() {
            state.write_u8(b.to_ascii_lowercase());
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Headers(HashMap<String, String>);

impl Headers {
    pub fn new() -> Self {
        Headers(HashMap::new())
    }

    pub fn insert(&mut self, key: &str, value: &str) {
        // Ejemplo: forzar keys a lowercase
        self.0.insert(key.to_lowercase(), value.to_string());
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.0.get(&key.to_lowercase())
    }
}

impl IntoIterator for Headers {
    type Item = (String, String);
    type IntoIter = hash_map::IntoIter<String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Headers {
    type Item = (&'a String, &'a String);
    type IntoIter = hash_map::Iter<'a, String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut Headers {
    type Item = (&'a String, &'a mut String);
    type IntoIter = hash_map::IterMut<'a, String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl PartialEq for Headers {
    fn eq(&self, other: &Self) -> bool {
        other.0 == self.0
    }
}

impl Deref for Headers {
    type Target = HashMap<String, String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Headers {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[macro_export]
macro_rules! headers {
    ( $($k:expr => $v:expr),* $(,)? ) => {{
        let mut headers = crate::hteapot::HttpHeaders::new();
        $( headers.insert($k, $v); )*
        Some(headers)
    }};
}
