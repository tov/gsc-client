use std::{env,
          fmt,
          fs,
          io::{self, BufRead, Write},
          path::{Path, PathBuf},
};

use super::prelude::*;

use serde_derive::Deserialize;
use serde_yaml;

const API_ENDPOINT: &str    = "https://stewie.cs.northwestern.edu";

const AUTHFILE_VAR: &str    = "GSC_AUTH";
const AUTHFILE_NAME: &str   = ".gscauth";

const DOTFILE_VAR: &str     = "GSC_RC";
const DOTFILE_NAME: &str    = ".gscrc";

#[derive(Debug)]
pub struct Config {
    cookie_file: Option<PathBuf>,
    dotfile:     Option<PathBuf>,
    endpoint:    String,
    on_behalf:   Option<String>,
    overwrite:   OverwritePolicy,
    verbosity:   isize,
    json_output: bool,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum OverwritePolicy {
    Always,
    Never,
    Ask,
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
        let cookie_file = find_dotfile(AUTHFILE_VAR, AUTHFILE_NAME);
        let dotfile     = find_dotfile(DOTFILE_VAR, DOTFILE_NAME);

        Config {
            cookie_file,
            dotfile,
            endpoint:    API_ENDPOINT.to_owned(),
            on_behalf:   None,
            overwrite:   OverwritePolicy::Ask,
            verbosity:   1,
            json_output: false,
        }
    }

    pub fn get_on_behalf(&self) -> Option<&str> {
        self.on_behalf.as_ref().map(String::as_str)
    }

    pub fn set_on_behalf(&mut self, username: String) {
        self.on_behalf = Some(username);
    }

    pub fn get_overwrite_policy(&self) -> OverwritePolicy {
        self.overwrite
    }

    pub fn set_overwrite_policy(&mut self, op: OverwritePolicy) {
        self.overwrite = op;
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

    pub fn json_output(&self) -> bool {
        self.json_output
    }

    pub fn set_json_output(&mut self, json_output: bool) {
        self.json_output = json_output;
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
                io::ErrorKind::NotFound => return Ok(None),
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

impl OverwritePolicy {
    pub fn confirm_overwrite<D: fmt::Display, F: FnOnce() -> D>(
        &mut self, dst_thunk: F) -> Result<bool> {

        use OverwritePolicy::*;

        match *self {
            Always => Ok(true),
            Never  => Err(ErrorKind::DestinationFileExists(dst_thunk().to_string()))?,
            Ask    => {
                let     stdin = io::stdin();
                let mut input = stdin.lock();
                let mut buf   = String::with_capacity(2);
                let     dst   = dst_thunk();

                loop {
                    print!("File ‘{}’ already exists.\nOverwrite [Y/N/A/C]? ", dst);
                    io::stdout().flush()?;

                    input.read_line(&mut buf)?;

                    if buf.is_empty() {
                        std::process::exit(1);
                    }

                    match buf.chars().flat_map(char::to_lowercase).next() {
                        Some('y') => return Ok(true),
                        Some('n') => {
                            v2!("Skipping ‘{}’.", dst);
                            return Ok(false);
                        },
                        Some('a') => {
                            *self = Always;
                            return Ok(true);
                        }
                        Some('c') => std::process::exit(0),
                        _ => {
                            ve1!("");
                            ve1!("Did not understand response. Options are:");
                            ve1!("   [Y]es, overwrite just this file");
                            ve1!("   [N]o, do not overwrite this file");
                            ve1!("   overwrite [A]ll files");
                            ve1!("   [C]ancel operation and exit");
                            ve1!("");
                            buf.clear();
                        }
                    }
                }
            }
        }
    }
}
