#![recursion_limit = "256"]

use percent_encoding as enc;

use reqwest::blocking;

use std::cell::{Cell, RefCell};
use std::collections::{hash_map, HashMap};
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::ops::Deref;
use std::path::Path;
use std::process::Command;

pub mod config;
pub mod credentials;
pub mod errors;
pub mod messages;

mod args;
mod cmd;
mod util;

const API_KEY_COOKIE: &str = "gsc_api_key";

pub mod prelude {
    pub use thousands::Separable;
    pub use vlog::*;

    pub use crate::{
        args::{
            traits::{Qualified, RemotePath, Unqualified},
            types::{CpArg, HwOptQual, HwQual, RemoteDestination, RemotePattern},
        },
        errors::{Error, ErrorKind, JsonStatus, RemoteFiles, ResultExt},
        GscClient,
    };

    pub type Result<T, E = Error> = std::result::Result<T, E>;
}

pub use prelude::*;

use self::credentials::*;
use self::util::{hanging, Percentage};
use crate::errors::ApiKeyExplanation;
use std::cmp::Ordering;

pub struct GscClient {
    http:            blocking::Client,
    config:          config::Config,
    submission_uris: RefCell<HashMap<String, Vec<Option<String>>>>,
    had_warning:     Cell<bool>,
}

impl GscClient {
    pub fn new() -> Result<Self> {
        let mut config = config::Config::new();
        config.load_dotfile()?;

        Ok(GscClient {
            http: blocking::Client::new(),
            config,
            submission_uris: RefCell::new(HashMap::new()),
            had_warning: Cell::new(false),
        })
    }

    pub fn config(&self) -> &config::Config {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut config::Config {
        &mut self.config
    }

    pub fn had_warning(&self) -> bool {
        self.had_warning.get()
    }

    pub fn admin_csv(&self) -> Result<()> {
        let uri = format!("{}/api/grades.csv", self.config.get_endpoint());
        let request = self.http.get(&uri);
        let mut response = self.send_request(request)?;
        response.copy_to(&mut io::stdout())?;
        Ok(())
    }

    pub fn admin_divorce(&self, username: &str, hw: usize) -> Result<()> {
        let message = messages::SubmissionChange {
            owner2: Some(()),
            ..Default::default()
        };

        let creds = self.load_credentials()?;
        let uri = self.get_uri_for_submission(username, hw, &creds)?;
        let request = self.http.patch(&uri).json(&message);
        self.send_request(request)?;

        v1!("Okay");
        Ok(())
    }

    pub fn admin_add_user(&self, name: &str, role: messages::UserRole) -> Result<()> {
        let uri = format!("{}/api/users", self.config.get_endpoint());
        let message = messages::UserCreate { name, role };
        let request = self.http.post(&uri).json(&message);
        v2!("Creating user {} with role {}...", name, role);
        let response = self.send_request(request)?;

        if self.config.json_output() {
            v1!("{}", response.text()?);
        } else {
            let result: messages::UserShort = response.json()?;
            v1!("Created user {}.", result.name);
        }

        Ok(())
    }

    pub fn admin_del_user(&self, name: &str) -> Result<()> {
        let uri = self.user_uri(name);
        let request = self.http.delete(&uri);
        v2!("Deleting user {}...", name);
        self.send_request(request)?;
        Ok(())
    }

    pub fn admin_extend(
        &self,
        username: &str,
        hw: usize,
        datetime: &str,
        eval: bool,
    ) -> Result<()> {
        let mut message = messages::SubmissionChange::default();
        if eval {
            message.eval_date = Some(datetime.parse()?);
        } else {
            message.due_date = Some(datetime.parse()?);
        }

        let creds = self.load_credentials()?;
        let uri = self.get_uri_for_submission(username, hw, &creds)?;
        let request = self.http.patch(&uri).json(&message);
        let response = self.send_request(request)?;
        let submission: messages::Submission = response.json()?;

        if eval {
            v2!("Set eval date set to {}", submission.eval_date);
        } else {
            v2!("Set due date to {}", submission.due_date);
        }

        Ok(())
    }

    pub fn admin_permalink(&self, username: &str, hw: usize, number: usize) -> Result<()> {
        let creds = self.load_credentials()?;
        let uri = self.get_uri_for_submission(username, hw, &creds)?;
        let request = self.http.get(&uri);
        let submission: messages::Submission = self.send_request(request)?.json()?;

        let uri = format!(
            "{}{}/{}/self",
            self.config.get_endpoint(),
            submission.evals_uri,
            number
        );
        let request = self.http.get(&uri);
        let self_eval: messages::SelfEval = self.send_request(request)?.json()?;

        v1!("{}", self_eval.permalink);

        Ok(())
    }

    pub fn admin_partners(&self, username: &str, hw: usize) -> Result<()> {
        let creds = self.load_credentials()?;
        let uri = self.get_uri_for_submission(username, hw, &creds)?;
        let request = self.http.get(&uri);
        let response = self.send_request(request)?;
        let submission: messages::Submission = response.json()?;

        let mut buf = submission.owner1.name.clone();
        if let Some(owner2) = &submission.owner2 {
            buf.push(' ');
            buf += &owner2.name;
        }

        v1!("{}", buf);

        Ok(())
    }

    fn get_evals(&self, username: &str, hw: usize) -> Result<Vec<messages::EvalShort>> {
        let creds = self.load_credentials()?;
        let uri = self.get_uri_for_submission(username, hw, &creds)?;
        let request = self.http.get(&uri);
        let response = self.send_request(request)?;
        let submission: messages::Submission = response.json()?;

        let uri = format!("{}{}", self.config.get_endpoint(), submission.evals_uri);
        let request = self.http.get(&uri);
        let response = self.send_request(request)?;
        Ok(response.json()?)
    }

    fn set_grade(
        &self,
        username: &str,
        hw: usize,
        eval: &messages::EvalShort,
        score: f64,
        comment: &str,
    ) -> Result<()> {
        let uri = format!("{}{}/grader", self.config.get_endpoint(), eval.uri);
        let mut request = self.http.put(&uri);
        let message = messages::GraderEval {
            uri,
            grader: "root".to_owned(),
            score,
            explanation: comment.to_owned(),
            status: messages::GraderEvalStatus::Ready,
        };
        request = request.json(&message);
        let response = self.send_request(request)?;
        let result: messages::GraderEval = response.json()?;

        v2!(
            "Set user {}’s hw{}, item {} to {}",
            username,
            hw,
            eval.sequence,
            result.score
        );
        Ok(())
    }

    pub fn admin_set_grade(
        &self,
        username: &str,
        hw: usize,
        number: usize,
        score: f64,
        comment: &str,
    ) -> Result<()> {
        let eval = self
            .get_evals(username, hw)?
            .into_iter()
            .find(|eval| eval.sequence == number)
            .ok_or(ErrorKind::EvalItemDoesNotExist(hw, number))?;
        self.set_grade(username, hw, &eval, score, comment)
    }

    pub fn admin_set_auto(
        &self,
        username: &str,
        hw: usize,
        score: f64,
        comment: &str,
    ) -> Result<()> {
        let eval = self
            .get_evals(username, hw)?
            .into_iter()
            .filter(|eval| eval.eval_type == messages::EvalType::Informational)
            .last()
            .chain_err(|| ErrorKind::NoInformationalEvalItem)?;
        self.set_grade(username, hw, &eval, score, comment)
    }

    pub fn admin_set_exam(
        &self,
        username: &str,
        number: usize,
        points: usize,
        possible: usize,
    ) -> Result<()> {
        let uri = self.user_uri(username);
        let message = messages::UserChange {
            exam_grades: vec![messages::ExamGrade {
                number,
                points,
                possible,
            }],
            ..Default::default()
        };

        let request = self.http.patch(&uri).json(&message);
        let response = self.send_request(request)?;
        self.print_results(response)
    }

    pub fn admin_submissions(&self, hw: usize) -> Result<()> {
        let uri = format!("{}/api/submissions/hw{}", self.config.get_endpoint(), hw);
        let request = self.http.get(&uri);
        let result = self.send_request(request)?;
        let submissions: Vec<messages::SubmissionShort> = result.json()?;

        let mut table = tabular::Table::new(" {:>}  {:<}  {:<}");

        for submission in &submissions {
            table.add_row(
                tabular::Row::new()
                    .with_cell(submission.id)
                    .with_cell(&submission.owner1.name)
                    .with_cell(
                        submission
                            .owner2
                            .as_ref()
                            .map(|o| o.name.as_str())
                            .unwrap_or(""),
                    ),
            );
        }

        v1!("{}", table);

        Ok(())
    }

    pub fn auth(&mut self, username: &str) -> Result<()> {
        let username = &username.to_lowercase();
        let uri = self.user_uri(username);

        loop {
            let api_key = prompt_secret("Enter API key", username)?;
            let api_key = check_api_key(&api_key, self.config())?;

            let creds = Credentials::new(username, API_KEY_COOKIE, api_key);
            ve3!("> Sending request to {}", uri);
            let response = self
                .http
                .get(&uri)
                .header(reqwest::header::COOKIE, creds.to_header()?)
                .send()?;

            match self.handle_response(response) {
                Ok(_) => {
                    v2!("Authenticated as {}", username);
                    self.save_credentials(&creds)?;
                    return Ok(());
                }
                Err(e @ Error(ErrorKind::ServerError(JsonStatus { status: 401, .. }), _)) => {
                    eprintln!("{}", e)
                }
                Err(e) => return Err(e),
            }
        }
    }

    pub fn cp(&self, srcs: &[CpArg], dst: &CpArg) -> Result<()> {
        match dst {
            CpArg::Local(filename) => self.cp_dn(srcs, filename),
            CpArg::Remote(rpat) => self.cp_up(srcs, rpat),
        }
    }

    fn cp_dn(&self, raw_srcs: &[CpArg], dst: &Path) -> Result<()> {
        let mut src_rpats = Vec::new();

        for src in raw_srcs {
            match src {
                CpArg::Local(filename) => {
                    return Err(ErrorKind::cannot_copy_local_to_local(filename, dst).into());
                }
                CpArg::Remote(rpat) => src_rpats.push(rpat),
            }
        }

        enum DstType {
            Dir,
            File,
            DoesNotExist,
        }

        let dst_type = match dst.metadata() {
            Err(e) => match e.kind() {
                io::ErrorKind::NotFound => DstType::DoesNotExist,
                _ => return Err(e.into()),
            },

            Ok(metadata) => match metadata.is_dir() {
                true => DstType::Dir,
                false => DstType::File,
            },
        };

        let policy = &mut self.config.get_overwrite_policy();

        match dst_type {
            DstType::File => {
                if src_rpats.len() != 1 {
                    return Err(ErrorKind::MultipleSourcesOneDestination.into());
                }

                let src_rpat = src_rpats[0];

                if src_rpat.is_whole_hw() {
                    return Err(ErrorKind::SourceHwToDestinationFile(
                        src_rpat.hw,
                        dst.to_owned(),
                    ).into());
                } else {
                    let src_file = self.fetch_one_matching_filename(src_rpat)?;
                    if policy.confirm_overwrite(|| dst.display())? {
                        self.download_file(src_rpat.hw, &src_file, dst)?;
                    }
                }
            }

            DstType::DoesNotExist => {
                if src_rpats.len() != 1 {
                    return Err(ErrorKind::MultipleSourcesOneDestination.into());
                }

                let src_rpat = src_rpats[0];

                if src_rpat.is_whole_hw() {
                    soft_create_dir(dst)?;
                    self.download_hw(policy, src_rpat.hw, dst)?;
                } else {
                    let src_file = self.fetch_one_matching_filename(src_rpat)?;
                    self.download_file(src_rpat.hw, &src_file, dst)?;
                }
            }

            DstType::Dir => {
                for src_rpat in src_rpats {
                    self.try_warn(|| {
                        if src_rpat.is_whole_hw() {
                            self.download_hw(policy, src_rpat.hw, dst)?;
                        } else {
                            let src_metas = self.fetch_nonempty_matching_file_list(src_rpat)?;

                            for src_meta in src_metas {
                                let mut file_dst = dst.to_owned();
                                file_dst.push(&src_meta.name);
                                if self.is_okay_to_write_local(policy, &file_dst)? {
                                    self.download_file(src_rpat.hw, &src_meta, &file_dst)?;
                                }
                            }
                        }

                        Ok(())
                    });
                }
            }
        }

        v2!("Done.");
        Ok(())
    }

    fn download_file(&self, hw: usize, meta: &messages::FileMeta, dst: &Path) -> Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(dst)?;

        let uri = format!("{}{}", self.config.get_endpoint(), meta.uri);
        let request = self.http.get(&uri);
        ve2!(
            "Downloading ‘hw{}:{}’ -> ‘{}’...",
            hw,
            meta.name,
            dst.display()
        );
        let mut response = self.send_request(request)?;
        response.copy_to(&mut file)?;

        if cfg!(unix) {
            let mtime = &meta.upload_time;
            ve2!(
                "Setting modification time of ‘{}’ to {}",
                dst.display(),
                mtime.touch_t_fmt()
            );
            set_file_mtime(dst, mtime)?;
        }

        Ok(())
    }

    fn download_hw(
        &self,
        policy: &mut config::OverwritePolicy,
        hw: usize,
        dst: &Path,
    ) -> Result<()> {
        let rpat = HwQual::just_hw(hw);
        let src_metas = self.fetch_matching_file_list(&rpat)?;

        for src_meta in src_metas {
            if src_meta.purpose == messages::FilePurpose::Log {
                continue;
            }

            let mut file_dst = dst.to_owned();
            file_dst.push(src_meta.purpose.to_dir());
            soft_create_dir(&file_dst)?;
            file_dst.push(&src_meta.name);
            if self.is_okay_to_write_local(policy, &file_dst)? {
                self.download_file(hw, &src_meta, &file_dst)?;
            }
        }

        Ok(())
    }

    fn cp_up(&self, raw_srcs: &[CpArg], dst: &RemotePattern) -> Result<()> {
        let mut srcs = Vec::new();

        for src in raw_srcs {
            match src {
                CpArg::Local(filename) =>
                    srcs.push(filename),
                CpArg::Remote(rpat) =>
                    return Err(
                        ErrorKind::CannotCopyRemoteToRemote(
                            rpat.clone(),
                            dst.clone(),
                        ).into()),
            }
        }

        if dst.is_whole_hw() {
            for src in srcs {
                let filename = match self.get_base_filename(src) {
                    Ok(s) => s,
                    Err(e) => {
                        self.warn(e);
                        continue;
                    }
                };
                self.upload_file(src, &dst.with_name(filename))?;
            }
        } else {
            let src = if srcs.len() == 1 {
                &srcs[0]
            } else {
                return Err(ErrorKind::MultipleSourcesOneDestination.into());
            };

            let dsts = self.fetch_matching_file_list(dst)?;
            let filename = match dsts.len() {
                0 => &dst.name,
                1 => &dsts[0].name,
                _ => return Err(Error::dest_pat_is_multiple(dst, &dsts)),
            };

            self.upload_file(src, &dst.with_name(filename))?;
        }

        v2!("Done.");
        Ok(())
    }

    fn upload_file(&self, src: &Path, dst: &RemotePattern) -> Result<()> {
        let src_file = fs::File::open(&src)?;
        let encoded_dst = enc::utf8_percent_encode(&dst.name, ENCODE_SET);
        let base_uri = self.get_uri_for_submission_files(dst.hw)?;
        let uri = format! {"{}/{}", base_uri, encoded_dst};
        let request = self.http.put(&uri).body(src_file);
        v2!("Uploading ‘{}’ -> ‘{}’...", src.display(), dst);
        self.send_request(request)?;

        Ok(())
    }

    fn get_base_filename<'a>(&self, path: &'a Path) -> Result<&'a str> {
        match path.file_name() {
            None => Err(ErrorKind::BadLocalPath(path.to_owned()).into()),
            Some(os_str) => match os_str.to_str() {
                None => Err(ErrorKind::FilenameNotUtf8(path.to_owned()).into()),
                Some(s) => Ok(s),
            },
        }
    }

    fn is_okay_to_write_remote<T>(
        &self,
        policy: &mut config::OverwritePolicy,
        dst: &HwQual<T>,
    ) -> Result<bool>
    where
        T: Deref<Target = str>,
    {
        if let Ok(dst_meta) = self.fetch_exact_file_name(dst.hw, &dst.name) {
            policy.confirm_overwrite(|| dst_meta)
        } else {
            Ok(true)
        }
    }

    fn is_okay_to_write_local(
        &self,
        policy: &mut config::OverwritePolicy,
        dst: &Path,
    ) -> Result<bool> {
        if dst.exists() {
            policy.confirm_overwrite(|| dst.display())
        } else {
            Ok(true)
        }
    }

    pub fn deauth(&self) -> Result<()> {
        let uri = format!("{}/api/whoami", self.config.get_endpoint());
        let request = self.http.delete(&uri);
        let result = match self.send_request(request) {
            Ok(response) => {
                let result: reqwest::Result<errors::JsonStatus> = response.json();
                match result {
                    Ok(e) => {
                        if e.status == 200 {
                            Ok("Deauthenticated with server.")
                        } else {
                            Err(String::from("Could not deauthenticate with server."))
                        }
                    }
                    Err(e) => Err(format!("Could not understand JSON from server:\n  {}", e)),
                }
            }

            Err(e) => match e.kind() {
                ErrorKind::LoginPlease => Ok("You aren’t authenticated."),
                _ => Err(format!("Could not deauthenticate with server:\n  {}", e)),
            },
        };

        match result {
            Ok(msg) => v2!("{}", msg),
            Err(msg) => self.warn(format!("{}\nDeleting local credentials anyway.", msg)),
        }

        self.clear_credentials()?;

        Ok(())
    }

    pub fn cat(&self, rpats: &[RemotePattern]) -> Result<()> {
        for rpat in rpats {
            self.try_warn(|| {
                let files = self.fetch_nonempty_matching_file_list(rpat)?;

                if rpat.is_whole_hw() {
                    let mut table = tabular::Table::new("{:>}  {:<}");
                    let mut line_no = 0;

                    for file in files {
                        if !file.purpose.is_line_numbered() {
                            continue;
                        }

                        let uri = format!("{}{}", self.config.get_endpoint(), file.uri);
                        let request = self.http.get(&uri);
                        let response = self.send_request(request)?;
                        let contents = BufReader::new(response);

                        let head = format!("hw{}:{}", rpat.hw, file.name);
                        let rule: String = "=".repeat(head.len());

                        table.add_heading(head);
                        table.add_heading(rule);
                        table.add_heading(String::new());

                        for line_result in contents.lines() {
                            line_no += 1;
                            let line = line_result.unwrap_or_else(|e| format!("<error: {}>", e));
                            table.add_row(
                                tabular::Row::new()
                                    .with_cell(line_no)
                                    .with_cell(line.trim_end()),
                            );
                        }

                        table.add_heading(String::new());
                    }

                    print!("{}", table);
                } else {
                    for file in files {
                        let uri = format!("{}{}", self.config.get_endpoint(), file.uri);
                        let request = self.http.get(&uri);
                        let mut response = self.send_request(request)?;
                        response.copy_to(&mut io::stdout())?;
                    }
                }

                Ok(())
            })
        }

        Ok(())
    }

    pub fn get_eval(&self, hw: usize, number: usize) -> Result<()> {
        let (who, creds) = self.load_effective_credentials()?;
        let uri = self.get_uri_for_submission(&who, hw, &creds)?;
        let request = self.http.get(&uri);
        let response = self.send_request(request)?;
        let submission: messages::Submission = response.json()?;

        let uri = format!(
            "{}{}/{}",
            self.config.get_endpoint(),
            submission.evals_uri,
            number
        );
        let request = self.http.get(&uri);
        let response = self.send_request(request)?;
        let eval: messages::Eval = response.json()?;

        v1!(
            "Homework {} item {} ({:?}, {})",
            hw,
            number,
            eval.eval_type,
            Percentage(eval.value)
        );
        v1!("{}", hanging(&eval.prompt));

        if let Some(ref self_eval) = eval.self_eval {
            v1!("Self evaluation:   {}", Percentage(self_eval.score));
            v1!("{}", hanging(&self_eval.explanation));
        }

        if let Some(ref grader_eval) = eval.grader_eval {
            v1!("Grader evaluation: {}", Percentage(grader_eval.score));
            v1!("{}", hanging(&grader_eval.explanation));
        }

        Ok(())
    }

    pub fn set_eval(&self, hw: usize, number: usize, score: f64, explanation: &str) -> Result<()> {
        let (who, creds) = self.load_effective_credentials()?;
        let uri = self.get_uri_for_submission(&who, hw, &creds)?;
        let request = self.http.get(&uri);
        let response = self.send_request(request)?;
        let submission: messages::Submission = response.json()?;

        let uri = format!(
            "{}{}/{}/self",
            self.config.get_endpoint(),
            submission.evals_uri,
            number
        );
        let mut request = self.http.put(&uri);
        let message = messages::SelfEval {
            uri,
            score,
            explanation: explanation.to_owned(),
            permalink: String::new(),
        };
        request = request.json(&message);
        let response = self.send_request(request)?;
        let result: messages::SelfEval = response.json()?;

        v2!(
            "Set hw{} item {} self eval to {}",
            hw,
            number,
            Percentage(result.score)
        );

        Ok(())
    }

    pub fn partner(&self) -> Result<()> {
        let (who, creds) = self.load_effective_credentials()?;
        let uri = self.user_uri(&who);
        let request = self.http.get(&uri);
        let response = self.send_request_with_credentials(request, &creds)?;
        let user: messages::User = response.json()?;
        self.print_partner_status(&user, "");
        Ok(())
    }

    pub fn partner_request(&self, hw: usize, them: &str) -> Result<()> {
        self.partner_operation(messages::PartnerRequestStatus::Outgoing, hw, them)
    }

    pub fn partner_accept(&self, hw: usize, them: &str) -> Result<()> {
        self.partner_operation(messages::PartnerRequestStatus::Accepted, hw, them)
    }

    pub fn partner_cancel(&self, hw: usize, them: &str) -> Result<()> {
        self.partner_operation(messages::PartnerRequestStatus::Canceled, hw, them)
    }

    fn partner_operation(
        &self,
        op: messages::PartnerRequestStatus,
        hw: usize,
        them: &str,
    ) -> Result<()> {
        let (who, creds) = self.load_effective_credentials()?;
        let uri = self.user_uri(&who);
        let message = messages::UserChange {
            partner_requests: vec![messages::PartnerRequest {
                assignment_number: hw,
                user:              them.to_owned(),
                status:            op,
            }],
            ..Default::default()
        };

        let request = self.http.patch(&uri).json(&message);
        let response = self.send_request_with_credentials(request, &creds)?;
        self.print_results(response)
    }

    pub fn rm(&self, pats: &[RemotePattern]) -> Result<()> {
        for rpat in pats {
            self.try_warn(|| {
                let files = self.fetch_nonempty_matching_file_list(rpat)?;

                for file in files {
                    let uri = format!("{}{}", self.config.get_endpoint(), file.uri);
                    let request = self.http.delete(&uri);
                    v2!("Deleting remote file ‘hw{}:{}’...", rpat.hw, file.name);
                    self.send_request(request)?;
                }

                Ok(())
            });
        }

        v2!("Done.");
        Ok(())
    }

    pub fn status_hw(&self, number: usize) -> Result<()> {
        let (who, creds) = self.load_effective_credentials()?;
        let uri = self.get_uri_for_submission(&who, number, &creds)?;
        let request = self.http.get(&uri);
        let response = self.send_request(request)?;

        let submission: messages::Submission = response.json()?;
        let in_evaluation = submission.status.is_self_eval();
        let quota_remaining = submission.quota_remaining();

        let mut table = tabular::Table::new("  {:<}  {:<}");
        table.add_row(
            tabular::Row::new()
                .with_cell("Submission status:")
                .with_cell(submission.status),
        );

        if in_evaluation {
            table.add_row(
                tabular::Row::new()
                    .with_cell("Evaluation status:")
                    .with_cell(submission.eval_status),
            );
        }

        table
            .add_row(
                tabular::Row::new()
                    .with_cell("Open date:")
                    .with_cell(submission.open_date),
            )
            .add_row(
                tabular::Row::new()
                    .with_cell("Submission due date:")
                    .with_cell(submission.due_date),
            )
            .add_row(
                tabular::Row::new()
                    .with_cell("Self-eval due date:")
                    .with_cell(submission.eval_date),
            )
            .add_row(
                tabular::Row::new()
                    .with_cell("Last modified:")
                    .with_cell(submission.last_modified),
            )
            .add_row(
                tabular::Row::new()
                    .with_cell("Quota remaining:")
                    .with_cell(format!(
                        "{:.1}% ({} of {} bytes used)",
                        quota_remaining,
                        submission.bytes_used.separate_with_commas(),
                        submission.bytes_quota.separate_with_commas()
                    )),
            );

        let mut owners = submission.owner1.name.clone();
        if let Some(owner2) = &submission.owner2 {
            owners += " and ";
            owners += &owner2.name;
        }

        v1!("hw{} ({})", number, owners);
        v1!("{}", table);

        Ok(())
    }

    pub fn status_user(&self) -> Result<()> {
        let (who, creds) = self.load_effective_credentials()?;
        let uri = self.user_uri(&who);
        let request = self.http.get(&uri);
        let response = self.send_request_with_credentials(request, &creds)?;

        let user: messages::User = response.json()?;

        v1!("Status for {}:\n", user.name);

        if user
            .submissions
            .iter()
            .any(|s| s.status != messages::SubmissionStatus::Future)
        {
            let mut table = tabular::Table::new("    hw{:<}: {:>}    {:<}");

            for s in &user.submissions {
                let grade = match s.status {
                    messages::SubmissionStatus::Future => continue,
                    messages::SubmissionStatus::Closed => format!("{:.1}%", 100.0 * s.grade),
                    _ => String::new(),
                };

                table.add_row(
                    tabular::Row::new()
                        .with_cell(s.assignment_number)
                        .with_cell(grade)
                        .with_cell(s.status),
                );
            }

            v1!("  Submissions:\n{}", table);
        }

        if !user.exam_grades.is_empty() {
            let mut table = tabular::Table::new("    ex{:<}: {:>}%    ({:<} / {:<})");

            for e in &user.exam_grades {
                let grade = format!("{:.1}", 100.0 * e.points as f64 / e.possible as f64);
                table.add_row(
                    tabular::Row::new()
                        .with_cell(e.number)
                        .with_cell(grade)
                        .with_cell(e.points)
                        .with_cell(e.possible),
                );
            }

            v1!("  Exam grades:\n{}", table);
        }

        if !user.partner_requests.is_empty() {
            self.print_partner_status(&user, "  ");
            v1!("Partner requests can be managed with the ‘gsc partner’ command.");
        }

        Ok(())
    }

    pub fn whoami(&self) -> Result<()> {
        let uri = format!("{}/api/whoami", self.config.get_endpoint());
        let request = self.http.get(&uri);
        let response = self.send_request(request)?;
        let text = response.text()?;
        v1!("{}", text);
        Ok(())
    }

    // Helper methods

    fn fetch_raw_file_list(&self, hw: usize) -> Result<blocking::Response> {
        let uri = self.get_uri_for_submission_files(hw)?;
        let request = self.http.get(&uri);
        self.send_request(request)
    }

    fn fetch_exact_file_name(&self, hw: usize, name: &str) -> Result<messages::FileMeta> {
        let response = self.fetch_raw_file_list(hw)?;

        let files: Vec<messages::FileMeta> = response.json()?;

        files
            .into_iter()
            .find(|file| file.name == name)
            .ok_or_else(|| ErrorKind::NoSuchRemoteFile(RemotePattern::hw_name(hw, name)).into())
    }

    fn fetch_matching_file_list(&self, rpat: &RemotePattern) -> Result<Vec<messages::FileMeta>> {
        let matcher = glob(&rpat.name)?;
        let response = self.fetch_raw_file_list(rpat.hw)?;

        let files: Vec<messages::FileMeta> = response.json()?;

        Ok(files
            .into_iter()
            .filter(|file| matcher.is_match(&file.name))
            .collect())
    }

    fn fetch_nonempty_matching_file_list(
        &self,
        rpat: &RemotePattern,
    ) -> Result<Vec<messages::FileMeta>> {
        let result = self.fetch_matching_file_list(rpat)?;

        if result.is_empty() {
            Err(ErrorKind::NoSuchRemoteFile(rpat.clone()).into())
        } else {
            Ok(result)
        }
    }

    fn fetch_one_matching_filename(&self, rpat: &RemotePattern) -> Result<messages::FileMeta> {
        let mut files = self.fetch_matching_file_list(rpat)?;

        match files.len() {
            0 => Err(ErrorKind::NoSuchRemoteFile(rpat.clone()).into()),
            1 => Ok(files.pop().unwrap()),
            _ => Err(ErrorKind::MultipleSourcesOneDestination.into()),
        }
    }

    fn fetch_submissions(
        &self,
        user: &str,
        creds: &Credentials,
    ) -> Result<Vec<messages::SubmissionShort>> {
        let uri = self.user_uri(user) + "/submissions";
        let request = self.http.get(&uri);
        let response = self.send_request_with_credentials(request, creds)?;
        response
            .json()
            .chain_err(|| "Could not understand response from server")
    }

    fn get_submission_uris(&self, user: &str, creds: &Credentials) -> Result<Vec<Option<String>>> {
        let submissions = self.fetch_submissions(user, creds)?;
        let mut result = Vec::new();

        for submission in &submissions {
            let number = submission.assignment_number;

            while number >= result.len() {
                result.push(None);
            }

            result[number] = Some(format!("{}{}", self.config.get_endpoint(), submission.uri));
        }

        Ok(result)
    }

    fn get_uri_for_submission(
        &self,
        user: &str,
        number: usize,
        creds: &Credentials,
    ) -> Result<String> {
        let mut cache = self.submission_uris.borrow_mut();
        let uris = match cache.entry(user.to_owned()) {
            hash_map::Entry::Occupied(entry) => entry.into_mut(),
            hash_map::Entry::Vacant(entry) => {
                entry.insert(self.get_submission_uris(user, creds)?)
            }
        };

        match uris.get(number) {
            Some(Some(uri)) => Ok(uri.to_owned()),
            _ => Err(ErrorKind::UnknownHomework(number).into()),
        }
    }

    fn get_uri_for_submission_files(&self, number: usize) -> Result<String> {
        let (who, creds) = self.load_effective_credentials()?;
        self.get_uri_for_submission(&who, number, &creds)
            .map(|uri| uri + "/files")
    }

    fn handle_response(&self, response: blocking::Response) -> Result<blocking::Response> {
        if response.status().is_success() {
            Ok(response)
        } else {
            let error = response.json()?;
            Err(ErrorKind::ServerError(error).into())
        }
    }

    fn load_credentials(&self) -> Result<Credentials> {
        Credentials::read(self.config.get_credentials_file()?)
    }

    fn load_effective_credentials(&self) -> Result<(String, Credentials)> {
        let creds = self.load_credentials()?;
        let user = self
            .config
            .get_on_behalf()
            .unwrap_or_else(|| creds.username());
        Ok((user.to_owned(), creds))
    }

    fn save_credentials(&self, creds: &Credentials) -> Result<()> {
        creds.write(self.config.get_credentials_file()?)
    }

    fn clear_credentials(&self) -> Result<()> {
        fs::remove_file(self.config.get_credentials_file()?)?;
        Ok(())
    }

    fn add_credentials(
        &self,
        mut request: blocking::RequestBuilder,
        creds: &Credentials,
    ) -> Result<blocking::RequestBuilder> {
        let cookie = creds.to_header()?;
        ve3!("> Sending cookie {}", cookie.to_str().unwrap());
        request = request.header(reqwest::header::COOKIE, cookie);
        Ok(request)
    }

    fn print_partner_status(&self, user: &messages::User, indent: &str) {
        if user.partner_requests.is_empty() {
            ve1!("No outstanding partner requests.");
        } else {
            let mut table = tabular::Table::new("    {:<} {:<}");

            for p in &user.partner_requests {
                use self::messages::PartnerRequestStatus::*;
                let hw = format!("hw{}:", p.assignment_number);
                let message = match p.status {
                    Outgoing => format!("sent to {}", p.user),
                    Incoming => format!("received from {}", p.user),
                    _ => continue,
                };

                table.add_row(tabular::Row::new().with_cell(hw).with_cell(message));
            }

            v1!("{}Partner requests:\n{}", indent, table);
        }
    }

    fn print_results(&self, response: blocking::Response) -> Result<()> {
        let results: Vec<messages::JsonResult> = response.json()?;
        self.print_results_helper(&results);
        Ok(())
    }

    fn print_results_helper(&self, results: &[messages::JsonResult]) {
        for result in results {
            match result {
                messages::JsonResult::Success(msg) => v2!("{}", msg),
                messages::JsonResult::Failure(msg) => self.warn(msg),
                messages::JsonResult::Nested(vec) => self.print_results_helper(vec),
            }
        }
    }

    fn user_uri(&self, user: &str) -> String {
        format!("{}/api/users/{}", self.config.get_endpoint(), user)
    }

    fn send_request(&self, req_builder: blocking::RequestBuilder) -> Result<blocking::Response> {
        let creds = self.load_credentials()?;
        self.send_request_with_credentials(req_builder, &creds)
    }

    fn send_request_with_credentials(
        &self,
        mut req_builder: blocking::RequestBuilder,
        creds: &Credentials,
    ) -> Result<blocking::Response> {
        req_builder = self.add_credentials(req_builder, creds)?;
        let request = req_builder.build()?;
        ve3!("> Sending request to {}", request.url());
        let response = self.http.execute(request)?;
        self.handle_response(response)
    }

    fn try_warn<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> Result<R>,
        R: Default,
    {
        f().unwrap_or_else(|error| {
            self.warn(error);
            R::default()
        })
    }

    fn warn<T: std::fmt::Display>(&self, msg: T) {
        ve1!("{}", msg);
        self.had_warning.set(true);
    }
}

impl messages::FilePurpose {
    fn is_automatically_deletable(self) -> bool {
        self == messages::FilePurpose::Log
    }

    fn is_line_numbered(self) -> bool {
        self != messages::FilePurpose::Resource && !self.is_automatically_deletable()
    }
}

const ENCODE_SET: &enc::AsciiSet = &enc::CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'<')
    .add(b'>')
    .add(b'`')
    .add(b'?')
    .add(b'{')
    .add(b'}')
    .add(b'%')
    .add(b'/')
    .add(b'+');

fn glob(pattern: &str) -> Result<globset::GlobMatcher> {
    let real_pattern = if pattern.is_empty() { "*" } else { pattern };
    Ok(globset::Glob::new(real_pattern)?.compile_matcher())
}

fn prompt_secret(prompt: &str, username: &str) -> Result<String> {
    let prompt = format!("{} for {}: ", prompt, username);
    let secret = rpassword::prompt_password_stderr(&prompt)?;
    Ok(secret)
}

fn check_api_key(api_key: &str, config: &config::Config) -> Result<String> {
    const KEY_LEN: usize = 40;

    let mut reasons = if config.get_verbosity() > 1 {
        ApiKeyExplanation::with_key(api_key)
    } else {
        ApiKeyExplanation::new()
    };

    if api_key.is_empty() {
        return reasons.final_straw("It’s empty!");
    }

    let api_key = api_key.trim_matches(|c: char| c.is_ascii_whitespace());

    let len = api_key.len();

    if len == 0 {
        return reasons.final_straw("It’s nothing but whitespace.");
    }

    match len.cmp(&KEY_LEN) {
        Ordering::Equal => {}
        Ordering::Less => reasons.add(format!(
            "It’s only {} characters, but I expected {}.",
            len, KEY_LEN
        )),
        Ordering::Greater => reasons.add(format!(
            "It’s {} characters, but I expected only {}.",
            len, KEY_LEN
        )),
    }

    let mut result = String::new();

    for c in api_key.chars() {
        if !c.is_ascii_hexdigit() {
            reasons.add(format!("It contains non-hexdigit characters like {:?}.", c));
            break;
        }

        result.push(c.to_ascii_lowercase());
    }

    reasons.into_result()?;

    Ok(result)
}

fn soft_create_dir(path: &Path) -> Result<()> {
    match fs::create_dir(path) {
        Ok(_) => Ok(()),
        Err(e) => match e.kind() {
            io::ErrorKind::AlreadyExists => Ok(()),
            _ => Err(e.into()),
        },
    }
}

fn set_file_mtime(dst: &Path, mtime: &messages::UtcDateTime) -> Result<()> {
    let mtime = mtime.touch_t_fmt().to_string();
    let output = Command::new("touch")
        .arg("-m")
        .arg("-t")
        .arg(mtime)
        .arg(dst.as_os_str())
        .output()?;

    if output.status.success() {
        Ok(())
    } else {
        let msg = String::from_utf8_lossy(&output.stderr).into_owned();
        Err(ErrorKind::SetModTimeFailed(dst.to_owned(), msg).into())
    }
}
