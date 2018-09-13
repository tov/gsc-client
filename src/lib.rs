#![recursion_limit = "128"]

use vlog::*;
use percent_encoding::{utf8_percent_encode, PATH_SEGMENT_ENCODE_SET};
use std::collections::{hash_map, HashMap};
use std::io;
use std::mem::replace;
use std::path::{Path, PathBuf};

pub mod config;
pub mod errors;
pub mod messages;
pub mod table;

use self::errors::*;
use self::config::DotfileLock;

pub struct GscClient {
    http:               reqwest::Client,
    config:             config::Config,
    submission_uris:    HashMap<String, Vec<Option<String>>>,
    had_warning:        bool,
}

#[derive(Clone, Debug)]
pub struct RemotePattern {
    pub hw:     usize,
    pub pat:    String,
}

pub enum CpArg {
    Local(PathBuf),
    Remote(RemotePattern),
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
                Ok(()) => {
                    v2!("Authenticated as {}", username);
                    return Ok(());
                }
                Err(e @ Error(ErrorKind::ServerError(JsonError { status: 401, .. }), _)) =>
                    eprintln!("{}", e),
                e =>
                    e?,
            }
        }
    }

    pub fn cp(&mut self, user: Option<&str>, srcs: &[CpArg], dst: &CpArg)
        -> Result<()> {

        match dst {
            CpArg::Local(filename) => self.cp_dn(user, srcs, filename),
            CpArg::Remote(rpat)    => self.cp_up(user, srcs, rpat),
        }
    }

    pub fn cp_dn(&mut self, user: Option<&str>, raw_srcs: &[CpArg], dst: &Path)
              -> Result<()> {

        let mut src_rpats = Vec::new();

        for src in raw_srcs {
            match src {
                CpArg::Local(filename) =>
                    Err(ErrorKind::CannotCopyLocalToLocal(filename.clone(), dst.to_owned()))?,
                CpArg::Remote(rpat)    =>
                    src_rpats.push(rpat),
            }
        }

        enum DstType {
            Dir,
            File,
            DoesNotExist,
        }

        let dst_type = match dst.metadata() {
            Err(e) =>
                match e.kind() {
                    io::ErrorKind::NotFound => DstType::DoesNotExist,
                    _                       => Err(e)?,
                }
            Ok(metadata) =>
                if metadata.is_dir() {
                    DstType::Dir
                } else {
                    DstType::File
                }
        };

        let mut src_files = Vec::new();

        for src_rpat in &src_rpats {
            let whole_hw = src_rpat.pat.is_empty();
            src_files.extend(
                self.fetch_file_list(user, src_rpat)?
                    .into_iter()
                    .map(|meta| {
                        let hw  = src_rpat.hw;
                        let pat = meta.name.clone();
                        (meta, whole_hw, RemotePattern { hw, pat })
                    }));
        }

        match dst_type {
            DstType::File if src_files.len() == 1 =>
                self.download_file(&src_files[0].2, &src_files[0].0.uri, dst)?,

            DstType::File =>
                Err(ErrorKind::MultipleSourcesOneDestination(dst.display().to_string()))?,

            DstType::DoesNotExist if src_files.len() == 1 && !ends_in_slash(dst) =>
                self.download_file(&src_files[0].2, &src_files[0].0.uri, dst)?,

            _ => {
                for (meta, whole_hw, rpat) in &src_files {
                    let mut file_dst = dst.to_owned();
                    if *whole_hw {
                        file_dst.push(meta.purpose.to_dir())
                    }
                    file_dst.push(&meta.name);
                    self.download_file(rpat, &meta.uri, &file_dst)?;
                }
            }
        }

        v2!("Done.");
        Ok(())
    }

    pub fn cp_up(&mut self, user: Option<&str>, raw_srcs: &[CpArg], dst: &RemotePattern)
                 -> Result<()> {

        let mut srcs = Vec::new();

        for src in raw_srcs {
            match src {
                CpArg::Local(filename) =>
                    srcs.push(filename),
                CpArg::Remote(rpat)    =>
                    Err(ErrorKind::CannotCopyRemoteToRemote(rpat.clone(), dst.clone()))?
            }
        }

        if dst.pat.is_empty() {
            for src in srcs {
                let filename     = match self.get_base_filename(&src) {
                    Ok(s)  => s.to_owned(),
                    Err(e) => {
                        ve1!("{}", e);
                        self.had_warning = true;
                        continue;
                    }
                };
                self.upload_file(user, src,
                                 &RemotePattern { hw: dst.hw, pat: filename })?;
            }
        } else {
            let src = if srcs.len() == 1 {
                &srcs[0]
            } else {
                Err(ErrorKind::MultipleSourcesOneDestination(dst.to_string()))?
            };

            let dsts = self.fetch_file_list(user, dst)?;
            let dst_filename = match dsts.len() {
                0 => dst.pat.to_owned(),
                1 => dsts[0].name.to_owned(),
                _ => Err(dest_pat_is_multiple(dst, &dsts))?,
            };

            self.upload_file(user, src, &RemotePattern { hw: dst.hw, pat: dst_filename })?;
        }

        v2!("Done.");
        Ok(())
    }

    fn upload_file(&mut self, user: Option<&str>, src: &Path, dst: &RemotePattern) -> Result<()> {
        let src_file     = std::fs::File::open(&src)?;
        let encoded_dst  = utf8_percent_encode(&dst.pat, PATH_SEGMENT_ENCODE_SET);
        let base_uri     = self.get_uri_for_submission_files(user, dst.hw)?;
        let uri          = format!{"{}/{}", base_uri, encoded_dst};
        let mut request  = self.http.put(&uri);
        request.body(src_file);
        v2!("Uploading ‘{}’ -> ‘{}’...", src.display(), dst);
        self.send_request(request)?;

        Ok(())
    }

    fn download_file(&mut self, src: &RemotePattern, rel_uri: &str, dst: &Path) -> Result<()> {
        if let Some(dir) = dst.parent() {
            std::fs::create_dir_all(dir)?;
        }

        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(dst)?;

        let uri          = format!("{}{}", self.config.get_endpoint(), rel_uri);
        let request      = self.http.get(&uri);
        ve2!("Downloading ‘{}’ -> ‘{}’...", src, dst.display());
        let mut response = self.send_request(request)?;
        response.copy_to(&mut file)?;

        Ok(())
    }

    fn get_base_filename<'a>(&mut self, path: &'a Path) -> Result<&'a str> {
        match path.file_name() {
            None         => Err(ErrorKind::BadLocalPath(path.to_owned()).into()),
            Some(os_str) => match os_str.to_str() {
                None         => Err(ErrorKind::FilenameNotUtf8(path.to_owned()).into()),
                Some(s)      => Ok(s),
            }
        }
    }

    pub fn deauth(&mut self) -> Result<()> {
        let mut cookie_lock = self.config.new_cookie()?;
        cookie_lock.deauth();
        v2!("Deauthenticated.");
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

        let user        = self.select_user(user_option);

        let mut cache   = replace(&mut self.submission_uris, HashMap::new());
        let uris        = match cache.entry(user.clone()) {
            hash_map::Entry::Occupied(entry) => entry.into_mut(),
            hash_map::Entry::Vacant(entry)   => entry.insert(self.get_submission_uris(&user)?),
        }.clone();
        replace(&mut self.submission_uris, cache);

        match uris.get(number) {
            Some(Some(uri)) => Ok(uri.to_owned()),
            _               => Err(ErrorKind::UnknownHomework(number).into()),
        }
    }

    fn get_uri_for_submission_files(&mut self, user: Option<&str>, number: usize)
        -> Result<String> {

        self.get_uri_for_submission(user, number).map(|uri| uri + "/files")
    }

    pub fn fetch_file_list(&mut self,
                           user: Option<&str>,
                           rpat: &RemotePattern)
        -> Result<Vec<messages::FileMeta>>
    {
        let matcher      = glob(&rpat.pat)?;
        let uri          = self.get_uri_for_submission_files(user, rpat.hw)?;
        let request      = self.http.get(&uri);
        let mut response = self.send_request(request)?;

        let files: Vec<messages::FileMeta> = response.json()?;

        Ok(files.into_iter()
               .filter(|file| matcher.is_match(&file.name))
               .collect())
    }

    pub fn ls(&mut self, user: Option<&str>, rpat: &RemotePattern) -> Result<()> {

        let files     = self.fetch_file_list(user, &rpat)?;
        let mut table = table::TextTable::new("%r  %l  [%l] %l\n");

        if files.is_empty() {
            return Err(ErrorKind::NoSuchRemoteFile(rpat.clone()).into());
        }

        for file in &files {
            table.add_row(
                table::Row::new()
                    .add_cell(file.byte_count)
                    .add_cell(&file.upload_time)
                    .add_cell(file.purpose.to_char())
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
        let in_evaluation   = submission.status.is_self_eval();
        let quota_remaining = submission.quota_remaining();

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
            .add_row(table::Row::new().add_cell("Quota remaining:")
                .add_cell(format!("{:.1}% ({}/{} bytes used)",
                                  quota_remaining,
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
        for rpat in pats {
            let files = self.fetch_file_list(user, &rpat)?;

            if files.is_empty() {
                let error = Error::from(ErrorKind::NoSuchRemoteFile(rpat.clone()));
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

        ve3!("> Sending request to {}", uri);
        let mut response = self.http.post(&uri)
            .basic_auth(username, Some(password))
            .send()?;
        let cookie_lock = self.config.new_cookie()?;
        self.handle_response(&mut response, cookie_lock)?;

        v2!("Created account: {}.", username);

        Ok(())
    }

    pub fn passwd(&mut self, user_option: Option<&str>) -> Result<()> {
        let user         = self.select_user(user_option);
        let password     = get_matching_passwords(&user)?;
        let message      = messages::PasswordChange { password };
        let uri          = format!("{}/api/users/{}", self.config.get_endpoint(), user);
        let mut request  = self.http.patch(&uri);
        request.json(&message);
        self.send_request(request)?;

        v2!("Changed password for user {}.", user);

        Ok(())
    }

    pub fn rm(&mut self, user: Option<&str>, pats: &[RemotePattern]) -> Result<()> {
        for rpat in pats {
            let files = self.fetch_file_list(user, &rpat)?;

            if files.is_empty() {
                let error = Error::from(ErrorKind::NoSuchRemoteFile(rpat.clone()));
                ve1!("{}", error);
                self.had_warning = true;
            }

            for file in files {
                let uri          = format!("{}{}", self.config.get_endpoint(), file.uri);
                let request      = self.http.delete(&uri);
                v2!("Deleting remote file ‘hw{}:{}’...", rpat.hw, file.name);
                self.send_request(request)?;
            }
        }

        v2!("Done.");
        Ok(())
    }

    pub fn whoami(&self) -> Result<()> {
        let username = self.config.get_username();

        if username.is_empty() {
            return Err(ErrorKind::LoginPlease.into())
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

    fn select_user(&self, username_option: Option<&str>) -> String {
        match username_option {
            Some(s) => s,
            None    => self.config.get_username(),
        }.to_owned()
    }

    fn send_request(&mut self, mut req_builder: reqwest::RequestBuilder)
        -> Result<reqwest::Response> {

        let cookie_lock = self.prepare_cookie(&mut req_builder)?;
        let request     = req_builder.build()?;
        ve3!("> Sending request to {}", request.url());
        let mut response = self.http.execute(request)?;
        self.handle_response(&mut response, cookie_lock)?;
        Ok(response)
    }

    fn handle_response(&mut self,
                       response: &mut reqwest::Response,
                       cookie_lock: DotfileLock)
        -> Result<()> {

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
        ve3!("> Sending cookie {}", cookie);
        request.header(cookie);
        Ok(cookie_lock)
    }

    fn save_cookie(&mut self,
                   response: &reqwest::Response,
                   mut cookie_lock: DotfileLock)
        -> Result<()> {

        if let Some(reqwest::header::SetCookie(chunks)) = response.headers().get() {
            if let Some((key, value)) = parse_cookies(&chunks) {
                ve3!("< Received cookie {}={}", key, value);
                cookie_lock.set_cookie(key, value);
            }
        }

        Ok(())
    }
}

fn ends_in_slash(path: &Path) -> bool {
    match path.to_str() {
        Some(s) => s.ends_with('/'),
        None    => false,
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

impl std::fmt::Display for RemotePattern {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "hw{}:{}", self.hw, self.pat)
    }
}

impl std::fmt::Display for CpArg {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CpArg::Local(filename) => write!(f, ":{}", filename.display()),
            CpArg::Remote(rp)      => write!(f, "{}", rp),
        }
    }
}

