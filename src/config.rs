// Written by Alberto Ruiz 2024-04-07 (Happy 3th monthsary)
// This is the config module, it will load the configuration
// file and provide the settings
// This provide partial support for TOML
// Features:
// - [X] Primitives (String, Integer, Float, Boolean)
// - [ ] Datetime
// - [X] Comments
// - [X] Arrays
// - [ ] Nested Arrays
// - [X] Tables
// - [ ] Nested Tables
// - [ ] Inline Tables
// - [ ] Multiline Strings
// - [ ] Array of Tables
// - [ ] Key-Value Pairs
// - [ ] Dotted Keys

use std::{any::Any, collections::HashMap, fs};

#[derive(Clone, Debug)]
pub enum TOMLtype {
    Text(String),
    Number(u16),
    Float(f64),
    Boolean(bool),
    List(Vec<TOMLtype>),
    Table(TOMLSchema),
}

impl TOMLtype {
    fn to<T: 'static + Clone>(&self) -> Option<T> {
        let any_value: Box<dyn Any> = match self {
            TOMLtype::Text(d) => Box::new(d.clone()),
            TOMLtype::Number(d) => Box::new(d.clone()),
            TOMLtype::Float(d) => Box::new(d.clone()),
            TOMLtype::Boolean(d) => Box::new(d.clone()),
            TOMLtype::List(d) => Box::new(d.clone()),
            TOMLtype::Table(d) => Box::new(d.clone()),
        };
        let r = any_value.downcast_ref::<T>().cloned();
        r
    }
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
            TOMLtype::List(d) => Box::new(d),
            TOMLtype::Table(d) => Box::new(d),
        };
        let r = any_value.downcast_ref::<T>().cloned();
        r
    }
}

pub fn toml_value_parser(value: &str) -> TOMLtype {
    if value.starts_with("[") && value.ends_with("]") {
        let value = value.strip_suffix(']').unwrap().strip_prefix('[').unwrap();
        let mut list = Vec::new();
        for item in value.split(",") {
            list.push(toml_value_parser(item.trim()));
        }
        TOMLtype::List(list)
    } else if value.to_lowercase() == "true" || value.to_lowercase() == "false" {
        let value = value.to_lowercase() == "true";
        TOMLtype::Boolean(value)
    } else if value.contains('.') {
        let value = value.parse::<f64>();
        if value.is_err() {
            panic!("Error parsing toml");
        }
        TOMLtype::Float(value.unwrap())
    } else if value.starts_with("[") && value.ends_with("]") {
        let value = value.strip_suffix(']').unwrap().strip_prefix('[').unwrap();
        let mut list = Vec::new();
        for item in value.split(",") {
            list.push(toml_value_parser(item.trim()));
        }
        TOMLtype::List(list)
    } else if value.contains('\'') || value.contains('"') {
        let value = value.trim_matches('"').trim();
        TOMLtype::Text(value.to_string())
    } else {
        let value = value.parse::<u16>();
        if value.is_err() {
            panic!("Error parsing toml");
        }
        TOMLtype::Number(value.unwrap())
    }
}

pub fn toml_parser(content: &str) -> TOMLSchema {
    let mut map = TOMLSchema::new();

    let mut pointer_title = String::new();
    let mut pointer = TOMLSchema::new();
    let lines = content.split("\n");
    for line in lines {
        // Check if is a comment
        if line.starts_with("#") || line.is_empty() {
            continue;
        }
        let line = if line.contains('#') {
            let parts = line.split("#").collect::<Vec<&str>>();
            parts[0].trim()
        } else {
            line.trim()
        };

        // Process line by line

        //Table
        if line.starts_with("[") && line.ends_with("]") {
            let key = line
                .strip_suffix(']')
                .unwrap()
                .strip_prefix('[')
                .unwrap()
                .trim();
            pointer_title = key.to_string();
            if pointer.len() != 0 && pointer_title.len() != 0 {
                if pointer_title.starts_with("[") && pointer_title.ends_with("]") {
                    let pointer_title = pointer_title
                        .strip_suffix(']')
                        .unwrap()
                        .strip_prefix('[')
                        .unwrap();
                    let mut table_list = map.get2(pointer_title).unwrap_or(Vec::new());
                    table_list.push(TOMLtype::Table(pointer.clone()));
                    map.insert(pointer_title.to_string(), TOMLtype::List(table_list));
                } else {
                    map.insert(pointer_title.clone(), TOMLtype::Table(pointer.clone()));
                }
            }
            pointer = TOMLSchema::new();
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
        let value = toml_value_parser(value);
        if pointer_title.is_empty() {
            map.insert(key.to_string(), value);
        } else {
            pointer.insert(key.to_string(), value);
        }
    }
    if pointer_title.starts_with("[") && pointer_title.ends_with("]") {
        let pointer_title = pointer_title
            .strip_suffix(']')
            .unwrap()
            .strip_prefix('[')
            .unwrap();
        let mut table_list = map.get2(pointer_title).unwrap_or(Vec::new());
        table_list.push(TOMLtype::Table(pointer.clone()));
        map.insert(pointer_title.to_string(), TOMLtype::List(table_list));
    } else {
        map.insert(pointer_title.clone(), TOMLtype::Table(pointer.clone()));
    }
    map
}

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
            log_file: None,
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
        let proxy_map: TOMLSchema = map.get2("proxy").unwrap_or(TOMLSchema::new());
        for k in proxy_map.keys() {
            let url = proxy_map.get2(k);
            if url.is_none() {
                println!();
                continue;
            }
            let url = url.unwrap();
            proxy_rules.insert(k.clone(), url);
        }

        let map = map.get2("HTEAPOT").unwrap_or(TOMLSchema::new());
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
        }
    }
}

#[cfg(test)]
#[test]
fn test_basic_parser() {
    let toml_content = r###"
        owner = "Alb"
        [user]
        name = "Juan"
        age = 30
        married = true
        childs = ["Pedro", "Maria"]
        "###;

    let data = toml_parser(toml_content);
    println!("{:?}", data);
    let owner = data.get2::<String>("owner");
    assert!(owner.is_some());
    assert!(owner.unwrap() == "Alb");
    let user = data.get2::<TOMLSchema>("user");
    assert!(user.is_some());
    let user = user.unwrap();
    let name = user.get2::<String>("name");
    let age = user.get2::<u16>("age");
    let married = user.get2::<bool>("married");
    let childs = user.get2::<Vec<TOMLtype>>("childs");
    assert!(name.is_some());
    assert!(name.unwrap() == "Juan");
    assert!(age.is_some());
    assert!(age.unwrap() == 30);
    assert!(married.is_some());
    assert!(married.unwrap() == true);
    assert!(childs.is_some());
    let childs = childs.unwrap();
    assert!(childs.len() == 2);
    let first_child_name: Option<String> = childs[0].clone().to();
    assert!(first_child_name.is_some());
    assert!(first_child_name.unwrap() == "Pedro");
}

#[cfg(test)]
#[test]
fn test_nested_tables_parser() {
    let toml_content = r###"

        [[user]]
        name = "Juan"
        age = 30
        married = true
        childs = ["Pedro", "Maria"]

        [[user]]
        name = "Pedro"
        age = 10
        married = false
        "###;

    let data = toml_parser(toml_content);
    let users = data.get2::<Vec<TOMLtype>>("user");
    assert!(users.is_some());
    let users = users.unwrap();
    assert!(users.len() == 2);
    let juan: Option<TOMLSchema> = users[0].clone().to();
    let pedro: Option<TOMLSchema> = users[1].clone().to();
    assert!(juan.is_some());
    assert!(pedro.is_some());
}
