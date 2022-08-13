#![allow(non_snake_case)]

use std::path::Path;

#[macro_use]
mod error;
mod config;
mod tree;
mod data_types;
mod data;
mod app;

use error::Error;

fn findConfig() -> Result<config::Configuration, Error>
{
    let p = Path::new("archiver.toml");
    if p.exists()
    {
        config::Configuration::readFromFile(p)
    }
    else
    {
        let p = Path::new("/etc/archiver.toml");
        if p.exists()
        {
            config::Configuration::readFromFile(p)
        }
        else
        {
            Ok(config::Configuration::default())
        }
    }
}

fn main() -> Result<(), Error>
{
    let config = findConfig()?;
    if !config.log_timestamp
    {
        env_logger::builder().format_timestamp(None).init();
    }
    else
    {
        env_logger::init();
    }
    let mut a = app::App::new(config);
    a.init()?;
    tokio::runtime::Runtime::new().unwrap().block_on(a.serve())?;
    Ok(())
}
