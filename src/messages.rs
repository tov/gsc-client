use serde_derive::{Serialize, Deserialize};

#[derive(Clone, Deserialize, Debug)]
pub struct DateTime(chrono::DateTime<chrono::offset::FixedOffset>);

#[derive(Deserialize, Serialize, Debug)]
pub struct ExamGrade {
    pub number:             usize,
    pub points:             usize,
    pub possible:           usize,
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

#[derive(Deserialize, Debug)]
pub struct FileMeta {
    pub byte_count:         usize,
    pub media_type:         String,
    pub name:               String,
    pub purpose:            FilePurpose,
    pub upload_time:        DateTime,
    pub uri:                String,
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
    pub assignment_number:  usize,
    pub user:               String,
    pub status:             PartnerRequestStatus,
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
    pub name:               String,
    pub uri:                String,
    pub submissions_uri:    String,
}

#[derive(Deserialize, Debug)]
pub struct User {
    pub name:               String,
    pub uri:                String,
    pub submissions_uri:    String,
    pub role:               UserRole,
    pub exam_grades:        Vec<ExamGrade>,
    pub partner_requests:   Vec<PartnerRequest>,
    pub submissions:        Vec<SubmissionShort>,
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
    pub assignment_number:  usize,
    pub id:                 usize,
    pub uri:                String,
    pub status:             SubmissionStatus,
    pub grade:              f64,
}

#[derive(Deserialize, Debug)]
pub struct Submission {
    pub assignment_number:  usize,
    pub id:                 usize,
    pub uri:                String,
    pub grade:              f64,
    pub files_uri:          String,
    pub owner1:             UserShort,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner2:             Option<UserShort>,
    pub bytes_used:         usize,
    pub bytes_quota:        usize,
    pub open_date:          DateTime,
    pub due_date:           DateTime,
    pub eval_date:          DateTime,
    pub last_modified:      DateTime,
    pub eval_status:        SubmissionEvalStatus,
    pub status:             SubmissionStatus,
}

#[derive(Serialize, Debug)]
pub struct UserChange {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exam_grades:      Vec<ExamGrade>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub partner_requests: Vec<PartnerRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password:         Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role:             Option<UserRole>,
}

#[derive(Serialize, Debug)]
pub struct SubmissionChange {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date:           Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_date:          Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_quota:        Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner2:             Option<()>,
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
            Future       => "future",
            Open         => "open for submission",
            SelfEval     => "open for self evaluation",
            Extended     => "open for submission (extended)",
            ExtendedEval => "open for self evaluation (extended)",
            Closed       => "closed",
        }
    }

    pub fn is_self_eval(&self) -> bool {
        use self::SubmissionStatus::*;
        match *self {
            SelfEval     => true,
            ExtendedEval => true,
            _            => false,
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
            Empty    => "empty",
            Started  => "started",
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

impl Default for SubmissionChange {
    fn default() -> Self {
        SubmissionChange {
            due_date:       None,
            eval_date:      None,
            bytes_quota:    None,
            owner2:         None,
        }
    }
}

impl FilePurpose {
    pub fn to_char(&self) -> char {
        use self::FilePurpose::*;

        match self {
            Source   => 's',
            Test     => 't',
            Config   => 'c',
            Resource => 'r',
            Log      => 'l',
        }
    }

    pub fn to_dir(&self) -> &str {
        use self::FilePurpose::*;

        match self {
            Source   => "src",
            Test     => "test",
            Config   => ".",
            Resource => "Resources",
            Log      => "build",
        }
    }
}

impl Default for UserChange {
    fn default() -> Self {
        UserChange {
            exam_grades:      Vec::new(),
            partner_requests: Vec::new(),
            password:         None,
            role:             None,
        }
    }
}

