use serde_derive::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct DateTime(chrono::DateTime<chrono::offset::FixedOffset>);

impl std::fmt::Display for DateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0.format("%b %d %H:%M"))
    }
}

#[derive(Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SubmissionShort {
    pub assignment_number: usize,
    pub uri:               String,
}

#[derive(Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionStatus {
    Future,
    Open,
    SelfEval,
    Extended,
    ExtendedEval,
    Closed,
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

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionEvalStatus {
    Empty,
    Started,
    Complete,
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

#[derive(Deserialize, Debug)]
pub struct Submission {
    pub assignment_number:  usize,
    pub uri:                String,
    pub files_uri:          String,
    pub owner1:             Owner,
    pub owner2:             Option<Owner>,
    pub bytes_used:         usize,
    pub bytes_quota:        usize,
    pub open_date:          DateTime,
    pub due_date:           DateTime,
    pub eval_date:          DateTime,
    pub last_modified:      DateTime,
    pub eval_status:        SubmissionEvalStatus,
    pub status:             SubmissionStatus,
}

#[derive(Deserialize, Debug)]
pub struct Owner {
    pub name:               String,
    pub uri:                String,
}

#[derive(Deserialize, Debug)]
pub struct FileMeta {
    pub byte_count:         usize,
    pub media_type:         String,
    pub name:               String,
    pub purpose:            String,
    pub upload_time:        DateTime,
    pub uri:                String,
}

#[derive(Serialize, Debug)]
pub struct PasswordChange {
    pub password:           String,
}
