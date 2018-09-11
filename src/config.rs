use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use super::errors::*;

use serde_derive::{Serialize, Deserialize};
use serde_yaml;

const API_ENDPOINT: &str = "http://localhost:9090/api";
const DOTFILE_VAR: &str  = "GSC_LOGIN";
const DOTFILE_NAME: &str = ".gsclogin";

/// This is the type of the client configuration. It is loaded from and saved to the `Dotfile`
/// (below).
#[derive(Debug)]
pub struct Config {
    pub dotfile:    Option<PathBuf>,
    pub username:   Option<String>,
    pub cookie:     Option<(String, String)>,
    pub endpoint:   String,
    pub save:       bool,
}

impl Drop for Config {
    fn drop(&mut self) {
        if let Err(err) = self.save_dotfile() {
            eprintln!("Error saving dotfile: {}", err);
        }
    }
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

        let username = env::var("USER").ok();

        Config {
            dotfile,
            username,
            cookie:     None,
            endpoint:   API_ENDPOINT.to_owned(),
            save:       false,
        }
    }

    pub fn get_username(&self) -> Result<&str> {
        match &self.username {
            Some(username) => Ok(&username),
            _              => Err(ErrorKind::NoUsernameGiven)?,
        }
    }

    pub fn get_cookie(&self) -> Result<(&str, &str)> {
        match &self.cookie {
            Some((key, value)) => Ok((&key, &value)),
            None               => Err(ErrorKind::LoginPlease)?,
        }
    }

    pub fn get_cookie_header(&self) -> Result<reqwest::header::Cookie> {
        let (key, value) = self.get_cookie()?;
        let mut header = reqwest::header::Cookie::new();
        header.set(key.to_owned(), value.to_owned());
        Ok(header)
    }

    pub fn get_dotfile(&self) -> Result<&Path> {
        match &self.dotfile {
            Some(dotfile) => Ok(&dotfile),
            _             => Err(ErrorKind::NoDotfileGiven)?,
        }
    }

    pub fn load_dotfile(&mut self) -> Result<()> {
        let dotfile_name = self.get_dotfile()?;
        let contents     = match fs::read_to_string(dotfile_name) {
            Ok(contents) => contents,
            Err(error) => match error.kind() {
                std::io::ErrorKind::NotFound => return Ok(()),
                _ => Err(error)?,
            }
        };

        let parsed: Dotfile = serde_yaml::from_str(&contents)
            .map_err(|e| Error::with_chain(e, "Could not parse dotfile"))?;

        let Dotfile { username, cookie, endpoint } = parsed;
        if !username.is_empty() { self.username = Some(username); }
        if !cookie.is_empty() { self.cookie = super::parse_cookie(&cookie); }
        if !endpoint.is_empty() { self.endpoint = endpoint; }

        Ok(())
    }

    fn save_dotfile(&self) -> Result<()> {
        if !self.save { return Ok(()); }

        let dotfile_name = self.get_dotfile()?;
        let username = self.get_username().unwrap_or("").to_owned();
        let cookie = match &self.cookie {
            Some((key, value)) => format!("{}={}", key, value),
            None               => "".to_owned(),
        };
        let endpoint = self.endpoint.clone();
        let contents = serde_yaml::to_string(&Dotfile { username, cookie, endpoint })?;
        fs::write(dotfile_name, contents)?;
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

