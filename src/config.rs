use std::path::{PathBuf, Path};

use serde::Deserialize;

use crate::error::Error;

#[derive(Deserialize, Clone)]
pub enum TwitterAuth
{
    GuestToken,
    StaticToken {
        consumer_key: String,
        consumer_secret: String,
        access_token: String,
        access_token_secret: String,
    },
}

impl Default for TwitterAuth
{
    fn default() -> Self
    {
        TwitterAuth::GuestToken
    }
}

#[derive(Deserialize, Clone)]
pub struct Config
{
    pub root_dir: PathBuf,
    // #[serde(default)]
    pub twitter_auth: TwitterAuth,
}

impl Config
{
    pub fn fromFile(filename: &Path) -> Result<Self, Error>
    {
        let content = std::fs::read_to_string(filename)
            .map_err(|_| rterr!("Failed to read config file"))?;
        toml::from_str(&content)
            .map_err(|e| rterr!("Invalid config file: {}", e))
    }
}

impl Default for Config
{
    fn default() -> Self
    {
        Self {
            root_dir: PathBuf::from("/"),
            twitter_auth: TwitterAuth::GuestToken,
        }
    }
}
