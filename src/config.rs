// Written by Alberto Ruiz 2024-04-07 (Happy 3th monthsary)
// This is the config module, it will load the configuration
// file and provide the settings

use std::{any::Any, collections::HashMap, fs};

#[derive(Clone, Debug)]
pub enum TOMLtype {
    Text(String),
    Number(u16),
    Float(f64),
    Boolean(bool),
}

type TOMLSchema = HashMap<String, TOMLtype>;
trait Schema {
    fn get2<T: 'static + Clone>(&self, key: &str) -> Option<T>;
}

impl Schema for TOMLSchema {
    fn get2<T: 'static + Clone>(&self, key: &str) -> Option<T> {
        let value = self.get(key)?;
        let value = value.clone();
        let any_value: Box<dyn Any> = match value {
            TOMLtype::Text(d) => Box::new(d),
            TOMLtype::Number(d) => Box::new(d),
            TOMLtype::Float(d) => Box::new(d),
            TOMLtype::Boolean(d) => Box::new(d),
        };
        let r = any_value.downcast_ref::<T>().cloned();
        if r.is_none() {
            println!("{} is none", key);
        }
        r
    }
}

pub fn toml_parser(content: &str) -> HashMap<String, TOMLSchema> {
    let mut map = HashMap::new();
    let mut submap = HashMap::new();
    let mut title = "".to_string();
    let lines = content.split("\n");
    for line in lines {
        if line.starts_with("#") || line.is_empty() {
            continue;
        }
        let line = if line.contains('#') {
            let parts = line.split("#").collect::<Vec<&str>>();
            parts[0].trim()
        } else {
            line.trim()
        };
        if line.starts_with("[") && line.ends_with("]") {
            let key = line.trim_matches('[').trim_matches(']').trim();
            if submap.len() != 0 && title.len() != 0 {
                map.insert(title.clone(), submap.clone());
            }
            title = key.to_string();
            submap = HashMap::new();
            continue;
        }
        let parts = line.split("=").collect::<Vec<&str>>();
        if parts.len() != 2 {
            continue;
        }
        let key = parts[0]
            .trim()
            .trim_end_matches('"')
            .trim_start_matches('"');
        if key.is_empty() {
            continue;
        }
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
        submap.insert(key.to_string(), value);
    }
    map.insert(title, submap.clone());
    map
}

#[derive(Debug)]
pub struct Config {
    pub port: u16,    // Port number to listen
    pub host: String, // Host name or IP
    pub ssl: bool,
    pub root: String, // Root directory to serve files
    pub cache: bool,
    pub cache_ttl: u16,
    pub threads: u16,
    pub index: String, // Index file to serve by default
    //pub error: String, // Error file to serve when a file is not found
    pub proxy_rules: HashMap<String, String>,
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

    pub fn new_default() -> Config {
        Config {
            port: 8080,
            host: "localhost".to_string(),
            root: "./".to_string(),
            index: "index.html".to_string(),
            ssl: false,
            //error: "error.html".to_string(),
            threads: 1,
            cache: false,
            cache_ttl: 0,
            proxy_rules: HashMap::new(),
        }
    }

    pub fn load_config(path: &str) -> Config {
        let content = fs::read_to_string(path);
        if content.is_err() {
            return Config::new_default();
        }
        let content = content.unwrap();
        let map = toml_parser(&content);
        let mut proxy_rules: HashMap<String, String> = HashMap::new();
        let proxy_map = map.get("proxy");
        if proxy_map.is_some() {
            let proxy_map = proxy_map.unwrap();
            for k in proxy_map.keys() {
                let url = proxy_map.get2(k);
                if url.is_none() {
                    println!();
                    continue;
                }
                let url = url.unwrap();
                proxy_rules.insert(k.clone(), url);
            }
        }

        let map = map.get("HTEAPOT").unwrap();
        Config {
            port: map.get2("port").unwrap_or(8080),
            host: map.get2("host").unwrap_or("".to_string()),
            root: map.get2("root").unwrap_or("./".to_string()),
            threads: map.get2("threads").unwrap_or(1),
            cache: map.get2("cache").unwrap_or(false),
            ssl: map.get2("ssl").unwrap_or(false),
            cache_ttl: map.get2("cache_ttl").unwrap_or(3600),
            index: map.get2("index").unwrap_or("index.html".to_string()),
            //error: map.get2("error").unwrap_or("error.html".to_string()),
            proxy_rules,
        }
    }
}
