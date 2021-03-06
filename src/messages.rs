use chrono::format::{DelayedFormat, StrftimeItems};
use chrono::{offset, DateTime};
use serde::Serializer;
use serde_derive::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Debug)]
pub struct UtcDateTime(DateTime<offset::Utc>);

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum EvalType {
    Boolean,
    Scale,
    Informational,
}

#[derive(Deserialize, Debug)]
pub struct EvalShort {
    pub uri: String,
    pub sequence: usize,
    pub submission_uri: String,
    #[serde(rename = "type")]
    pub eval_type: EvalType,
}

#[derive(Deserialize, Debug)]
pub struct Eval {
    pub uri: String,
    pub sequence: usize,
    pub submission_uri: String,
    #[serde(rename = "type")]
    pub eval_type: EvalType,
    pub prompt: String,
    pub value: f64,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub self_eval: Option<SelfEval>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grader_eval: Option<GraderEval>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ExamGrade {
    pub number: usize,
    pub points: usize,
    pub possible: usize,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum FilePurpose {
    Source,
    Test,
    Config,
    Resource,
    Log,
    Forbidden,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum GraderEvalStatus {
    Editing,
    HeldBack,
    Ready,
    Regrade,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GraderEval {
    pub uri: String,
    pub grader: String,
    pub score: f64,
    pub explanation: String,
    pub status: GraderEvalStatus,
}

#[derive(Deserialize, Debug)]
pub struct FileMeta {
    #[serde(rename = "assignment_number")]
    pub hw: usize,
    pub byte_count: usize,
    pub media_type: String,
    pub name: String,
    pub purpose: FilePurpose,
    pub upload_time: UtcDateTime,
    pub uri: String,
}

impl std::fmt::Display for FileMeta {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "hw{}:{}", self.hw, self.name)
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum JsonResult {
    Success(String),
    Failure(String),
    Nested(Vec<JsonResult>),
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum PartnerRequestStatus {
    Outgoing,
    Incoming,
    Accepted,
    Canceled,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PartnerRequest {
    pub assignment_number: usize,
    pub user: String,
    pub status: PartnerRequestStatus,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    Student,
    Grader,
    Admin,
}

impl UserRole {
    fn as_str(self) -> &'static str {
        match self {
            UserRole::Student => "student",
            UserRole::Grader  => "grader",
            UserRole::Admin   => "admin",
        }
    }
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Serialize, Debug)]
pub struct UserCreate<'a> {
    pub name: &'a str,
    pub role: UserRole,
}

#[derive(Deserialize, Debug)]
pub struct UserShort {
    pub name: String,
    pub uri: String,
}

#[derive(Deserialize, Debug)]
pub struct User {
    pub name: String,
    pub uri: String,
    pub submissions_uri: String,
    pub role: UserRole,
    pub exam_grades: Vec<ExamGrade>,
    pub partner_requests: Vec<PartnerRequest>,
    pub submissions: Vec<SubmissionShort>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SelfEval {
    pub uri: String,
    pub score: f64,
    pub explanation: String,
    pub permalink: String,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionStatus {
    Future,
    Open,
    Extended,
    Overtime,
    SelfEval,
    ExtendedEval,
    Closed,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionEvalStatus {
    Empty,
    Started,
    Overdue,
    Complete,
}

#[derive(Deserialize, Debug)]
pub struct SubmissionShort {
    pub assignment_number: usize,
    pub id: usize,
    pub uri: String,
    pub status: SubmissionStatus,
    pub grade: f64,
    pub owner1: UserShort,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner2: Option<UserShort>,
}

#[derive(Deserialize, Debug)]
pub struct Submission {
    pub assignment_number: usize,
    pub id: usize,
    pub uri: String,
    pub grade: f64,
    pub files_uri: String,
    pub evals_uri: String,
    pub owner1: UserShort,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner2: Option<UserShort>,
    pub bytes_used: usize,
    pub bytes_quota: usize,
    pub open_date: UtcDateTime,
    pub due_date: UtcDateTime,
    pub eval_date: UtcDateTime,
    pub last_modified: UtcDateTime,
    pub eval_status: SubmissionEvalStatus,
    pub status: SubmissionStatus,
}

#[derive(Serialize, Debug, Default)]
pub struct FileMetaChange {
    #[serde(rename = "assignment_number")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hw: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<FilePurpose>,
    pub overwrite: bool,
}

impl std::fmt::Display for FileMetaChange {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let name = self.name.as_ref().map(String::as_str).unwrap_or("");
        match self.hw {
            Some(hw) => write!(f, "hw{}:{}", hw, name),
            None => write!(f, ":{}", name),
        }
    }
}

#[derive(Serialize, Debug, Default)]
pub struct UserChange {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exam_grades: Vec<ExamGrade>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub partner_requests: Vec<PartnerRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<UserRole>,
}

#[derive(Serialize, Debug, Default)]
pub struct SubmissionChange {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<UtcDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_date: Option<UtcDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_quota: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner2: Option<()>,
}

impl UtcDateTime {
    pub fn into_local(self) -> DateTime<offset::Local> {
        self.0.into()
    }

    pub fn format_local<'a>(&self, fmt: &'a str) -> DelayedFormat<StrftimeItems<'a>> {
        self.clone().into_local().format(fmt)
    }

    // [[CC]YY]MMDDhhmm[.ss]
    pub fn touch_t_fmt(&self) -> DelayedFormat<StrftimeItems> {
        self.format_local("%Y%m%d%H%M.%S")
    }
}

impl serde::Serialize for UtcDateTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let str = self.0.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        serializer.serialize_str(&str)
    }
}

impl std::fmt::Display for UtcDateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.format_local("%a %d %b, %H:%M (%z)"))
    }
}

impl std::str::FromStr for UtcDateTime {
    type Err = chrono::format::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        type Fixed = DateTime<offset::FixedOffset>;
        let fixed = Fixed::parse_from_str(s, "%Y-%m-%d %H:%M:%S %z")?;
        Ok(Self(fixed.into()))
    }
}

impl SubmissionStatus {
    fn to_str(&self) -> &'static str {
        use self::SubmissionStatus::*;
        match *self {
            Future => "future",
            Open => "open for submission",
            Extended => "open for submission (extended)",
            Overtime => "overtime submission or self-eval",
            SelfEval => "open for self evaluation",
            ExtendedEval => "open for self evaluation (extended)",
            Closed => "closed",
        }
    }

    pub fn is_self_eval(&self) -> bool {
        use self::SubmissionStatus::*;
        match *self {
            Overtime => true,
            SelfEval => true,
            ExtendedEval => true,
            _ => false,
        }
    }
}

impl std::fmt::Display for SubmissionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(self.to_str())
    }
}

impl SubmissionEvalStatus {
    fn to_str(&self) -> &'static str {
        use self::SubmissionEvalStatus::*;
        match *self {
            Empty => "empty",
            Started => "started",
            Overdue => "overdue",
            Complete => "complete",
        }
    }
}

impl std::fmt::Display for SubmissionEvalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(self.to_str())
    }
}

impl Submission {
    pub fn quota_remaining(&self) -> f32 {
        100.0 * (self.bytes_quota - self.bytes_used) as f32 / self.bytes_quota as f32
    }
}

impl FilePurpose {
    pub fn to_char(&self) -> char {
        use self::FilePurpose::*;

        match self {
            Source => 's',
            Test => 't',
            Config => 'c',
            Resource => 'r',
            Log => 'l',
            Forbidden => 'F',
        }
    }

    pub fn to_dir(&self) -> &str {
        use self::FilePurpose::*;

        match self {
            Source => "src",
            Test => "test",
            Config => ".",
            Resource => "Resources",
            Log => ".",
            Forbidden => ".",
        }
    }
}
