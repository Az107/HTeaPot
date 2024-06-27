// Written by Alberto Ruiz 2024-04-07 (Happy 3th monthsary)
// This is the config module, it will load the configuration
// file and provide the settings

use std::{collections::HashMap, fs};


pub fn toml_parser(content: &str) -> HashMap<String,HashMap<String,String>> {
    let mut map = HashMap::new();
    let mut submap = HashMap::new();
    let mut title = "".to_string();
    let lines = content.split("\n");
    for line in lines {
        if line.starts_with("#")  || line.is_empty() {
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
        let key = parts[0].trim().trim_end_matches('"').trim_start_matches('"');
        println!("{}",key);
        if key.is_empty(){
            continue;
        }
        let value = parts[1].trim();
        let value = value.trim_matches('"').trim();
        submap.insert(key.to_string(), value.to_string());
    }
    map.insert(title, submap.clone());
    map
}


pub struct Config {
    pub port: u16, // Port number to listen
    pub host: String, // Host name or IP
    pub root: String, // Root directory to serve files
    pub cache: bool,
    pub threads: u16,
    pub index: String, // Index file to serve by default
    pub error: String, // Error file to serve when a file is not found
    pub proxy_rules: HashMap<String, String>
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
            error: "error.html".to_string(),
            threads: 1,
            cache: false,
            proxy_rules: HashMap::new()
        }
    }

    pub fn load_config(path: &str) -> Config {
      let content = fs::read_to_string(path);
      if content.is_err() {
          return Config::new_default();
      }
      let content = content.unwrap();
      let map = toml_parser(&content);
      println!("{:?}", map);
      let proxy_rules = map.get("proxy").unwrap_or(&HashMap::new()).clone();
      let map = map.get("HTEAPOT").unwrap();
      Config {
          port: map.get("port").unwrap_or(&"8080".to_string()).parse::<u16>().unwrap(),
          host: map.get("host").unwrap_or(&"".to_string()).to_string(),
          root: map.get("root").unwrap_or(&"./".to_string()).to_string(),
          threads: map.get("threads").unwrap_or(&"1".to_string()).parse::<u16>().unwrap(),
          cache: match map.get("cache") {
              Some(r) => {
                if r == "true" {
                    true
                } else {
                    false
                }
              },
              None => {
                false
              }
          },
          index: map.get("index").unwrap_or(&"index.html".to_string()).to_string(),
          error: map.get("error").unwrap_or(&"error.html".to_string()).to_string(),
          proxy_rules
      }
    }
}
  