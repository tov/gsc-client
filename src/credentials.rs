use fs2::FileExt;
use reqwest::header::HeaderValue;
use std::default::Default;
use std::fs;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

use super::errors::ErrorKind;

type Result<T> = super::errors::Result<T>;

#[derive(Clone, Debug, Default)]
pub struct Credentials {
    pub username:     String,
    pub cookie_key:   String,
    pub cookie_value: String,
}

impl Credentials {
    pub fn new(username:     impl Into<String>,
               cookie_key:   impl Into<String>,
               cookie_value: impl Into<String>) -> Self {

        Self {
            username:     username.into(),
            cookie_key:   cookie_key.into(),
            cookie_value: cookie_value.into(),
        }
    }

    pub fn read(path: &Path) -> Result<Self> {
        let file = fs::File::open(path)
            .map_err(|_| ErrorKind::LoginPlease)?;
        file.lock_shared()?;

        let mut buf_reader = BufReader::new(file);
        let mut buf = String::new();
        let _ = buf_reader.read_line(&mut buf);

        let (username, key, value) =
            parse_cookie_file(buf.trim_end()).ok_or(ErrorKind::LoginPlease)?;

        Ok(Self {
            username:     username.to_owned(),
            cookie_key:   key.to_owned(),
            cookie_value: value.to_owned(),
        })
    }

    pub fn write(&self, filename: &Path) -> Result<()> {
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(filename)?;
        file.lock_exclusive()?;

        let mut w = BufWriter::new(file);
        write!(w, "{}:{}={}", self.username, self.cookie_key, self.cookie_value)?;

        Ok(())
    }

    pub fn to_header(&self) -> Result<HeaderValue> {
        let s = format!("{}={}", self.cookie_key, self.cookie_value);
        Ok(HeaderValue::from_str(&s)?)
    }
}

fn parse_cookie_file(contents: &str) -> Option<(&str, &str, &str)> {
    let colon = contents.find(':')?;
    let equals = contents.find('=')?;
    if colon > equals {
        return None;
    }
    Some((
        &contents[..colon],
        &contents[colon + 1..equals],
        &contents[equals + 1..],
    ))
}

