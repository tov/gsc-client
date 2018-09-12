use serde_derive::Deserialize;

type DateTime = chrono::DateTime<chrono::offset::FixedOffset>;

#[derive(Deserialize, Debug)]
pub struct SubmissionShort {
    pub assignment_number: usize,
    pub uri:               String,
}

#[derive(Deserialize, Debug)]
pub struct Submission {
    pub assignment_number:  usize,
    pub uri:                String,
    pub files_uri:          String,
    pub owner1:             Owner,
    pub owner2:             Option<Owner>,
    pub bytes_remaining:    usize,
    pub bytes_used:         usize,
    pub open_date:          DateTime,
    pub due_date:           DateTime,
    pub eval_date:          DateTime,
    pub last_modified:      DateTime,
    pub eval_status:        String,
    pub status:             String,
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

