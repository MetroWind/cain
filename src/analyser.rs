use std::path::PathBuf;

use crate::error::Error;

#[derive(PartialEq, Debug)]
pub enum TempItem
{
    File(PathBuf),
    Url(String),
    Text(String),
}

/// A ResourceAnalyser figures out the required resources from the
/// origianl URL. For each of these resources the ResourceAnalyser
/// provide either a temperary local file or a URL where the resource
/// can be directly downloaded. A ResourceAnalyser does not deal with
/// categories and metadata.
pub trait ResourceAnalyser
{
    fn analyse(&self, url: &str) -> Result<Vec<TempItem>, Error>;
}
