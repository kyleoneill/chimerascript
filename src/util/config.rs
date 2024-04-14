use std::fs;
use std::ops::Add;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub ip: String,
    pub port: u16
}

impl Config {
    pub fn from_path_str(path_str: &str) -> Result<Self, String> {
        match fs::read_to_string(path_str) {
            Ok(file_contents) => {
                match toml::from_str::<Self>(&file_contents) {
                    Ok(res) => Ok(res),
                    Err(e) => Err(format!("Failed to parse config file, {}", e.message()))
                }
            },
            Err(_) => {
                return Err(format!("Failed to read config file with path {}", path_str))
            }
        }
    }
    pub fn get_target_address(&self) -> String {
        let mut cloned = self.ip.clone();
        cloned.push(':');
        cloned.add(&self.port.to_string())
    }
}
