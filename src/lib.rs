use vlog::*;
use reqwest;
use rpassword;

pub mod config;
pub mod errors;
pub mod messages;

use self::errors::{Error, ErrorKind, JsonError, Result};

pub struct GscClient {
    http:   reqwest::Client,
    config: config::Config,
}

pub (crate) fn parse_cookie(cookie: &str) -> Option<(String, String)> {
    let pair = match cookie.find(';') {
        Some(index) => &cookie[.. index],
        None        => cookie,
    };

    pair.find('=').map(|index| {
        let key   = pair[.. index].to_owned();
        let value = pair[index + 1 ..].to_owned();
        (key, value)
    })
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
    pub fn new(config: config::Config) -> Result<Self> {
        let http = reqwest::Client::new();
        Ok(GscClient { http, config })
    }

    pub fn auth(&mut self, username: &str) -> Result<()> {
        let uri = format!("{}/api/users/{}", self.config.endpoint, username);

        self.config.username = Some(username.to_owned());

        loop {
            let password = self.prompt_password("Password")?;
            let mut response = self.http.get(&uri)
                .basic_auth(username, Some(password))
                .send()?;

            match self.handle_response(&mut response) {
                Ok(()) =>
                    return Ok(()),
                Err(e @ Error(ErrorKind::ServerError(JsonError { status: 401, .. }), _)) =>
                    eprintln!("{}", e),
                e =>
                    e?,
            }
        }
    }

    pub fn deauth(&mut self) {
        self.config.cookie   = None;
        self.config.username = None;
        self.config.save     = true;
    }

    fn ls_submissions(&mut self) -> Result<Vec<messages::SubmissionShort>> {
        let uri          = format!("{}/api/users/{}/submissions",
                                   self.config.endpoint,
                                   self.config.get_username()?);
        let request      = self.http.get(&uri);
        let mut response = self.send_request(request)?;
        Ok(response.json()?)
    }

    fn get_uri_for_submission(&mut self, number: usize) -> Result<String> {
        let submissions = self.ls_submissions()?;

        for submission in &submissions {
            if submission.assignment_number == number {
                return Ok(format!("{}{}", self.config.endpoint, submission.uri));
            }
        }

        Err(errors::ErrorKind::UnknownHomework(number))?
    }

    pub fn ls_submission(&mut self, number: usize) -> Result<()>
    {
        let uri          = self.get_uri_for_submission(number)?;
        let request      = self.http.get(&uri);
        let mut response = self.send_request(request)?;

        let submission: messages::Submission = response.json()?;
        v1!("{:?}", submission);
        Ok(())
    }

    pub fn get_users(&mut self) -> Result<String> {
        let uri          = format!("{}/api/users", self.config.endpoint);;
        let request      = self.http.get(&uri);
        let mut response = self.send_request(request)?;
        Ok(response.text()?)
    }

    fn send_request(&mut self, mut req_builder: reqwest::RequestBuilder)
        -> Result<reqwest::Response> {

        self.prepare_cookie(&mut req_builder)?;
        let request = req_builder.build()?;
        ve2!("> Sending request to {}", request.url());
        let mut response = self.http.execute(request)?;
        self.handle_response(&mut response)?;
        Ok(response)
    }

    fn handle_response(&mut self, response: &mut reqwest::Response) -> Result<()> {
        self.save_cookie(response);

        if response.status().is_success() {
            Ok(())
        } else {
            let error = response.json()?;
            Err(ErrorKind::ServerError(error))?
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

