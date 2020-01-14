use serde_derive::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Debug)]
pub struct DateTime(chrono::DateTime<chrono::offset::FixedOffset>);

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
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum GraderEvalStatus {
    Editing,
    HeldBack,
    Ready,
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
    pub upload_time: DateTime,
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
    SelfEval,
    Extended,
    ExtendedEval,
    Closed,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionEvalStatus {
    Empty,
    Started,
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
    pub open_date: DateTime,
    pub due_date: DateTime,
    pub eval_date: DateTime,
    pub last_modified: DateTime,
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
    pub due_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_quota: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner2: Option<()>,
}

impl std::fmt::Display for DateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0.format("%b %d %H:%M"))
    }
}

impl SubmissionStatus {
    fn to_str(&self) -> &'static str {
        use self::SubmissionStatus::*;
        match *self {
            Future => "future",
            Open => "open for submission",
            SelfEval => "open for self evaluation",
            Extended => "open for submission (extended)",
            ExtendedEval => "open for self evaluation (extended)",
            Closed => "closed",
        }
    }

    pub fn is_self_eval(&self) -> bool {
        use self::SubmissionStatus::*;
        match *self {
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
        }
    }
}
