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

pub fn parse_cookies(chunks: &[String]) -> Option<(String, String)> {
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
        self.handle_response(&mut response)?;

        Ok(response.text()?)
    }

    pub fn logout(&mut self) {
        self.config.cookie   = None;
        self.config.username = Some("".to_owned());
        self.config.save     = true;
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
        let mut response = request.send()?;
        self.handle_response(&mut response)?;
        Ok(response)
    }

    fn handle_response(&mut self, response: &mut reqwest::Response) -> Result<()> {
        if response.status().is_success() {
            self.save_cookie(response);
            Ok(())
        } else {
            let error = response.json()?;
            Err(errors::ErrorKind::ServerError(error))?
        }
    }

    fn prompt_password(&self, prompt: &str) -> Result<String> {
        let prompt   = format!("{} for {}: ", prompt, self.config.get_username()?);
        let password = rpassword::prompt_password_stderr(&prompt)?;
        Ok(password)
    }

    fn prepare_cookie(&self, request: &mut reqwest::RequestBuilder) -> Result<()> {
        let cookie = self.config.get_cookie_header()?;
        ve2!("> Sending cookie: {}", cookie);
        request.header(cookie);
        Ok(())
    }

    fn save_cookie(&mut self, response: &reqwest::Response) -> bool {
        if let Some(reqwest::header::SetCookie(chunks)) = response.headers().get() {
            if let Some(cookie) = parse_cookies(&chunks) {
                ve2!("< Received cookie: {}={}", cookie.0, cookie.1);
                self.config.cookie = Some(cookie);
                self.config.save   = true;
                return true;
            }
        }

        false
    }

}

