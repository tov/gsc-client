use fs2::FileExt;
use reqwest::header::HeaderValue;
use std::fs::File;
use std::io::{BufRead, Seek, Write};
use std::path::Path;
use vlog::*;

use super::errors::*;

#[derive(Debug)]
pub struct CookieFile {
    file: File,
    dirty: bool,
    key: String,
    value: String,
    username: String,
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
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(cookie_file)?;
        file.lock_exclusive()?;

        Ok(CookieFile {
            file,
            dirty: false,
            username: username.to_owned(),
            key: String::new(),
            value: String::new(),
        })
    }

    pub fn lock(cookie_file: &Path) -> Result<Self> {
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(cookie_file)
            .map_err(|_| ErrorKind::LoginPlease)?;
        file.lock_exclusive()?;

        let mut buf_reader = std::io::BufReader::new(file);
        let mut buf = String::new();
        let _ = buf_reader.read_line(&mut buf);

        file = buf_reader.into_inner();

        let (username, key, value) =
            parse_cookie_file(buf.trim_end()).ok_or(ErrorKind::LoginPlease)?;

        Ok(CookieFile {
            file,
            dirty: false,
            username: username.to_owned(),
            key: key.to_owned(),
            value: value.to_owned(),
        })
    }

    pub fn get_username(&self) -> &str {
        &self.username
    }

    pub fn get_cookie_header(&self) -> Result<HeaderValue> {
        let cookie = format!("{}={}", self.key, self.value);
        Ok(HeaderValue::from_str(&cookie)?)
    }

    pub fn set_cookie(&mut self, key: String, value: String) {
        self.key = key;
        self.value = value;
        self.dirty = true;
    }

    pub fn deauth(&mut self) {
        self.key.clear();
        self.value.clear();
        self.username.clear();
        self.dirty = true;
    }

    fn flush(&mut self) -> Result<()> {
        if self.dirty {
            self.file.set_len(0)?;
            self.file.seek(std::io::SeekFrom::Start(0))?;

            if !self.key.is_empty() {
                let contents = format!("{}:{}={}\n", self.username, self.key, self.value);
                self.file.write_all(contents.as_bytes())?;
            }
        }

        self.dirty = false;

        Ok(())
    }
}
