use fs2::FileExt;
use vlog::*;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use super::errors::*;

use serde_derive::{Serialize, Deserialize};
use serde_yaml;

const API_ENDPOINT: &str = "http://localhost:9090";
const DOTFILE_VAR: &str  = "GSC_LOGIN";
const DOTFILE_NAME: &str = ".gsclogin";

/// This is the type of the client configuration. It is loaded from and saved to the `Dotfile`
/// (below).
#[derive(Debug)]
pub struct Config {
    dotfile:    Option<PathBuf>,
    username:   String,
    endpoint:   String,
}

impl Config {
    pub fn new() -> Self {
        let dotfile = match env::var_os(DOTFILE_VAR) {
            Some(file) => Some(PathBuf::from(file)),
            None       => match env::var_os("HOME") {
                Some(home) => {
                    let mut buf = PathBuf::from(home);
                    buf.push(DOTFILE_NAME);
                    Some(buf)
                },
                None       => None,
            }
        };

        Config {
            dotfile,
            username:   String::new(),
            endpoint:   API_ENDPOINT.to_owned(),
        }
    }

    pub fn get_endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn get_username(&self) -> &str {
        &self.username
    }

    pub fn set_username(&mut self, username: String) {
        self.username = username;
    }

    fn new_cookie_lock(&mut self, dotfile: Dotfile, key: String, value: String)
        -> Result<CookieLock> {

        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&self.get_dotfile()?)?;
        file.lock_exclusive()?;

        Ok(CookieLock {
            file,
            dirty: false,
            key,
            value,
            dotfile,
        })
    }

    pub fn new_cookie(&mut self) -> Result<CookieLock> {
        self.new_cookie_lock(self.read_dotfile()?, String::new(), String::new())
    }

    pub fn lock_cookie(&mut self) -> Result<CookieLock> {
        let dotfile = self.read_dotfile()?;
        let (key, value) = super::parse_cookie(&dotfile.cookie)
            .ok_or(ErrorKind::LoginPlease)?;
        self.new_cookie_lock(dotfile, key, value)
    }

    pub fn get_dotfile(&self) -> Result<&Path> {
        match &self.dotfile {
            Some(dotfile) => Ok(&dotfile),
            _             => Err(ErrorKind::NoDotfileGiven)?,
        }
    }

    fn default_dotfile(&self) -> Dotfile {
        Dotfile {
            username: self.username.clone(),
            cookie:   String::new(),
            endpoint: self.endpoint.clone(),
        }
    }

    pub fn read_dotfile(&self) -> Result<Dotfile> {
        let dotfile_name = self.get_dotfile()?;
        let contents     = match fs::read_to_string(dotfile_name) {
            Ok(contents) => contents,
            Err(error) => match error.kind() {
                std::io::ErrorKind::NotFound => return Ok(self.default_dotfile()),
                _                            => Err(error)?,
            }
        };
        
        let parsed = serde_yaml::from_str(&contents)
            .map_err(|e| {
                let message = format!("Could not parse dotfile: {}", dotfile_name.display());
                Error::with_chain(e, message)
            })?;

        Ok(parsed)
    }

    pub fn load_dotfile(&mut self) -> Result<()> {
        let Dotfile { username, endpoint, .. } = self.read_dotfile()?;
        if !username.is_empty() { self.username = username; }
        if !endpoint.is_empty() { self.endpoint = endpoint; }

        Ok(())
    }
}

#[derive(Debug)]
pub struct CookieLock {
    file:       std::fs::File,
    dirty:      bool,
    key:        String,
    value:      String,
    dotfile:    Dotfile,
}

impl Drop for CookieLock {
    fn drop(&mut self) {
        if self.dirty {
            if let Err(e) = self.flush_cookie() {
                ve1!("Could not save cookie: {}", e);
            }
        }
    }
}

impl CookieLock {
    pub fn get_cookie(&self) -> (&str, &str) {
        return (&self.key, &self.value);
    }

    pub fn get_cookie_header(&self) -> reqwest::header::Cookie {
        let (key, value) = self.get_cookie();
        let mut header = reqwest::header::Cookie::new();
        header.set(key.to_owned(), value.to_owned());
        header
    }

    pub fn set_cookie(&mut self, key: String, value: String) {
        self.key   = key;
        self.value = value;
        self.dirty = true;
    }

    fn flush_cookie(&mut self) -> Result<()> {
        if self.dirty {
            self.dotfile.cookie = if self.key.is_empty() {
                String::new()
            } else {
                format!("{}={}", self.key, self.value)
            };

            self.file.set_len(0)?;
            let r1 = serde_yaml::to_writer(&mut self.file, &self.dotfile);
            let r2 = self.file.unlock();
            r1?; r2?
        }
        
        Ok(())
    }
}

/// This is the format of the dotfile.
#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Dotfile {
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub username:   String,
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub cookie:     String,
    #[serde(default)]
    pub endpoint:   String,
}

