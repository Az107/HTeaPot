use std::collections::hash_map::Entry;
use std::collections::{HashMap, hash_map};
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

#[derive(Debug, Clone)]
pub struct CaseInsensitiveString(String);

impl PartialEq for CaseInsensitiveString {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl Display for CaseInsensitiveString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
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

impl Deref for CaseInsensitiveString {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Default, Clone)]
pub struct Headers(HashMap<CaseInsensitiveString, String>);

impl Headers {
    pub fn new() -> Self {
        Headers(HashMap::new())
    }

    pub fn insert(&mut self, key: &str, value: &str) {
        // Ejemplo: forzar keys a lowercase
        self.0
            .insert(CaseInsensitiveString(key.to_string()), value.to_string());
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.0.get(&CaseInsensitiveString(key.to_string()))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn entry(&mut self, key: &str) -> Entry<'_, CaseInsensitiveString, String> {
        self.0.entry(CaseInsensitiveString(key.to_string()))
    }

    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.0.remove(&CaseInsensitiveString(key.to_string()))
    }
}

impl IntoIterator for Headers {
    type Item = (CaseInsensitiveString, String);
    type IntoIter = hash_map::IntoIter<CaseInsensitiveString, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Headers {
    type Item = (&'a CaseInsensitiveString, &'a String);
    type IntoIter = hash_map::Iter<'a, CaseInsensitiveString, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut Headers {
    type Item = (&'a CaseInsensitiveString, &'a mut String);
    type IntoIter = hash_map::IterMut<'a, CaseInsensitiveString, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl PartialEq for Headers {
    fn eq(&self, other: &Self) -> bool {
        other.0 == self.0
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

#[cfg(test)]
#[test]
fn test_caseinsensitive() {
    let mut headers = Headers::new();
    headers.insert("X-Test-Header", "Value");
    assert!(headers.get("x-test-header").is_some());
    assert!(headers.get("x-test-header").unwrap() == "Value");
    assert!(headers.get("x-test-header").unwrap() != "value");
}

#[cfg(test)]
#[test]
fn test_remove() {
    let mut headers = Headers::new();
    headers.insert("X-Test-Header", "Value");
    assert!(headers.get("x-test-header").is_some());
    assert!(headers.get("x-test-header").unwrap() == "Value");
    assert!(headers.get("x-test-header").unwrap() != "value");
    assert!(headers.remove("x-test-header").is_some());
    assert!(headers.get("x-test-header").is_none());
}
