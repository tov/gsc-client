use super::RemotePattern;

use error_chain::*;
use serde_derive::{Serialize, Deserialize};

use std::path::PathBuf;

/// This is the format of error messages produced by the server.
#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct JsonError {
    pub status:  u16,
    pub title:   String,
    pub message: String,
}

error_chain! {
    foreign_links {
        Clap(clap::Error);
        Globset(globset::Error);
        Io(std::io::Error);
        Reqwest(reqwest::Error);
        SerdeYaml(serde_yaml::Error);
    }

    errors {
        ServerError(contents: JsonError) {
            description("error from server")
            display("Error response from server:\n  {} {}\n  {}",
                    contents.status, contents.title,
            contents.message)
        }

        UnknownHomework(number: usize) {
            description("unknown homework")
            display("Homework hw{} does not exist", number)
        }

        SyntaxError(class: String, thing: String) {
            description("syntax error")
            display("Syntax error: Could not parse {}: ‘{}’.", class, thing)
        }

        PasswordMismatch {
            description("password mismatch")
            display("Passwords do not match")
        }

        NoCommandGiven {
            description("no subcommand given")
            display("No subcommand given; pass -h for help.")
        }

        LoginPlease {
            description("login please")
            display("You are not logged in; use the ‘gsc auth’ command to authenticate.")
        }

        NoDotfileGiven {
            description("no dotfile given")
            display("Please specify a configuration file.")
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
    }
}

pub fn syntax_error<S1: Into<String>, S2: Into<String>>(class: S1, thing: S2) -> Error {
    ErrorKind::SyntaxError(class.into(), thing.into()).into()
}

pub fn no_command_given() -> Error {
    ErrorKind::NoCommandGiven.into()
}