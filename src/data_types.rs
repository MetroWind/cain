use std::str::FromStr;
use std::hash::Hash;
use std::collections::HashMap;
use std::fmt;

use chrono::prelude::*;
use serde::ser;
use serde::ser::SerializeStruct;
use serde::{Serialize, Deserialize};

use crate::error;
use crate::error::Error as Error;

#[derive(Clone, Hash, Serialize)]
pub enum EntryKey
{
    ArchiveSingleFile,
    DescAsciiDoc,
    Other(String),
}

impl FromStr for EntryKey
{
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err>
    {
        let r = match s
        {
            "ArchiveSingleFile" => Self::ArchiveSingleFile,
            "DescAsciiDoc" => Self::DescAsciiDoc,
            _ => Self::Other(s.to_owned()),
        };
        Ok(r)
    }
}

impl From<&str> for EntryKey
{
    fn from(s: &str) -> Self
    {
        match s
        {
            "ArchiveSingleFile" => Self::ArchiveSingleFile,
            "DescAsciiDoc" => Self::DescAsciiDoc,
            _ => Self::Other(s.to_owned()),
        }
    }
}

impl fmt::Display for EntryKey
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        let s: &str = match self
        {
            Self::ArchiveSingleFile => "ArchiveSingleFile",
            Self::DescAsciiDoc => "DescAsciiDoc",
            Self::Other(s) => &s,
        };
        write!(f, "{}", s)
    }
}

/// The type of the content string.
#[derive(Serialize, Deserialize)]
pub enum ReferenceType
{
    Direct,    /// It’s not a reference. The string itself is the data.
    Path,      /// It’s a path in the local filesystem
    URI,
}

#[derive(Serialize, Deserialize)]
pub struct EntryData
{
    ref_type: ReferenceType,
    content: String,
}

#[derive(Serialize, Deserialize)]
pub struct Category
{
    pub id: i64,
    pub name: String,
}

impl Category
{
    pub fn new(id: i64, name: &str) -> Self
    {
        Self { id: id, name: name.to_owned() }
    }
}

pub struct Entry
{
    pub title: String,
    pub uri: String,
    pub categories: Vec<Category>,
    pub time_add: chrono::DateTime<Utc>,
    pub data: HashMap<EntryKey, EntryData>,
}

impl Serialize for Entry
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: ser::Serializer,
    {
        let mut s = serializer.serialize_struct("Entry", 3)?;
        s.serialize_field("title", &self.title)?;
        s.serialize_field("uri", &self.uri)?;
        s.serialize_field("categories", &self.categories)?;
        s.serialize_field(
            "time_add", &self.time_add.format("%F %R").to_string())?;
        s.end()
    }
}
