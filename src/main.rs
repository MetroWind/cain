#![allow(non_snake_case)]

#[macro_use]
mod error;
mod analyser;
mod records;
mod config;
mod env;
mod organizer;
mod runtime_config;
mod twitter;
mod webpage;

use std::path::Path;

use crate::error::Error;
use crate::config::Config;
use records::ListItem;

fn formatPath(path: &Path) -> Result<String, Error>
{
    path.to_str().map(|s| s.to_owned()).ok_or_else(
        || rterr!("Failed to encode path {:?}", path))
}

fn defaultConfWithCurrentDir() -> Result<Config, Error>
{
    let mut conf = Config::default();
    conf.root_dir = std::env::current_dir().map_err(
        |_| rterr!("Failed to get current directory"))?;
    return Ok(conf);
}

fn getConfig() -> Result<Config, Error>
{
    let conf_dir = env::configDir();
    if let Err(e) = conf_dir
    {
        eprintln!("WARNING: failed to find config dir: {}. \
                   Using current dir as root...", e);
        return defaultConfWithCurrentDir();
    }
    let conf_dir = conf_dir.unwrap();
    if !conf_dir.exists()
    {
        eprintln!("WARNING: config file not found. \
                   Using current dir as root...");
        return defaultConfWithCurrentDir();
    }

    let conf_file = conf_dir.join("config.toml");
    if !conf_file.exists()
    {
        eprintln!("WARNING: config file not found. \
                   Using current dir as root...");
        return defaultConfWithCurrentDir();
    }
    Config::fromFile(&conf_file)
}

fn cli() -> Result<(), Error>
{
    let opts = clap::Command::new("Cain")
        .author("MetroWind")
        .about("A naively simple personal web resource archive system")
        .subcommand(
            clap::Command::new("record")
                .about("Archive an URL")
                .arg(clap::Arg::new("TITLE")
                     .required(true)
                     .help("The title of the record"))
                .arg(clap::Arg::new("URL")
                    .required(true)
                     .help("The URL to record"))
                .arg(clap::Arg::new("category")
                     .short('c')
                     .long("category")
                     .help("The category of the record. Default: \
                            Place record at root")))
        .subcommand(clap::Command::new("list")
                    .about("List all categories and records"))
        .get_matches();

    let config = getConfig()?;

    match opts.subcommand()
    {
        Some(("record", sub_opts)) =>
        {
            let url = sub_opts.get_one::<String>("URL").unwrap();
            let title = sub_opts.get_one::<String>("TITLE").unwrap();
            records::make(url, title, "", &config)?;
        },
        Some(("list", _)) =>
        {
            for item in records::listAll(Path::new(""), &config)?
            {
                match item
                {
                    ListItem::Category(path) =>
                        println!("C {}", formatPath(&path)?),
                    ListItem::Record(path) =>
                        println!("R {}", formatPath(&path)?),
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn main()
{
    if let Err(e) = cli()
    {
        eprintln!("{}", e);
        std::process::exit(1);
    }
    std::process::exit(0);
}
