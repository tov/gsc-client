use super::RemotePattern;

use error_chain::*;
use serde_derive::{Serialize, Deserialize};

use std::path::PathBuf;

/// This is the format of error messages produced by the server.
#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct JsonStatus {
    pub status:  u16,
    pub title:   String,
    pub message: String,
}

#[derive(Debug)]
pub struct RemoteFiles(pub Vec<String>);

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
                    contents.status, contents.title,
            contents.message)
        }

        UnknownHomework(number: usize) {
            description("unknown homework")
            display("Homework hw{} does not exist.", number)
        }

        SyntaxError(class: String, thing: String) {
            description("syntax error")
            display("Syntax error: could not parse ‘{}’ as {}.", thing, class)
        }

        PasswordMismatch {
            description("password mismatch")
            display("Passwords do not match.")
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

        DestinationFileExists(filename: String) {
            description("destination file exists, and flag ‘-n’ was given")
            display("Not overwriting destination file ‘{}’ (-n).", filename)
        }
    }
}

pub fn syntax_error<S1: Into<String>, S2: Into<String>>(class: S1, thing: S2) -> ErrorKind {
    ErrorKind::SyntaxError(class.into(), thing.into())
}

pub fn dest_pat_is_multiple(rpat: &RemotePattern,
                            rfile_metas: &[super::messages::FileMeta]) -> Error {

    let rfiles = RemoteFiles(rfile_metas.iter().map(|meta| meta.name.clone()).collect());
    ErrorKind::DestinationPatternIsMultiple(rpat.clone(), rfiles).into()
}

impl std::fmt::Display for RemoteFiles {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for filename in &self.0 {
            write!(f, " - {}\n", filename)?;
        }

        Ok(())
    }
}
