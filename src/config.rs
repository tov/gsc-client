use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use super::errors::*;

use serde;
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
    pub cookie:     Option<String>,
    pub endpoint:   String,
}

impl Default for Config {
    fn default() -> Self {
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
        }
    }
}

impl Config {
    pub fn get_username(&self) -> Result<&str> {
        match &self.username {
            Some(username) => Ok(&username),
            _              => Err(ErrorKind::NoUsernameGiven)?,
        }
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
        if !cookie.is_empty() { self.cookie = Some(cookie); }
        if !endpoint.is_empty() { self.endpoint = endpoint; }

        Ok(())
    }

    pub fn save_dotfile(&self) -> Result<()> {
        let dotfile_name = self.get_dotfile()?;
        let username = self.get_username()?.to_owned();
        let cookie = match &self.cookie {
            Some(cookie) => cookie.to_owned(),
            None         => "".to_owned(),
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
    pub username:   String,
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub cookie:     String,
    #[serde(default)]
    pub endpoint:   String,
}

