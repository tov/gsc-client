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
        Io(std::io::Error);
        Reqwest(reqwest::Error);
        SerdeYaml(serde_yaml::Error);
    }

    errors {
        ServerError(contents: JsonError) {
            description("error from server")
            display("Error response from server:\n  {}: {}\n  {}",
                    contents.status, contents.title,
            contents.message)
        }

        NoCommandGiven {
            description("no subcommand given")
            display("No subcommand given; pass -h for help.")
        }

        LoginPlease {
            description("login please")
            display("You are not logged in.")
        }

        NoUsernameGiven {
            description("no username given")
            display("Please specify a username.")
        }

        NoDotfileGiven {
            description("no dotfile given")
            display("Please specify a configuration file.")
        }
    }
}