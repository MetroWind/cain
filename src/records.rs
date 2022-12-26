use std::path::{Path, PathBuf};

use crate::analyser::ResourceAnalyser;
use crate::error::Error;
use crate::organizer;
use crate::twitter;
use crate::webpage;
use crate::organizer::createRecord;
use crate::config::Config;

pub enum ListItem
{
    Category(PathBuf),          // Contains a category path.
    Record(PathBuf),            // Contains a category path.
}

impl ListItem
{
    fn fromPath(category: &Path, config: &Config) -> Result<ListItem, Error>
    {
        let cat_path = config.root_dir.join(category);
        let mut entries = cat_path.read_dir().map_err(
            |_| rterr!("Failed to access directory at {:?}", cat_path))?
            .filter_map(|entry| entry.ok());

        // Is there a metadata file in the dir?
        if entries.find(|entry| entry.file_name().to_string_lossy() ==
                        organizer::METADATA_FILE).is_some()
        {
            Ok(ListItem::Record(PathBuf::from(category)))
        }
        else
        {
            Ok(ListItem::Category(PathBuf::from(category)))
        }
    }
}

/// List the sub-categories and records under `category`.
pub fn list(category: &Path, config: &Config) -> Result<Vec<ListItem>, Error>
{
    let current = ListItem::fromPath(category, config)?;
    if let ListItem::Record(_) = current
    {
        return Ok(vec![current]);
    }

    let cat_rel_path: &Path = Path::new(category);
    let cat_path = config.root_dir.join(cat_rel_path);

    // Unfortunately we read this dir for the second time here.
    // Hopefully file system cache will help us.
    let result: Vec<ListItem> = cat_path.read_dir().map_err(
        |_| rterr!("Failed to access directory at {:?}", category))?
        .filter_map(|entry| {
            if let Ok(e) = entry
            {
                if let Some(base_name) = e.file_name().to_str()
                {
                    return ListItem::fromPath(&cat_rel_path.join(base_name),
                                              config).ok();
                }
            }
            None
        }).collect();
    Ok(result)
}

/// List all the leaf categories and records under `category`,
/// recursivly.
pub fn listAll(category: &Path, config: &Config) -> Result<Vec<ListItem>, Error>
{
    let mut items = list(category, config)?;
    if items.is_empty()
    {
        return Ok(vec![ListItem::Category(PathBuf::from(category))]);
    }
    let mut result: Vec<ListItem> = Vec::new();
    for item in items.drain(..)
    {
        match item
        {
            ListItem::Category(cat) => result.append(
                &mut listAll(&cat, config)?),
            ListItem::Record(_) => result.push(item),
        }
    }
    Ok(result)
}

pub fn make(uri: &str, title: &str, category: &str, config: &Config) ->
    Result<(), Error>
{
    let u = url::Url::parse(uri).map_err(|_| rterr!("Invalid URL: {}", uri))?;
    let host = u.host_str().ok_or_else(|| rterr!("URL should have a host"))?;
    let items = if host == "twitter.com" || host == "www.twitter.com"
    {
        let client = twitter::Client::new()?;
        client.analyse(uri)?
    }
    else
    {
        let downloader = webpage::Downloader::new();
        downloader.analyse(uri)?
    };

    let full_path = config.root_dir.join(category).join(title);
    std::fs::create_dir_all(&full_path).map_err(
        |_| rterr!("Failed to create directory at {:?}", full_path))?;

    createRecord(items, title, uri, &full_path)
}
