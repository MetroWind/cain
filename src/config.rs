use serde::Deserialize;

use crate::error;
use crate::error::Error;

#[derive(Deserialize)]
pub struct Configuration
{
    pub data_dir: String,
    pub db_path: String,
    pub listen_address: String,
    pub listen_port: u16,
    pub log_timestamp: bool,
}

impl Default for Configuration
{
    fn default() -> Self
    {
        Self {
            data_dir: "/var/lib/archiver".to_owned(),
            db_path: "/var/lib/archiver/archiver.db".to_owned(),
            listen_address: "127.0.0.1".to_owned(),
            listen_port: 8080,
            log_timestamp: false,
        }
    }
}

impl Configuration
{
    pub fn readFromFile(f: &std::path::Path) -> Result<Self, Error>
    {
        let contents = std::fs::read_to_string(f).map_err(
            |_| rterr!("Failed to read configuration file"))?;
        let result: Configuration = toml::from_str(&contents).map_err(
            |_| rterr!("Invalid configuration file"))?;
        Ok(result)
    }
}
