// Written by Alberto Ruiz 2024-04-07 (Happy 3th monthsary)
//
// This is the config module: responsible for loading application configuration
// from a file and providing structured access to settings.

use std::{any::Any, collections::HashMap, fs, path::Path};

/// Dynamic TOML value representation.
///
/// Each parsed TOML key is stored as a `TOMLtype`, which can be a string,
/// number, float, or boolean. This allows the custom parser to support basic
/// TOML configuration without external dependencies.
#[derive(Clone, Debug)]
pub enum TOMLtype {
    Text(String),
    Number(u16),
    Float(f64),
    Boolean(bool),
}

/// A section of the parsed TOML file, keyed by strings and holding `TOMLtype` values.
type TOMLSchema = HashMap<String, TOMLtype>;

/// Trait for safely extracting typed values from a `TOMLSchema`.
trait Schema {
    /// Attempts to retrieve a value of type `T` from the schema by key.
    fn get2<T: 'static + Clone>(&self, key: &str) -> Option<T>;
}

impl Schema for TOMLSchema {
    fn get2<T: 'static + Clone>(&self, key: &str) -> Option<T> {
        let value = self.get(key)?;
        let value = value.clone();

        // Convert the TOMLtype to a dynamically typed value
        let any_value: Box<dyn Any> = match value {
            TOMLtype::Text(d) => Box::new(d),
            TOMLtype::Number(d) => Box::new(d),
            TOMLtype::Float(d) => Box::new(d),
            TOMLtype::Boolean(d) => Box::new(d),
        };

        // Try to downcast to the requested type
        let r = any_value.downcast_ref::<T>().cloned();
        if r.is_none() {
            println!("{} is none", key);
        }
        r
    }
}

/// Parses a TOML-like string into a nested `HashMap` structure.
///
/// This is a minimal, custom TOML parser that supports:
/// - Sections (e.g., `[HTEAPOT]`)
/// - Key-value pairs with types: string, bool, u16, f64
/// - Ignores comments and blank lines
///
/// # Panics
/// Panics if a numeric or float value fails to parse.
pub fn toml_parser(content: &str) -> HashMap<String, TOMLSchema> {
    let mut map = HashMap::new();
    let mut submap = HashMap::new();
    let mut title = "".to_string();

    let lines = content.split("\n");
    for line in lines {
        if line.starts_with("#") || line.is_empty() {
            continue;
        }

        // Remove trailing inline comments
        let line = if line.contains('#') {
            let parts = line.split("#").collect::<Vec<&str>>();
            parts[0].trim()
        } else {
            line.trim()
        };

        // Skip empty lines
        if line.starts_with("[") && line.ends_with("]") {
            // New section starts
            let key = line.trim_matches('[').trim_matches(']').trim();
            if submap.len() != 0 && title.len() != 0 {
                map.insert(title.clone(), submap.clone());
            }
            title = key.to_string();
            submap = HashMap::new();
            continue;
        }

        // Split key and value
        let parts = line.split("=").collect::<Vec<&str>>();
        if parts.len() != 2 {
            continue;
        }

        // Remove leading and trailing whitespace
        let key = parts[0]
            .trim()
            .trim_end_matches('"')
            .trim_start_matches('"');
        if key.is_empty() {
            continue;
        }

        // Remove leading and trailing whitespace
        let value = parts[1].trim();
        let value = if value.contains('\'') || value.contains('"') {
            let value = value.trim_matches('"').trim();
            TOMLtype::Text(value.to_string())
        } else if value.to_lowercase() == "true" || value.to_lowercase() == "false" {
            let value = value.to_lowercase() == "true";
            TOMLtype::Boolean(value)
        } else if value.contains('.') {
            let value = value.parse::<f64>();
            if value.is_err() {
                panic!("Error parsing toml");
            }
            TOMLtype::Float(value.unwrap())
        } else {
            let value = value.parse::<u16>();
            if value.is_err() {
                panic!("Error parsing toml");
            }
            TOMLtype::Number(value.unwrap())
        };

        // Suggested alternative parsing logic
        // let value = if value.contains('\'') || value.contains('"') {
        //     TOMLtype::Text(value.trim_matches('"').to_string())
        // } else if value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false") {
        //     TOMLtype::Boolean(value.eq_ignore_ascii_case("true"))
        // } else if value.contains('.') {
        //     TOMLtype::Float(value.parse().expect("Error parsing float"))
        // } else {
        //     TOMLtype::Number(value.parse().expect("Error parsing number"))
        // };

        // Insert the key-value pair into the submap
        submap.insert(key.to_string(), value);
    }

    // Insert the last section if it exists
    map.insert(title, submap.clone());
    map
}

/// Configuration for the HTeaPot server.
///
/// This struct holds the runtime settings for the server,
/// such as host, port, caching behavior, and proxy rules.
#[derive(Debug)]
pub struct Config {
    pub port: u16,    // Port number to listen
    pub host: String, // Host name or IP
    pub root: String, // Root directory to serve files
    pub cache: bool,
    pub cache_ttl: u16,
    pub threads: u16,
    pub log_file: Option<String>,
    pub index: String, // Index file to serve by default
    // pub error: String, // Error file to serve when a file is not found
    pub proxy_rules: HashMap<String, String>,
    pub cgi_rules: HashMap<String, String>,
}

impl Config {
    // pub fn new(port: u16, host: String, root: String, index: String, error: String) -> Config {
    //     Config {
    //       port: port,
    //       host: host,
    //       root: root,
    //       index: index,
    //       error: error,
    //       proxy_rules: HashMap::new()
    //     }
    //   }

    /// Returns a default configuration with sensible values.
    pub fn new_default() -> Config {
        Config {
            port: 8080,
            host: "localhost".to_string(),
            root: ".".to_string(),
            index: "index.html".to_string(),
            log_file: None,
            //error: "error.html".to_string(),
            threads: 1,
            cache: false,
            cache_ttl: 0,
            proxy_rules: HashMap::new(),
            cgi_rules: HashMap::new(),
        }
    }

    pub fn new_serve(path: &str) -> Config {
        let mut s_path = "./".to_string();
        s_path.push_str(path);
        let serving_path = Path::new(&s_path);
        let file_name: &str;
        let root_dir: String;
        if serving_path.is_file() {
            let parent_path = serving_path.parent().unwrap();
            root_dir = parent_path.to_str().unwrap().to_string();
            file_name = serving_path.file_name().unwrap().to_str().unwrap();
        } else {
            file_name = "index.html";
            root_dir = serving_path.to_str().unwrap().to_string();
        };

        Config {
            port: 8080,
            host: "0.0.0.0".to_string(),
            root: root_dir,
            index: file_name.to_string(),
            log_file: None,

            threads: 1,
            cache: false,
            cache_ttl: 0,
            proxy_rules: HashMap::new(),
            cgi_rules: HashMap::new(),
        }
    }

    /// Loads configuration from a TOML file, returning defaults on failure.
    ///
    /// Expects the file to contain `[HTEAPOT]` and optionally `[proxy]` sections.
    /// Supports type-safe extraction for each expected key.
    pub fn load_config(path: &str) -> Config {
        let content = fs::read_to_string(path);
        if content.is_err() {
            return Config::new_default();
        }

        // Read the file content
        let content = content.unwrap();
        let map = toml_parser(&content);

        // Extract proxy rules
        let mut proxy_rules: HashMap<String, String> = HashMap::new();
        let proxy_map = map.get("proxy");
        if proxy_map.is_some() {
            let proxy_map = proxy_map.unwrap();
            for k in proxy_map.keys() {
                let url = proxy_map.get2(k);
                if url.is_none() {
                    println!("Missing or invalid proxy URL for key: {}", k);
                    continue;
                }
                let url = url.unwrap();
                proxy_rules.insert(k.clone(), url);
            }
        }

        // Suggested alternative parsing logic
        // if let Some(proxy_map) = map.get("proxy") {
        // for k in proxy_map.keys() {
        // if let Some(url) = proxy_map.get2(k) {
        // proxy_rules.insert(k.clone(), url);
        // } else {
        // println!("Missing or invalid proxy URL for key: {}", k);
        // }
        // }
        // }
        let mut cgi_rules: HashMap<String, String> = HashMap::new();
        #[cfg(feature = "cgi")]
        {
            let cgi_map = map.get("cgi");
            if cgi_map.is_some() {
                let cgi_map = cgi_map.unwrap();
                for k in cgi_map.keys() {
                    let command = cgi_map.get2(k);
                    if command.is_none() {
                        continue;
                    }
                    let command = command.unwrap();
                    cgi_rules.insert(k.clone(), command);
                }
            }
        }

        let map = map.get("HTEAPOT").unwrap();

        // Suggested alternative parsing logic (Not working)
        // let map = map.get("HTEAPOT").unwrap_or(&TOMLSchema::new());

        Config {
            port: map.get2("port").unwrap_or(8080),
            host: map.get2("host").unwrap_or("".to_string()),
            root: map.get2("root").unwrap_or("./".to_string()),
            threads: map.get2("threads").unwrap_or(1),
            cache: map.get2("cache").unwrap_or(false),
            cache_ttl: map.get2("cache_ttl").unwrap_or(3600),
            index: map.get2("index").unwrap_or("index.html".to_string()),
            log_file: map.get2("log_file"),
            //error: map.get2("error").unwrap_or("error.html".to_string()),
            proxy_rules,
            cgi_rules,
        }
    }

    pub fn new_proxy() -> Config {
        let mut proxy_rules = HashMap::new();
        proxy_rules.insert("/".to_string(), "".to_string());
        Config {
            port: 8080,
            host: "0.0.0.0".to_string(),
            root: "./".to_string(),
            cache: false,
            cache_ttl: 0,
            threads: 2,
            log_file: None,
            index: "index.html".to_string(),
            proxy_rules,
            cgi_rules: HashMap::new(),
        }
    }
}
