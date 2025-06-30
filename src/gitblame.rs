use crate::bean::Job;
use anyhow::{anyhow, Result};
use chrono::NaiveDateTime;
use regex::Regex;
use std::path::PathBuf;
use tokio::process::Command;

pub async fn blame(job: &Job) -> Result<String> {
    let start_line_no = job.start;
    let end_line_no = job.end;
    let origin_file = &job.flow_file;

    let parent_path = origin_file
        .parent()
        .ok_or_else(|| anyhow!("Can't get parent dir"))?
        .canonicalize()?;

    let file_name = origin_file
        .file_name()
        .ok_or_else(|| anyhow!("Can't get file name"))?
        .to_string_lossy()
        .to_string();

    let output = Command::new("git")
        .current_dir(&parent_path)
        .arg("blame")
        .arg(format!("-L{},{}", start_line_no, end_line_no))
        .arg(file_name)
        .output()
        .await?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("git blame failed: {}", err).into());
    }

    let blame_info = String::from_utf8_lossy(&output.stdout);

    let owner = extract_latest_author(blame_info.to_string().as_str()).await?;

    Ok(owner)
}

async fn extract_latest_author(blame_info: &str) -> Result<String> {
    let re = Regex::new(r"\((\S+)\s+(\d{4}-\d{2}-\d{2})\s+(\d{2}:\d{2}:\d{2})").unwrap();
    let mut latest_author = None;
    let mut latest_dt = None;

    for line in blame_info.lines() {
        if let Some(cap) = re.captures(line) {
            let author = cap[1].to_string();
            let date_str = &cap[2];
            let time_str = &cap[3];
            let dt_str = format!("{} {}", date_str, time_str);
            if let Ok(dt) = NaiveDateTime::parse_from_str(&dt_str, "%Y-%m-%d %H:%M:%S") {
                if latest_dt.is_none() || dt > latest_dt.unwrap() {
                    latest_dt = Some(dt);
                    latest_author = Some(author);
                }
            }
        }
    }
    Ok(latest_author.unwrap_or_default())
}

#[cfg(test)]
#[tokio::test]
async fn test_blame() {
    let job = Job {
        flow_file: PathBuf::from(""),
        start: 217,
        end: 227,
        flow: "idc_bill_cost_new".to_string(),
        job: "dwd_idc_tencloud_bill_detail_pmi".to_string(),
        other: " ".to_string(),
        owner: "".to_string(),
    };

    let r = blame(&job).await.unwrap();
}
