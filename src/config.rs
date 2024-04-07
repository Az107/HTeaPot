// Written by Alberto Ruiz 2024-04-07 (Happy 3th monthsary)
// This is the config module, it will load the configuration
// file and provide the settings

use std::{collections::HashMap, fs};


pub fn toml_parser(content: &str) -> HashMap<String,String> {
    let mut map = HashMap::new();
    let lines = content.split("\n");
    for line in lines {
        if line.starts_with("#") || line.starts_with("[")  || line.is_empty() {
            continue;
        }
        let parts = line.split("=").collect::<Vec<&str>>();
        let key = parts[0].trim();
        if key.is_empty() || key.contains('#'){
            continue;
        }
        let value = parts[1].trim().split("#").collect::<Vec<&str>>()[0].trim();
        let value = value.trim_matches('"');
        map.insert(key.to_string(), value.to_string());
    }
    map
}


pub struct config {
    pub port: u16, // Port number to listen
    pub host: String, // Host name or IP
    pub root: String, // Root directory to serve files
    pub index: String, // Index file to serve by default
    pub error: String, // Error file to serve when a file is not found
}

impl config {
    pub fn new(port: u16, host: String, root: String, index: String, error: String) -> config {
        config {
          port: port,
          host: host,
          root: root,
          index: index,
          error: error
        }
      }

    pub fn new_default() -> config {
        config {
            port: 8080,
            host: "".to_string(),
            root: "./".to_string(),
            index: "index.html".to_string(),
            error: "error.html".to_string(),
        }
    }

    pub fn load_config(path: &str) -> config {
      let content = fs::read_to_string(path);
      if content.is_err() {
          return config::new_default();
      }
      let content = content.unwrap();
      let map = toml_parser(&content);
      
      config {
          port: map.get("port").unwrap_or(&"8080".to_string()).parse::<u16>().unwrap(),
          host: map.get("host").unwrap_or(&"".to_string()).to_string(),
          root: map.get("root").unwrap_or(&"./".to_string()).to_string(),
          index: map.get("index").unwrap_or(&"index.html".to_string()).to_string(),
          error: map.get("error").unwrap_or(&"error.html".to_string()).to_string(),
      }
    }
}
  