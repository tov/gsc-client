use super::RemotePattern;

use error_chain::*;
use lazy_static::lazy_static;
use regex::Regex;
use serde_derive::{Deserialize, Serialize};

use std::borrow::Borrow;
use std::fmt;
use std::path::PathBuf;

/// This is the format of error messages produced by the server.
#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct JsonStatus {
    pub status: u16,
    pub title: String,
    pub message: String,
}

#[derive(Debug)]
pub struct RemoteFiles(pub Vec<String>);

#[derive(Debug)]
pub struct ApiKeyExplanation<S> {
    reasons: Vec<String>,
    bad_key: Option<S>,
}

impl<'a> ApiKeyExplanation<&'a str> {
    pub fn new() -> Self {
        Self {
            reasons: Vec::new(),
            bad_key: None,
        }
    }

    pub fn with_key(bad_key: &'a str) -> Self {
        Self {
            reasons: Vec::new(),
            bad_key: Some(bad_key),
        }
    }

    fn into_err<T>(self) -> self::Result<T> {
        Err(Error::from(ErrorKind::NotAnApiKey(ApiKeyExplanation {
            reasons: self.reasons,
            bad_key: self.bad_key.map(str::to_owned),
        })))
    }

    pub fn add(&mut self, reason: impl Into<String>) {
        self.reasons.push(reason.into());
    }

    pub fn final_straw<T>(mut self, reason: impl Into<String>) -> self::Result<T> {
        self.add(reason);
        self.into_err()
    }

    pub fn into_result(self) -> self::Result<()> {
        if self.reasons.is_empty() {
            Ok(())
        } else {
            self.into_err()
        }
    }
}

impl<S: Borrow<str>> fmt::Display for ApiKeyExplanation<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        const SHOW_LEN: usize = 30;

        if let Some(bad_key) = &self.bad_key {
            let bad_key = bad_key.borrow();
            write!(f, "\nThis doesn’t look like an API key: ")?;
            if let Some(abbrev) = bad_key.get(..SHOW_LEN) {
                write!(f, "\"")?;
                for c in abbrev.chars() {
                    write!(f, "{}", c.escape_default())?;
                }
                writeln!(f, "[…]\".")?;
            } else {
                writeln!(f, "{:?}.", bad_key)?;
            }
        } else {
            writeln!(f, "That doesn’t look like an API key.")?;
        }

        for reason in &self.reasons {
            writeln!(f, " - {}", reason)?;
        }

        if self.bad_key.is_none() {
            writeln!(f, "\nTo see the key you entered, re-run gsc with the -v flag.")?;
        }

        writeln!(f)
    }
}

error_chain! {
    foreign_links {
        Clap(clap::Error);
        Globset(globset::Error);
        InvalidHeaderValue(reqwest::header::InvalidHeaderValue);
        Io(std::io::Error);
        ParseInt(std::num::ParseIntError);
        ParseFloat(std::num::ParseFloatError);
        Reqwest(reqwest::Error);
        SerdeYaml(serde_yaml::Error);
    }

    errors {
        ServerError(contents: JsonStatus) {
            description("error from server")
            display("Error response from server:\n  {} {}\n  {}",
                    contents.status, contents.title, contents.message)
        }

        NotAnApiKey(explanation: ApiKeyExplanation<String>) {
            description("doesn't look like an API key")
            display("{}", explanation)
        }

        UnknownHomework(number: usize) {
            description("unknown homework")
            display("Homework hw{} does not exist.", number)
        }

        SyntaxError(class: String, thing: String) {
            description("syntax error")
            display("Syntax error: could not parse ‘{}’ as {}.", thing, class)
        }

        NoCommandGiven {
            description("no subcommand given")
            display("No subcommand given; pass -h for help.")
        }

        LoginPlease {
            description("login please")
            display("You are not logged in; use the ‘gsc auth’ command to authenticate.")
        }

        NoCookieFileGiven {
            description("no cookie file given")
            display("Please specify a cookie file.")
        }

        NoSuchRemoteFile(rpat: RemotePattern) {
            description("no such remote file")
            display("No remote files matching pattern ‘{}’.", rpat)
        }

        CannotCopyLocalToLocal(src: PathBuf, dst: PathBuf) {
            description("cannot copy local to local")
            display("Cannot copy local file ({}) to local destination ({}).",
                    src.display(), dst.display())
        }

        CannotCopyLocalToLocalExtra(src: PathBuf, dst: PathBuf, extra: String) {
            description("cannot copy local to local")
            display("Cannot copy local file ({}) to local destination ({}).\n{}",
                    src.display(), dst.display(), extra)
        }

        CannotCopyRemoteToRemote(src: RemotePattern, dst: RemotePattern) {
            description("cannot copy remote to remote")
            display("Cannot copy remote file ({}) to remote destination ({}).", src, dst)
        }

        BadLocalPath(filename: PathBuf) {
            description("bad local path")
            display("Not a well-formed local file path: ‘{}’.", filename.display())
        }

        FilenameNotUtf8(filename: PathBuf) {
            description("filename not UTF-8")
            display("Filename not proper UTF-8: ‘{}’.", filename.display())
        }

        MultipleSourcesOneDestination {
            description("multiple sources one destination")
            display("Multiple source files cannot be copied to one destination file.")
        }

        DestinationPatternIsMultiple(rpat: RemotePattern, rfiles: RemoteFiles) {
            description("destination pattern is multiple")
            display("Destination pattern ‘{}’ resolves to multiple remote files:\n{}", rpat, rfiles)
        }

        SourceHwToDestinationFile(src: usize, dst: PathBuf) {
            description("source homework to destination file")
            display("Cannot copy whole source homework ‘hw{}’ over file destination ‘{}’.",
                    src, dst.display())
        }

        CommandRequiresFlag(command: String) {
            description("command requires ‘-a’ flag")
            display("To ‘{}’ a whole homework, you must provide the ‘-a’ flag.", command)
        }

        NoInformationalEvalItem {
            description("no informational eval item")
            display("Could not find informational eval item to add score to.")
        }

        EvalItemDoesNotExist(hw: usize, number: usize) {
            description("requested eval item does not exist")
            display("Homework {} does not have item {}.", hw, number)
        }

        DestinationFileExists(filename: String) {
            description("destination file exists, and flag ‘-n’ was given")
            display("Not overwriting destination file ‘{}’ (-n).", filename)
        }
    }
}

impl ErrorKind {
    pub fn syntax(class: impl Into<String>, thing: impl Into<String>) -> Self {
        Self::SyntaxError(class.into(), thing.into())
    }

    pub fn dest_pat_is_multiple(
        rpat: &RemotePattern,
        rfile_metas: &[super::messages::FileMeta],
    ) -> Self {
        let rfiles = RemoteFiles(rfile_metas.iter().map(|meta| meta.name.clone()).collect());
        Self::DestinationPatternIsMultiple(rpat.clone(), rfiles)
    }

    pub fn cannot_copy_local_to_local(src: impl Into<PathBuf>,
                                      dst: impl Into<PathBuf>) -> Self {

        lazy_static! {
            pub static ref HW_NUM: Regex = Regex::new(r"^hw\d+$").unwrap();
        }

        let src = src.into();
        let dst = dst.into();

        let dst_str = dst.display().to_string();

        if HW_NUM.is_match(&dst_str) {
            let message = format!("Did you leave out the colon ({}:)?", dst_str);
            Self::CannotCopyLocalToLocalExtra(src, dst, message)
        } else {
            Self::CannotCopyLocalToLocal(src, dst)
        }
    }
}

impl Error {
    pub fn syntax(class: impl Into<String>, thing: impl Into<String>) -> Self {
        ErrorKind::syntax(class, thing).into()
    }

    pub fn dest_pat_is_multiple(
        rpat: &RemotePattern,
        rfile_metas: &[super::messages::FileMeta],
    ) -> Self {
        ErrorKind::dest_pat_is_multiple(rpat, rfile_metas).into()
    }
}

impl std::fmt::Display for RemoteFiles {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for filename in &self.0 {
            write!(f, " - {}\n", filename)?;
        }

        Ok(())
    }
}
