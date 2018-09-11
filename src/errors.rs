use error_chain::*;
use reqwest;

error_chain! {
    foreign_links {
        Reqwest(reqwest::Error);
        Io(std::io::Error);
        SerdeYaml(serde_yaml::Error);
    }

    errors {
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