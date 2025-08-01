use crate::utli::format_duration_chinese;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

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
    pub input_params: String,
    pub output_params: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration: Duration,
    pub desc: String,
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
    pub desc: String,
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
            desc: self.desc.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Configuration {
    pub cron: Vec<String>,
}

impl Task {
    pub fn to_string(&self) -> Result<String, anyhow::Error> {
        Ok(format!(
            "
                **flow_id** : {}\n\
                **exec_id** : {}\n\
                **job_id**: {}\n\
                **attempt**: {}\n\
                **start_time**: {}\n\
                **end_time**: {}\n\
                **duration**: {}\n\
                ",
            self.flow_id,
            self.exec_id,
            self.job_id,
            self.attempt,
            self.start_time,
            self.end_time,
            format_duration_chinese(self.duration),
        )
        .trim_start()
        .to_string())
    }
}
