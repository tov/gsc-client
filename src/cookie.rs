use fs2::FileExt;
use reqwest::header::HeaderValue;
use std::default::Default;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, BufWriter, Seek, Write};
use std::path::Path;
use vlog::*;

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

#[derive(Debug, Default, Clone)]
pub struct CookieContents {
    key: String,
    value: String,
}

impl fmt::Display for CookieContents {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}={}", self.key, self.value)
    }
}

impl CookieContents {
    pub fn new(key: impl Into<String>,
               value: impl Into<String>) -> Self {
        CookieContents {
            key: key.into(),
            value: value.into(),
        }
    }

    pub fn to_header(&self) -> Result<HeaderValue> {
        Ok(HeaderValue::from_str(&self.to_string())?)
    }

    pub fn clear(&mut self) {
        self.key.clear();
        self.value.clear();
    }
}

#[derive(Debug)]
pub struct CookieFile {
    file: File,
    dirty: bool,
    username: String,
    contents: CookieContents,
}

impl fmt::Display for CookieFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.username, self.contents)
    }
}

impl Drop for CookieFile {
    fn drop(&mut self) {
        if self.dirty {
            if let Err(e) = self.flush() {
                ve1!("Could not save cookie: {}", e);
            }

            if let Err(e) = self.file.unlock() {
                ve1!("Could not unlock cookie file: {}", e);
            }
        }
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

impl CookieFile {
    pub fn new(cookie_file: &Path, username: &str) -> Result<Self> {
        let file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(cookie_file)?;
        file.lock_exclusive()?;

        Ok(CookieFile {
            file,
            dirty: false,
            username: username.to_owned(),
            contents: CookieContents::default(),
        })
    }

    pub fn lock(cookie_file: &Path) -> Result<Self> {
        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(cookie_file)
            .map_err(|_| ErrorKind::LoginPlease)?;
        file.lock_exclusive()?;

        let mut buf_reader = BufReader::new(file);
        let mut buf = String::new();
        let _ = buf_reader.read_line(&mut buf);

        file = buf_reader.into_inner();

        let (username, key, value) =
            parse_cookie_file(buf.trim_end()).ok_or(ErrorKind::LoginPlease)?;

        Ok(CookieFile {
            file,
            dirty: false,
            username: username.to_owned(),
            contents: CookieContents::new(key, value),
        })
    }

    pub fn get_username(&self) -> &str {
        &self.username
    }

    pub fn to_header(&self) -> Result<HeaderValue> {
        self.contents.to_header()
    }

    pub fn deauth(&mut self) {
        self.contents.clear();
        self.username.clear();
        self.dirty = true;
    }

    fn flush(&mut self) -> Result<()> {
        if self.dirty {
            self.file.set_len(0)?;
            self.file.seek(io::SeekFrom::Start(0))?;

            if !self.contents.key.is_empty() {
                let content = self.to_string();
                writeln!(self.file, "{}", content)?;
            }
        }

        self.dirty = false;

        Ok(())
    }
}
