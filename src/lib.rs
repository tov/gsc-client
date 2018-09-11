use reqwest;

pub mod errors;
pub mod config;

use self::errors::Result;

pub struct GscClient {
    http:   reqwest::Client,
    config: config::Config,
}

impl GscClient {
    pub fn new() -> Result<Self> {
        let http = reqwest::Client::new();

        let mut config = config::Config::default();
        eprintln!("config before dotfile: {:?}", config);
        config.load_dotfile()?;
        eprintln!("config after dotfile: {:?}", config);

        config.save_dotfile()?;

        Ok(GscClient { http, config })
    }

    pub fn login(&self) -> Result<String> {
        let uri          = format!("{}/users/{}", self.config.endpoint, "root");
        let password     = "foo";
        let mut response = self.http.get(&uri)
            .basic_auth("root", Some(password))
            .send()?;


        if let Some(reqwest::header::SetCookie(chunks)) = response.headers().get() {
            eprintln!("Set-Cookie: {:?}", chunks);
        }

        Ok("".to_owned())
    }

    pub fn get_users(&self) -> Result<String> {
        let uri          = format!("{}/users", self.config.endpoint);
        let mut response = self.http.get(&uri)
            .basic_auth("root", Some("foo"))
            .send()?;
        let text = response.text()?;
        Ok(text)
    }
}

