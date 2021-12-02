#[cfg(feature = "file_locking")]
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
    username_:     String,
    cookie_key_:   String,
    cookie_value_: String,
}

impl Credentials {
    pub fn new(
        username: impl Into<String>,
        cookie_key: impl Into<String>,
        cookie_value: impl Into<String>,
    ) -> Self {
        Self {
            username_:     username.into(),
            cookie_key_:   cookie_key.into(),
            cookie_value_: cookie_value.into(),
        }
    }

    pub fn read(path: &Path) -> Result<Self> {
        let file = fs::File::open(path).map_err(|_| ErrorKind::LoginPlease)?;

        #[cfg(feature = "file_locking")]
        file.lock_shared()?;

        let mut buf_reader = BufReader::new(file);
        let mut buf = String::new();
        let _ = buf_reader.read_line(&mut buf);

        let (username, key, value) =
            parse_cookie_file(buf.trim_end()).ok_or(ErrorKind::LoginPlease)?;

        Ok(Self {
            username_:     username.to_owned(),
            cookie_key_:   key.to_owned(),
            cookie_value_: value.to_owned(),
        })
    }

    pub fn write(&self, filename: &Path) -> Result<()> {
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(filename)?;

        #[cfg(feature = "file_locking")]
        file.lock_exclusive()?;

        let mut w = BufWriter::new(file);
        writeln!(
            w,
            "{}:{}={}",
            self.username_, self.cookie_key_, self.cookie_value_
        )?;

        Ok(())
    }

    pub fn username(&self) -> &str {
        &self.username_
    }

    pub fn to_header(&self) -> Result<HeaderValue> {
        let s = format!("{}={}", self.cookie_key_, self.cookie_value_);
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
