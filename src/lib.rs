use vlog::*;
use reqwest;
use rpassword;

pub mod config;
pub mod errors;
pub mod messages;
pub mod table;

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

fn glob(pattern: &str) -> Result<globset::GlobMatcher> {
    let real_pattern = if pattern.is_empty() { "*" } else { pattern };
    Ok(globset::Glob::new(real_pattern)?.compile_matcher())
}

impl GscClient {
    pub fn new(config: config::Config) -> Result<Self> {
        let http = reqwest::Client::new();
        Ok(GscClient { http, config })
    }

    pub fn auth(&mut self, username: &str) -> Result<()> {
        let uri = format!("{}/api/users/{}", self.config.endpoint, username);

        self.config.username = username.to_owned();

        loop {
            let password = prompt_password("Password", username)?;
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
        self.config.username = String::new();
        self.config.save     = true;
    }

    fn fetch_submissions(&mut self, user_option: Option<&str>)
        -> Result<Vec<messages::SubmissionShort>> {

        let user         = self.select_user(user_option);
        let uri          = format!("{}/api/users/{}/submissions", self.config.endpoint, user);
        let request      = self.http.get(&uri);
        let mut response = self.send_request(request)?;
        response.json()
            .map_err(|e| Error::with_chain(e, "Could not understand response from server"))
    }

    fn get_uri_for_submission(&mut self, user: Option<&str>, number: usize)
        -> Result<String> {

        let submissions = self.fetch_submissions(user)?;

        for submission in &submissions {
            if submission.assignment_number == number {
                return Ok(format!("{}{}", self.config.endpoint, submission.uri));
            }
        }

        Err(errors::ErrorKind::UnknownHomework(number))?
    }

    fn get_uri_for_submission_files(&mut self, user: Option<&str>, number: usize)
        -> Result<String> {

        self.get_uri_for_submission(user, number).map(|uri| uri + "/files")
    }

    pub fn fetch_file_list(&mut self,
                           user: Option<&str>,
                           hw_number: usize,
                           pattern: &str)
        -> Result<Vec<messages::FileMeta>>
    {
        let matcher      = glob(pattern)?;
        let uri          = self.get_uri_for_submission_files(user, hw_number)?;
        let request      = self.http.get(&uri);
        let mut response = self.send_request(request)?;

        let files: Vec<messages::FileMeta> = response.json()?;

        Ok(files.into_iter()
               .filter(|file| matcher.is_match(&file.name))
               .collect())
    }

    pub fn ls(&mut self,
              user: Option<&str>,
              number: usize,
              pattern: &str)
        -> Result<()> {

        let files     = self.fetch_file_list(user, number, pattern)?;
        let mut table = table::TextTable::new("%r  %l  [%l] %l\n");

        for file in &files {
            table.add_row(
                table::Row::new()
                    .add_cell(file.byte_count)
                    .add_cell(&file.upload_time)
                    .add_cell(&file.purpose[..1])
                    .add_cell(&file.name));
        }

        v1!("{}", table);

        Ok(())
    }

    pub fn status(&mut self, user: Option<&str>, number: usize) -> Result<()>
    {
        let uri          = self.get_uri_for_submission(user, number)?;
        let request      = self.http.get(&uri);
        let mut response = self.send_request(request)?;

        let submission: messages::Submission = response.json()?;
        let in_evaluation = submission.status.is_self_eval();

        let mut table = table::TextTable::new("  %l  %l\n");
        table.add_row(table::Row::new().add_cell("Submission status:")
            .add_cell(submission.status));

        if in_evaluation {
            table.add_row(table::Row::new().add_cell("Evaluation status:")
                .add_cell(submission.eval_status));
        }

        table
            .add_row(table::Row::new().add_cell("Open date:")
                .add_cell(submission.open_date))
            .add_row(table::Row::new().add_cell("Submission due date:")
                .add_cell(submission.due_date))
            .add_row(table::Row::new().add_cell("Self-eval due date:")
                .add_cell(submission.eval_date))
            .add_row(table::Row::new().add_cell("Last modified:")
                .add_cell(submission.last_modified))
            .add_row(table::Row::new().add_cell("Bytes used:")
                .add_cell(format!("{} (of {} allowed)",
                                  submission.bytes_used,
                                  submission.bytes_quota)));

        let mut owners = submission.owner1.name.clone();
        if let Some(owner2) = &submission.owner2 {
            owners += " and ";
            owners += &owner2.name;
        }

        v1!("hw{} ({})", number, owners);
        v1!("{}", table);

        Ok(())
    }

    pub fn create(&mut self, username: &str) -> Result<()> {
        self.config.username = username.to_owned();

        let password = get_matching_passwords(username)?;
        let uri      = format!("{}/api/users", self.config.endpoint);

        ve2!("> Sending request to {}", uri);
        let mut response = self.http.post(&uri)
            .basic_auth(username, Some(password))
            .send()?;
        self.handle_response(&mut response)?;

        Ok(())
    }

    pub fn passwd(&mut self, user_option: Option<&str>) -> Result<()> {
        let user         = self.select_user(user_option);
        let uri          = format!("{}/api/users/{}", self.config.endpoint, user);
        let password     = get_matching_passwords(user)?;
        let message      = messages::PasswordChange { password };
        let mut request  = self.http.patch(&uri);
        request.json(&message);
        self.send_request(request)?;

        Ok(())
    }

    pub fn get_users(&mut self) -> Result<String> {
        let uri          = format!("{}/api/users", self.config.endpoint);;
        let request      = self.http.get(&uri);
        let mut response = self.send_request(request)?;
        Ok(response.text()?)
    }

    fn select_user<'a>(&'a self, username_option: Option<&'a str>) -> &'a str {
        match username_option {
            Some(s) => s,
            None    => self.config.get_username(),
        }
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
        ve3!("< Raw response from server: {:?}", response);

        self.save_cookie(response);

        if response.status().is_success() {
            Ok(())
        } else {
            let error = response.json()?;
            Err(ErrorKind::ServerError(error))?
        }
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

fn get_matching_passwords(username: &str) -> Result<String> {
    let password1 = prompt_password("New password", username)?;
    let password2 = prompt_password("Confirm password", username)?;

    if password1 == password2 {
        Ok(password1)
    } else {
        Err(errors::ErrorKind::PasswordMismatch)?
    }
}

fn prompt_password(prompt: &str, username: &str) -> Result<String> {
    let prompt   = format!("{} for {}: ", prompt, username);
    let password = rpassword::prompt_password_stderr(&prompt)?;
    Ok(password)
}

