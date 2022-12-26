use std::path::PathBuf;

use crate::error::Error;

pub fn configDir() -> Result<PathBuf, Error>
{
    let home = std::env::var("HOME")
        .map_err(|_| rterr!("Failed to get home dir"))?;
    let mut path = PathBuf::from(home);
    path.push(".config");
    path.push("cain");
    Ok(path)
}
