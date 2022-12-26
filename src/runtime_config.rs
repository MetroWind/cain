use std::io::BufWriter;
use std::path::PathBuf;

use serde_json;

use crate::error::Error;
use crate::env;

fn runtimeFile() -> Result<PathBuf, Error>
{
    env::configDir().map(|d| d.join("runtime.json"))
}

pub fn get(key: &str) -> Result<Option<String>, Error>
{
    let runtime_file = runtimeFile()?;
    if !runtime_file.exists()
    {
        return Ok(None);
    }
    let contents = std::fs::read_to_string(runtime_file)
        .map_err(|_| rterr!("Failed to open runtime file"))?;
    let value: serde_json::Value = serde_json::from_str(&contents)
        .map_err(|_| rterr!("Invalid runtime file"))?;
    let value = &value[key];
    if value.is_null()
    {
        Ok(None)
    }
    else
    {
        value.as_str().map(|v| Some(v.to_owned()))
            .ok_or_else(|| rterr!("Key {} is not a string", key))
    }
}

pub fn set(key: &str, value: &str) -> Result<(), Error>
{
    let runtime_file = runtimeFile()?;
    let mut data = serde_json::Value::default();
    if runtime_file.exists()
    {
        let contents = std::fs::read_to_string(runtime_file.as_path())
            .map_err(|_| rterr!("Failed to open runtime file"))?;
        data = serde_json::from_str(&contents)
            .map_err(|_| rterr!("Invalid runtime file"))?;
    }
    data[key] = serde_json::Value::from(value);
    let f = std::fs::File::create(runtime_file)
        .map_err(|e| rterr!("Failed to open runtime file for write: {}", e))?;
    serde_json::to_writer_pretty(BufWriter::new(f), &data)
        .map_err(|_| rterr!("Failed to serialize runtime config"))
}
