use vlog::*;
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

    if let Some(index) = pair.find('=') {
        let key   = pair[.. index].to_owned();
        let value = pair[index + 1 ..].to_owned();
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

        let mut config = config::Config::new();
        config.load_dotfile()?;

        Ok(GscClient { http, config })
    }

    pub fn login(&mut self, username: &str) -> Result<String> {
        let uri = format!("{}/users/{}", self.config.endpoint, username);

        self.config.username = Some(username.to_owned());
        let password = self.prompt_password("Password")?;

        let mut response = self.http.get(&uri)
            .basic_auth(username, Some(password))
            .send()?;

        self.save_cookie(&response);

        Ok(response.text()?)
    }

    pub fn logout(&mut self) {
        self.config.cookie = None;
        self.config.username = Some("".to_owned());
    }

    pub fn get_users(&mut self) -> Result<String> {
        let uri          = format!("{}/users", self.config.endpoint);;
        let request      = self.http.get(&uri);
        let mut response = self.send_request(request)?;
        Ok(response.text()?)
    }

    fn send_request(&mut self, mut request: reqwest::RequestBuilder)
        -> Result<reqwest::Response> {

        self.prepare_cookie(&mut request)?;
        let response = request.send()?;
        self.save_cookie(&response);
        Ok(response)
    }

    fn prompt_password(&self, prompt: &str) -> Result<String> {
        let prompt   = format!("{} for {}: ", prompt, self.config.get_username()?);
        let password = rpassword::prompt_password_stderr(&prompt)?;
        Ok(password)
    }

    fn prepare_cookie(&self, request: &mut reqwest::RequestBuilder) -> Result<()> {
        let cookie = self.config.get_cookie_header()?;
        request.header(cookie);
        Ok(())
    }

    fn save_cookie(&mut self, response: &reqwest::Response) -> bool {
        if let Some(reqwest::header::SetCookie(chunks)) = response.headers().get() {
            if let Some(cookie) = parse_cookies(&chunks) {
                ve3!("< Received cookie: {}={}", cookie.0, cookie.1);
                self.config.cookie = Some(cookie);
                self.config.save   = true;
                return true;
            }
        }

        false
    }

}

