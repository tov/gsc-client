use serde_derive::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct SubmissionShort {
    pub assignment_number: usize,
    pub uri:               String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Submission {
    pub assignment_number:  usize,
    pub uri:                String,
    pub files_uri:          String,
    pub owner1:             Owner,
    pub owner2:             Option<Owner>,
    pub bytes_remaining:    usize,
    pub bytes_used:         usize,
    pub open_date:          String,
    pub due_date:           String,
    pub eval_date:          String,
    pub last_modified:      String,
    pub eval_status:        String,
    pub status:             String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Owner {
    name:                   String,
    uri:                    String,
}

