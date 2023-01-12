use std::path::PathBuf;
use std::process::Command;

use crate::analyser;
use crate::analyser::TempItem;
use crate::error::Error;

pub struct Downloader
{
    download_font: bool,
    disable_js: bool,
}

impl Downloader
{
    pub fn new(download_font: bool, disable_js: bool) -> Self
    {
        Self { download_font, disable_js }
    }
}

impl analyser::ResourceAnalyser for Downloader
{
    fn analyse(&self, url: &str) -> Result<Vec<TempItem>, Error>
    {
        // Create a temp file
        let mut temp_file = PathBuf::from(std::env::temp_dir());
        temp_file.push("cain-monolith.html");

        // Download the URL to the temp file
        let mut proc = Command::new("monolith");
        proc.args(&["--no-audio", "--isolate", "-o",
                    temp_file.to_str().ok_or_else(
                        || rterr!("Empty output file for Monolith"))?]);
        if !self.download_font
        {
            proc.arg("--no-fonts");
        }
        if self.disable_js
        {
            proc.arg("--no-js");
        }

        proc.arg(url);
        let status = proc.status().map_err(
            |e| rterr!("Failed to run Monolith: {}", e))?;
        if status.success()
        {
            Ok(vec![TempItem::File(temp_file)])
        }
        else
        {
            Err(rterr!("Monolith failed with code {}",
                       status.code().or(Some(0)).unwrap()))
        }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::analyser::ResourceAnalyser;

    #[test]
    fn analyse() -> Result<(), Error>
    {
        let downloader = Downloader::new(false, true);
        let items = downloader.analyse("http://example.org/")?;
        assert_eq!(items.len(), 1);
        match items[0]
        {
            TempItem::File(_) => assert!(true),
            _ => assert!(false),
        }
        Ok(())
    }
}
