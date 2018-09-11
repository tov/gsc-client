use reqwest;
use rpassword;

pub mod errors;
pub mod config;

use self::errors::Result;

pub struct GscClient {
    http:   reqwest::Client,
    config: config::Config,
}

pub (crate) fn parse_cookie(cookie: &str) -> Option<(String, String)> {
    let pair = match cookie.find(';') {
        Some(index) => &cookie[.. index],
        None        => cookie,
    };

    if let Some(index) = cookie.find('=') {
        let key   = cookie[.. index].to_owned();
        let value = cookie[index + 1 ..].to_owned();
        return Some((key, value));
    } else {
        return None;
    }
}

pub (crate) fn parse_cookies(chunks: &[String]) -> Option<(String, String)> {
    for chunk in chunks {
        if let Some(pair) = parse_cookie(&chunk) {
            return Some(pair);
        }
    }

    None
}

impl GscClient {
    pub fn new() -> Result<Self> {
        let http = reqwest::Client::new();

        let mut config = config::Config::default();
        config.load_dotfile()?;

        Ok(GscClient { http, config })
    }

    pub fn login(&mut self) -> Result<String> {
        let username     = self.config.get_username()?;
        let uri          = format!("{}/users/{}", self.config.endpoint, username);

        let password     = rpassword::prompt_password_stderr("Password: ")?;

        let mut response = self.http.get(&uri)
            .basic_auth(username, Some(password))
            .send()?;

        if let Some(reqwest::header::SetCookie(chunks)) = response.headers().get() {
            self.config.cookie = parse_cookies(&chunks);
        }

        Ok(response.text()?)
    }

    pub fn get_users(&mut self) -> Result<String> {
        let cookie       = self.config.get_cookie_header()?;
        let uri          = format!("{}/users", self.config.endpoint);;
        let mut request  = self.http.get(&uri);
        request.header(cookie);
        let mut response = request.send()?;

        if let Some(reqwest::header::SetCookie(chunks)) = response.headers().get() {
            self.config.cookie = parse_cookies(&chunks);
        }

        Ok(response.text()?)
    }
}

