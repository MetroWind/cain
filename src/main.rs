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
use log::warn;

use crate::error::Error;
use crate::config::Config;
use crate::records::ListItem;

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
        warn!("Failed to find config dir: {}. Using current dir as root...", e);
        return defaultConfWithCurrentDir();
    }
    let conf_dir = conf_dir.unwrap();
    if !conf_dir.exists()
    {
        warn!("Config file not found. Using current dir as root...");
        return defaultConfWithCurrentDir();
    }

    let conf_file = conf_dir.join("config.toml");
    if !conf_file.exists()
    {
        warn!("Config file not found. Using current dir as root...");
        return defaultConfWithCurrentDir();
    }
    Config::fromFile(&conf_file)
}

fn cli() -> Result<(), Error>
{
    simple_logger::init_with_level(log::Level::Info).map_err(
        |_| rterr!("Failed to create logger"))?;

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
                     .default_value("")
                     .help("The category of the record. Default: \
                            Place record at root"))
                .arg(clap::Arg::new("download-font")
                     .short('F')
                     .long("download-font")
                     .action(clap::ArgAction::SetTrue)
                     .help("Download web fonts when using \
                            the web page downloader.")))
        .subcommand(clap::Command::new("list")
                    .about("List all categories and records"))
        .get_matches();

    let mut config = getConfig()?;

    match opts.subcommand()
    {
        Some(("record", sub_opts)) =>
        {
            config.single_page_config.download_font =
                *sub_opts.get_one::<bool>("download-font").unwrap();
            let url = sub_opts.get_one::<String>("URL").unwrap();
            let title = sub_opts.get_one::<String>("TITLE").unwrap();
            let cat = sub_opts.get_one::<String>("category").unwrap();
            records::make(url, title, &cat, &config)?;
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
        log::error!("{}", e);
        std::process::exit(1);
    }
    std::process::exit(0);
}
