use vlog::*;
use std::collections::{hash_map, HashMap};
use std::mem::replace;

pub mod config;
pub mod errors;
pub mod messages;
pub mod table;

use self::errors::{Error, ErrorKind, JsonError, Result};
use self::config::DotfileLock;

pub struct GscClient {
    http:               reqwest::Client,
    config:             config::Config,
    submission_uris:    HashMap<String, Vec<Option<String>>>,
    had_warning:        bool,
}

pub struct RemotePattern {
    pub hw:     usize,
    pub pat:    String,
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
        Ok(GscClient {
            http:               reqwest::Client::new(),
            config,
            submission_uris:    HashMap::new(),
            had_warning:        false,
        })
    }

    pub fn had_warning(&self) -> bool {
        self.had_warning
    }

    pub fn auth(&mut self, username: &str) -> Result<()> {
        let uri = format!("{}/api/users/{}", self.config.get_endpoint(), username);

        self.config.set_username(username.to_owned());

        loop {
            let password = prompt_password("Password", username)?;
            let mut response = self.http.get(&uri)
                .basic_auth(username, Some(password))
                .send()?;

            let cookie_lock = self.config.new_cookie()?;
            match self.handle_response(&mut response, cookie_lock) {
                Ok(()) =>
                    return Ok(()),
                Err(e @ Error(ErrorKind::ServerError(JsonError { status: 401, .. }), _)) =>
                    eprintln!("{}", e),
                e =>
                    e?,
            }
        }
    }

    pub fn deauth(&mut self) -> Result<()> {
        let mut cookie_lock = self.config.new_cookie()?;
        cookie_lock.deauth();
        Ok(())
    }

    fn fetch_submissions(&mut self, user: &str)
        -> Result<Vec<messages::SubmissionShort>> {

        let uri          = format!("{}/api/users/{}/submissions", self.config.get_endpoint(), user);
        let request      = self.http.get(&uri);
        let mut response = self.send_request(request)?;
        response.json()
            .map_err(|e| Error::with_chain(e, "Could not understand response from server"))
    }

    fn get_submission_uris(&mut self, user: &str) -> Result<Vec<Option<String>>> {
        let submissions = self.fetch_submissions(user)?;
        let mut result  = Vec::new();

        for submission in &submissions {
            let number = submission.assignment_number;

            while number >= result.len() {
                result.push(None);
            }

            result[number] = Some(format!("{}{}", self.config.get_endpoint(), submission.uri));
        }

        Ok(result)
    }

    fn get_uri_for_submission(&mut self, user_option: Option<&str>, number: usize)
        -> Result<String> {

        let user        = self.select_user(user_option).to_owned();
        let mut cache   = replace(&mut self.submission_uris, HashMap::new());
        let uris        = match cache.entry(user.clone()) {
            hash_map::Entry::Occupied(entry) => entry.into_mut(),
            hash_map::Entry::Vacant(entry) => entry.insert(self.get_submission_uris(&user)?),
        };

        match uris.get(number) {
            Some(Some(uri)) => Ok(uri.to_owned()),
            _               => Err(Error::from(ErrorKind::UnknownHomework(number))),
        }
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

        if files.is_empty() {
            return Err(Error::from(ErrorKind::NoSuchRemoteFile(number, pattern.to_owned())));
        }

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

    pub fn cat(&mut self, user: Option<&str>, pats: &[RemotePattern]) -> Result<()> {
        for RemotePattern { hw, pat } in pats {
            let files = self.fetch_file_list(user, *hw, pat)?;

            if files.is_empty() {
                let error = Error::from(ErrorKind::NoSuchRemoteFile(*hw, pat.to_owned()));
                ve1!("{}", error);
                self.had_warning = true;
            }

            for file in files {
                let uri          = format!("{}{}", self.config.get_endpoint(), file.uri);
                let request      = self.http.get(&uri);
                let mut response = self.send_request(request)?;
                response.copy_to(&mut std::io::stdout())?;
            }
        }

        Ok(())
    }

    pub fn create(&mut self, username: &str) -> Result<()> {
        self.config.set_username(username.to_owned());

        let password = get_matching_passwords(username)?;
        let uri      = format!("{}/api/users", self.config.get_endpoint());

        ve2!("> Sending request to {}", uri);
        let mut response = self.http.post(&uri)
            .basic_auth(username, Some(password))
            .send()?;
        let cookie_lock = self.config.new_cookie()?;
        self.handle_response(&mut response, cookie_lock)?;

        Ok(())
    }

    pub fn passwd(&mut self, user_option: Option<&str>) -> Result<()> {
        let user         = self.select_user(user_option);
        let password     = get_matching_passwords(user)?;
        let message      = messages::PasswordChange { password };
        let uri          = format!("{}/api/users/{}", self.config.get_endpoint(), user);
        let mut request  = self.http.patch(&uri);
        request.json(&message);
        self.send_request(request)?;

        Ok(())
    }

    pub fn rm(&mut self, user: Option<&str>, pats: &[RemotePattern]) -> Result<()> {
        for RemotePattern { hw, pat } in pats {
            let files = self.fetch_file_list(user, *hw, pat)?;

            if files.is_empty() {
                let error = Error::from(ErrorKind::NoSuchRemoteFile(*hw, pat.to_owned()));
                ve1!("{}", error);
                self.had_warning = true;
            }

            for file in files {
                let uri          = format!("{}{}", self.config.get_endpoint(), file.uri);
                let request      = self.http.delete(&uri);
                self.send_request(request)?;
            }
        }

        Ok(())
    }

    pub fn whoami(&self) -> Result<()> {
        let username = self.config.get_username();

        if username.is_empty() {
            return Err(Error::from(ErrorKind::LoginPlease))
        }

        v1!("{}", username);
        Ok(())
    }

    pub fn get_users(&mut self) -> Result<String> {
        let uri          = format!("{}/api/users", self.config.get_endpoint());;
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

        let cookie_lock = self.prepare_cookie(&mut req_builder)?;
        let request     = req_builder.build()?;
        ve2!("> Sending request to {}", request.url());
        let mut response = self.http.execute(request)?;
        self.handle_response(&mut response, cookie_lock)?;
        Ok(response)
    }

    fn handle_response(&mut self,
                       response: &mut reqwest::Response,
                       cookie_lock: DotfileLock)
        -> Result<()> {

        ve3!("< Raw response from server: {:?}", response);

        self.save_cookie(response, cookie_lock)?;

        if response.status().is_success() {
            Ok(())
        } else {
            let error = response.json()?;
            Err(ErrorKind::ServerError(error))?
        }
    }

    fn prepare_cookie(&mut self, request: &mut reqwest::RequestBuilder) -> Result<DotfileLock> {
        let cookie_lock = self.config.lock_dotfile()?;
        let cookie      = cookie_lock.get_cookie_header();
        ve2!("> Sending cookie: {}", cookie);
        request.header(cookie);
        Ok(cookie_lock)
    }

    fn save_cookie(&mut self,
                   response: &reqwest::Response,
                   mut cookie_lock: DotfileLock)
        -> Result<()> {

        if let Some(reqwest::header::SetCookie(chunks)) = response.headers().get() {
            if let Some((key, value)) = parse_cookies(&chunks) {
                ve2!("< Received cookie: {}={}", key, value);
                cookie_lock.set_cookie(key, value);
            }
        }

        Ok(())
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

