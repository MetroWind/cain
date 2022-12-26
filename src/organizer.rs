use std::ffi::OsString;
use std::io::{Read, Write, BufWriter, BufRead};
use std::path::{Path, PathBuf};

use time::OffsetDateTime;
use time::format_description::well_known::Iso8601;
use serde::{Serialize, Deserialize};
use md5::{Md5, Digest};
use quick_xml::events::{Event, BytesEnd, BytesStart, BytesText};
use quick_xml::{Reader, Writer};

use crate::error::Error;
use crate::analyser::TempItem;

pub static METADATA_FILE: &str = "metadata.xml";

#[derive(Serialize, Deserialize)]
struct ResourceMetadata
{
    filename: String,
    url: Option<String>,
}

fn writeXMLTagBegin<W: Write>(tag: &str, writer: &mut Writer<W>) ->
    Result<(), Error>
{
    writer.write_event(Event::Start(BytesStart::new(tag))).map_err(
        |e| rterr!("Failed to start tag: {}", e))
}

fn writeXMLTagEnd<W: Write>(tag: &str, writer: &mut Writer<W>) ->
    Result<(), Error>
{
    writer.write_event(Event::End(BytesEnd::new(tag))).map_err(
        |e| rterr!("Failed to end tag: {}", e))
}

fn writeXMLText<W: Write>(content: &str, writer: &mut Writer<W>) ->
    Result<(), Error>
{
    writer.write_event(Event::Text(BytesText::new(content))).map_err(
        |e| rterr!("Failed to write text: {}", e))
}

impl ResourceMetadata
{
    #[allow(dead_code)]
    fn fromXMLReader<R: BufRead>(reader: &mut Reader<R>) -> Result<Self, Error>
    {
        #[derive(PartialEq)]
        #[allow(dead_code)]
        enum State
        {
            Filename,
            Url,
            Unknown,
            Stop,
        }

        let mut state = State::Unknown;
        let mut result = Self { filename: String::new(), url: None };
        let mut buffer = Vec::new();

        while state != State::Stop
        {
            match reader.read_event_into(&mut buffer) {
                Ok(Event::Start(e)) =>
                {
                    if e.name().as_ref() == b"filename"
                    {
                        state = State::Filename;
                    }
                    else if e.name().as_ref() == b"url"
                    {
                        state = State::Url;
                    }
                    else
                    {
                        return Err(rterr!("Invalid element in resource"));
                    }
                },
                Ok(Event::End(e)) =>
                {
                    if e.name().as_ref() == b"resource"
                    {
                        state = State::Stop;
                    }
                    else
                    {
                        state = State::Unknown;
                    }
                },
                Ok(Event::Text(inner)) =>
                {
                    match state
                    {
                        State::Filename =>
                        {
                            result.filename = inner.unescape().map_err(
                                |_| rterr!("Invalid filename in XML"))?
                                .into_owned();
                        },
                        State::Url =>
                        {
                            let u = inner.unescape().map_err(
                                |_| rterr!("Invalid filename in XML"))?
                                .into_owned();
                            result.url = Some(u);
                        },
                        _ => {},
                    }
                },
                Ok(_) => {},
                Err(_) =>
                {
                    return Err(rterr!("Failed to parse XML"));
                },
            }
        }
        Ok(result)
    }

    fn writeXML<W: Write>(&self, writer: &mut quick_xml::Writer<W>) ->
        Result<(), Error>
    {
        writeXMLTagBegin("resource", writer)?;
        writeXMLTagBegin("filename", writer)?;
        writeXMLText(&self.filename, writer)?;
        writeXMLTagEnd("filename", writer)?;
        if let Some(u) = &self.url
        {
            writeXMLTagBegin("url", writer)?;
            writeXMLText(&u, writer)?;
            writeXMLTagEnd("url", writer)?;
        }
        writeXMLTagEnd("resource", writer)
    }
}

struct Metadata
{
    title: String,
    time: OffsetDateTime,
    url: String,
    resources: Vec<ResourceMetadata>,
}

impl Metadata
{
    fn new() -> Self
    {
        Self {
            title: String::new(),
            time: OffsetDateTime::UNIX_EPOCH,
            url: String::new(),
            resources: Vec::new(),
        }
    }

    #[allow(dead_code)]
    fn fromXMLReader<R: BufRead>(reader: &mut Reader<R>) -> Result<Self, Error>
    {
        #[derive(PartialEq)]
        #[allow(dead_code)]
        enum State
        {
            Title,
            Time,
            Url,
            Resources,
            Unknown,
            Stop,
        }

        let mut state = State::Unknown;
        let mut result = Self::new();
        let mut buffer = Vec::new();

        while state != State::Stop
        {
            match reader.read_event_into(&mut buffer)
            {
                Ok(Event::Start(e)) =>
                {
                    match e.name().as_ref()
                    {
                        b"metadata" => {},
                        b"title" => state = State::Title,
                        b"time" => state = State::Time,
                        b"url" => state = State::Url,
                        b"resources" => state = State::Resources,
                        b"resource" =>
                        {
                            result.resources.push(
                                ResourceMetadata::fromXMLReader(reader)?);
                        },
                        _ =>
                        {
                            return Err(rterr!("Invalid XML element"));
                        }
                    }
                },
                Ok(Event::End(e)) =>
                {
                    if e.name().as_ref() == b"metadata"
                    {
                        state = State::Stop;
                    }
                    else
                    {
                        state = State::Unknown;
                    }
                },
                Ok(Event::Text(inner)) =>
                {
                    match state
                    {
                        State::Title =>
                        {
                            result.title = inner.unescape().map_err(
                                |_| rterr!("Invalid filename in XML"))?
                                .into_owned();
                        },
                        State::Time =>
                        {
                            let time_str = inner.unescape().map_err(
                                |_| rterr!("Invalid time in XML"))?
                                .into_owned();
                            result.time = OffsetDateTime::parse(
                                &time_str, &Iso8601::DEFAULT).map_err(
                                |_| rterr!("Invalid time string in XML: {}",
                                           time_str))?;
                        },
                        State::Url =>
                        {
                            result.url = inner.unescape().map_err(
                                |_| rterr!("Invalid URL in XML"))?
                                .into_owned();
                        },
                        _ => {},
                    }
                },
                Ok(_) => {},
                Err(_) =>
                {
                    return Err(rterr!("Failed to parse XML"));
                },
            }
        }
        Ok(result)
    }

    #[allow(dead_code)]
    fn fromFile(filename: &Path) -> Result<Self, Error>
    {
        let mut reader = Reader::from_file(filename).map_err(
            |_| rterr!("Failed to open XML file at {:?}", filename))?;
        reader.trim_text(true);
        Self::fromXMLReader(&mut reader)
    }

    fn writeXML<W: Write>(&self, writer: &mut quick_xml::Writer<W>) ->
        Result<(), Error>
    {
        writeXMLTagBegin("metadata", writer)?;
        writeXMLTagBegin("title", writer)?;
        writeXMLText(&self.title, writer)?;
        writeXMLTagEnd("title", writer)?;

        writeXMLTagBegin("time", writer)?;
        writeXMLText(&self.time.format(&Iso8601::DEFAULT).map_err(
            |_| rterr!("Failed to format time"))?, writer)?;
        writeXMLTagEnd("time", writer)?;

        writeXMLTagBegin("url", writer)?;
        writeXMLText(&self.url, writer)?;
        writeXMLTagEnd("url", writer)?;

        writeXMLTagBegin("resources", writer)?;
        for resource in &self.resources
        {
            resource.writeXML(writer)?;
        }
        writeXMLTagEnd("resources", writer)?;
        writeXMLTagEnd("metadata", writer)
    }

    fn writeToFile(&self, filename: &Path) -> Result<(), Error>
    {
        let w = BufWriter::new(std::fs::File::create(filename).map_err(
            |_| rterr!("Failed to open XML file at {:?}", filename))?);
        let mut writer = Writer::new_with_indent(w, b' ', 2);
        self.writeXML(&mut writer)
    }
}

fn moveFile(from: &Path, to: &Path) -> Result<(), Error>
{
    if let Err(_) = std::fs::rename(from, to)
    {
        std::fs::copy(from, to).map_err(
            |e| rterr!("Failed to copy file {:?} --> {:?}: {}",
                       from, to, e))?;
        std::fs::remove_file(from).map_err(
            |e| rterr!("Failed to delete file {:?}: {}", from, e))
    }
    else
    {
        Ok(())
    }
}

/// Return the hash of some bytes as a hex literal string. This is for
/// the purpose of file naming.
fn hashData(data: &[u8]) -> String
{
    let mut hasher = Md5::new();
    hasher.update(data);
    let hash = hasher.finalize();
    hash.iter().map(|byte| format!("{:02x}", byte))
        .collect::<Vec<String>>().join("")
}

/// Return the hash of the file content as a hex literal string. This
/// is for the purpose of naming the file.
fn hashFile(filename: &Path) -> Result<String, Error>
{
    let data = std::fs::read(filename).map_err(
        |e| rterr!("Failed to read file at {:?}: {}", filename, e))?;
    Ok(hashData(&data))
}

/// Download the resource at `url` into a file in directory `dir`. The
/// filename is the hash of the content with a detected extension
/// name.
fn download(url: &str, dir: &Path) -> Result<PathBuf, Error>
{
    // Download file
    let res = ureq::get(url).call().map_err(
        |e| rterr!("Failed to download from {}: {}", url, e))?;

    let mut data: Vec<u8> = if let Some(len) =
        res.header("Content-Length")
    {
        let len: usize = len.parse().map_err(
            |_| rterr!("Invalid content length “{}” from {}.", len, url))?;
        Vec::with_capacity(len)
    }
    else
    {
        Vec::new()
    };

    let final_url = res.get_url().to_owned();
    res.into_reader().take(1_000_000_000).read_to_end(&mut data).map_err(
        |e| rterr!("Failed to download {}: {}", url, e))?;

    // Try to detect content type
    let res = ureq::head(&final_url).call().map_err(
        |e| rterr!("Failed to get header from {}: {}", url, e))?;
    let ext_name = match res.content_type()
    {
        "video/mp4" => "mp4",
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/heic" => "heic",
        "image/webp" => "webp",
        _ => "bin",
    };

    // Write downloaded file
    let filename = format!("{}.{}", hashData(&data), ext_name);
    let target = dir.join(filename);
    let mut f = std::fs::File::create(&target).map_err(
        |_| rterr!("Failed to create download file for {}", url))?;
    f.write_all(&data).map_err(
        |e| rterr!("Failed to write download file: {}", e))?;
    Ok(target)
}

/// Record the resource into `dir`. This creates a file in that dir.
/// Returns the name of the file (only the filename itself, not
/// including the directory).
fn recordResource(resource: &TempItem, dir: &Path) -> Result<PathBuf, Error>
{
    match resource
    {
        TempItem::File(path) =>
        {
            let default_ext = OsString::default();
            let ext = path.extension().or(Some(&default_ext)).unwrap()
                .to_str().ok_or_else(
                    || rterr!("Invalid file name for file resource at {:?}",
                              path))?;

            let hash = hashFile(&path)?;
            let target = dir.join(hash + "." + ext);
            moveFile(&path, &target)?;
            Ok(target)
        },
        TempItem::Text(s) =>
        {
            let utf8 = s.as_bytes();
            let hash = hashData(utf8);
            let target = dir.join(hash + ".txt");
            let mut f = std::fs::File::create(&target).map_err(
                |e| rterr!("Failed to open file at {:?}: {}", target, e))?;
            f.write(utf8).map_err(
                |e| rterr!("Failed to write file at {:?}: {}", target, e))?;
            Ok(target)
        },
        TempItem::Url(u) =>
        {
            download(u, dir)
        },
    }
}

/// Create a new record from `resources` at a `path`. `Path` should
/// exit.
pub fn createRecord(resources: Vec<TempItem>, title: &str, url: &str,
                    path: &Path) -> Result<(), Error>
{
    let mut resources_data: Vec<ResourceMetadata> = Vec::new();
    for resource in resources
    {
        let filename: String = recordResource(&resource, path)?
            .file_name().unwrap().to_str().unwrap().to_owned();
        let url: Option<String> = match resource
        {
            TempItem::File(_) | TempItem::Text(_) => None,
            TempItem::Url(u) => Some(u),
        };
        resources_data.push(ResourceMetadata { filename, url });
    }

    let metadata = Metadata {
        title: title.to_owned(),
        time: OffsetDateTime::now_utc(),
        url: url.to_owned(),
        resources: resources_data,
    };
    metadata.writeToFile(&path.join(METADATA_FILE))
}

#[cfg(test)]
mod tests
{
    use super::*;
    use anyhow::Result;

    #[test]
    fn download() -> Result<()>
    {
        let temp_dir = tempfile::tempdir()?;
        let dir = temp_dir.path();
        createRecord(vec![TempItem::Url(
            String::from("https://picsum.photos/id/123/16"))],
                     "test", "https://google.com", dir)?;

        assert!(dir.join("dcc866d76ca96cee9559d124d2c22f8b.jpg").exists());

        let data = Metadata::fromFile(&dir.join("metadata.xml"))?;
        assert_eq!(data.title, "test");
        assert_eq!(data.url, "https://google.com");
        assert_eq!(data.resources.len(), 1);

        Ok(())
    }
}
