use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use super::errors::*;

use serde_derive::Deserialize;
use serde_yaml;

const API_ENDPOINT: &str = "http://localhost:9090";

const COOKIEFILE_VAR: &str  = "GSC_LOGIN";
const COOKIEFILE_NAME: &str = ".gsclogin";

const DOTFILE_VAR: &str     = "GSC_DOTFILE";
const DOTFILE_NAME: &str    = ".gscrc";

#[derive(Debug)]
pub struct Config {
    cookie_file: Option<PathBuf>,
    dotfile:     Option<PathBuf>,
    endpoint:    String,
    on_behalf:   Option<String>,
    verbosity:   isize,
}

/// This is the format of the dotfile.
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Dotfile {
    #[serde(default)]
    pub endpoint:   String,
    #[serde(default)]
    pub verbosity:  Option<isize>,
}

fn find_dotfile(env_var: &str, filename: &str) -> Option<PathBuf> {
    match env::var_os(env_var) {
        Some(file) => Some(PathBuf::from(file)),
        None       => match env::var_os("HOME") {
            Some(home) => {
                let mut buf = PathBuf::from(home);
                buf.push(filename);
                Some(buf)
            },
            None       => None,
        }
    }
}

impl Config {
    pub fn new() -> Self {
        let cookie_file = find_dotfile(COOKIEFILE_VAR, COOKIEFILE_NAME);
        let dotfile     = find_dotfile(DOTFILE_VAR, DOTFILE_NAME);

        Config {
            cookie_file,
            dotfile,
            endpoint:    API_ENDPOINT.to_owned(),
            on_behalf:   None,
            verbosity:   1,
        }
    }

    pub fn get_on_behalf(&self) -> Option<&str> {
        self.on_behalf.as_ref().map(String::as_str)
    }

    pub fn set_on_behalf(&mut self, username: String) {
        self.on_behalf = Some(username);
    }

    pub fn get_verbosity(&self) -> isize {
        self.verbosity
    }

    pub fn activate_verbosity(&self) {
        let verbosity = if self.verbosity > 0 {
            self.verbosity as usize
        } else {
            0
        };

        vlog::set_verbosity_level(verbosity);
    }

    pub fn set_verbosity(&mut self, verbosity: isize) {
        self.verbosity = verbosity;
    }

    pub fn get_endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn get_cookie_file(&self) -> Result<&Path> {
        match &self.cookie_file {
            Some(filename) => Ok(&filename),
            _              => Err(ErrorKind::NoCookieFileGiven)?,
        }
    }

    pub fn get_dotfile(&self) -> Option<&Path> {
        self.dotfile.as_ref().map(PathBuf::as_path)
    }

    pub fn read_dotfile(&self) -> Result<Option<Dotfile>> {
        let dotfile_name = match self.get_dotfile() {
            None           => return Ok(None),
            Some(filename) => filename,
        };

        let contents     = match fs::read_to_string(dotfile_name) {
            Ok(contents)   => contents,
            Err(error)     => match error.kind() {
                std::io::ErrorKind::NotFound => return Ok(None),
                _ => {
                    let message = format!("Could not read dotfile: {}", dotfile_name.display());
                    return Err(Error::with_chain(error, message));
                }
            }
        };
        
        let parsed = serde_yaml::from_str(&contents)
            .chain_err(|| format!("Could not parse dotfile: {}", dotfile_name.display()))?;

        Ok(Some(parsed))
    }

    pub fn load_dotfile(&mut self) -> Result<()> {
        if let Some(Dotfile {endpoint, verbosity}) = self.read_dotfile()? {
            if !endpoint.is_empty() {
                self.endpoint = endpoint;
            }

            if let Some(i) = verbosity {
                self.verbosity = i;
            }
        }

        Ok(())
    }
}


