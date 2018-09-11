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
        Reqwest(reqwest::Error);
        Io(std::io::Error);
        SerdeYaml(serde_yaml::Error);
    }

    errors {
        ServerError(contents: JsonError) {
            description("error from server")
            display("Error {}: {}\n{}", contents.status, contents.title, contents.message)
        }

        LoginPlease {
            description("login please")
            display("you are not logged in")
        }

        NoUsernameGiven {
            description("no username given")
            display("please specify a username")
        }

        NoDotfileGiven {
            description("no dotfile given")
            display("please specify a configuration file")
        }
    }
}