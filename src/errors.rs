use error_chain::*;
use reqwest;
use serde_derive::{Serialize, Deserialize};

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

        SyntaxError(thing: String) {
            description("syntax error")
            display("Syntax error: could not parse {}", thing)
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

        NoSuchRemoteFile(hw: usize, pat: String) {
            description("no such remote file")
            display("No remote files matching pattern ‘hw{}:{}’.", hw, pat)
        }
    }
}