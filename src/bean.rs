use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize, Clone, Serialize, Debug)]
pub struct InitConfig {
    pub feishu_url: Vec<String>,
    pub target_cron_dir: Vec<String>,
    pub mapping_file: String,
    pub db_full_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub exec_id: String,
    pub project_name: String,
    pub flow_id: String,
    pub job_id: String,
    pub attempt: u8,
    pub owner: String,
    pub output_params: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Job {
    pub start: u16,
    pub end: u16,
    pub flow: String,
    pub job: String,
    pub other: String,
    pub flow_file: PathBuf,
    pub owner: String,
}

impl Clone for Job {
    fn clone(&self) -> Self {
        Job {
            start: self.start,
            end: self.end,
            flow: self.flow.clone(),
            job: self.job.clone(),
            other: self.other.clone(),
            flow_file: self.flow_file.clone(),
            owner: self.owner.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Configuration {
    pub cron: Vec<String>,
}
