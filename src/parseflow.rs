//! parse flow file by git blame
//! so get the final owner

use crate::bean::{InitConfig, Job};
use crate::gitblame::blame;
use futures::stream::{Stream, StreamExt};
use regex::Regex;
use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use tokio::fs;
use tokio::io;

pub async fn parse(
) -> Result<HashMap<String, HashMap<String, HashMap<String, Job>>>, Box<dyn std::error::Error>> {
    let mut result: HashMap<String, HashMap<String, HashMap<String, Job>>> = HashMap::new();
    let config: &InitConfig = InitConfig::global();
    let cron_dirs = config.target_cron_dir.clone();
    for config_path in cron_dirs {
        let config_path = PathBuf::from(config_path);
        let parse_result = do_parse(&config_path).await?;

        result.extend(parse_result);
    }

    Ok(result)
}

async fn do_parse(
    config_path: &PathBuf,
) -> Result<HashMap<String, HashMap<String, HashMap<String, Job>>>, Box<dyn std::error::Error>> {
    let mut result: HashMap<String, HashMap<String, HashMap<String, Job>>> = HashMap::new();

    let all_files = r_list(config_path.clone()).await?;

    for (k, files) in all_files {
        println!("Handle cron file : {:?}", k);

        let project_file = files
            .iter()
            .find(|p| p.extension().map(|e| e == "project").unwrap_or(false));
        if project_file.is_none() {
            continue;
        }

        let project_pure_name = project_file
            .unwrap()
            .file_stem()
            .map(|t| t.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        for f in files.iter() {
            if f.eq(project_file.unwrap()) {
                continue;
            }

            let cuts: HashMap<String, Vec<Job>> = cut_flow_into_each_task(&f).await?; // todo

            cuts.iter().for_each(|t| {
                let flow_id = t.0;

                for job in t.1 {
                    let job_name = &job.job;
                    result
                        .entry(project_pure_name.clone())
                        .or_insert_with(HashMap::new)
                        .entry(flow_id.clone())
                        .or_insert_with(HashMap::new)
                        .insert(job_name.to_string(), job.clone());
                }
            })
        }
    }

    Ok(result)
}

fn r_list(
    input: PathBuf,
) -> Pin<Box<dyn Future<Output = io::Result<HashMap<PathBuf, Vec<PathBuf>>>> + Send + 'static>> {
    Box::pin(async move {
        if !input.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Directory does not exist",
            ));
        }

        if !input.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotADirectory,
                "Input is not a directory",
            ));
        }

        let mut result = HashMap::new();
        let mut files = Vec::new();

        let mut entries = fs::read_dir(&input).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            let meta = fs::metadata(&path).await.unwrap();

            if meta.is_file() {
                files.push(path.clone());
            } else if meta.is_dir() {
                let spawn_path = path.clone();
                let sub_result = tokio::spawn(r_list(spawn_path)).await??;
                println!("{}", sub_result.len());
                result.extend(sub_result);
            }
        }

        result.insert(input.clone(), files.clone());
        Ok(result)
    })
}


pub async fn cut_flow_into_each_task(
    config_path: &PathBuf,
) -> Result<HashMap<String, Vec<Job>>, Box<dyn std::error::Error>> {
    let mut result: HashMap<String, Vec<Job>> = HashMap::new();
    let content = fs::read_to_string(config_path).await?;
    let filename = config_path
        .file_stem()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let lines: Vec<&str> = content.lines().collect();

    let mut in_nodes = false;
    let mut jobs: Vec<Job> = Vec::new();
    let mut start_line: Option<u16> = None;
    let mut end_line: Option<u16> = None;
    let mut job_name = String::new();
    let mut other = String::new();
    let mut desc = String::new();

    let re_name = Regex::new(r"^\s*-\s*name:\s*(\S+)").unwrap();
    let re_comment = Regex::new(r"^\s*#(.*)").unwrap();

    for (idx, line) in lines.iter().enumerate() {
        let lno = (idx + 1) as u16;

        if line.trim_start().starts_with("nodes:") {
            in_nodes = true;
            continue;
        }
        if !in_nodes {
            continue;
        }
        if line.trim().eq("") {
            continue;
        }

        if let Some(cap) = re_name.captures(line) {
            if let Some(st) = start_line {
                jobs.push(Job {
                    start: st,
                    end: end_line.unwrap_or(lno - 1),
                    flow: filename.clone(),
                    job: job_name.clone(),
                    other: other.trim().to_string(),
                    flow_file: config_path.clone(),
                    owner: String::from(""),
                });
            }
            start_line = Some(lno);
            job_name = cap[1].to_string();
            other = desc.trim().to_string();
            desc.clear();
            end_line = None;
        }

        if start_line.is_none() {
            if let Some(cap) = re_comment.captures(line) {
                desc = cap[1].trim().to_string();
            }
        }

        end_line = Some(lno);
    }

    if let Some(st) = start_line {
        jobs.push(Job {
            start: st,
            end: end_line.unwrap_or(lines.len() as u16),
            flow: filename.clone(),
            job: job_name,
            other: other.trim().to_string(),
            flow_file: config_path.clone(),
            owner: String::from(""),
        });
    }

    for job in jobs.iter_mut() {
        let username = blame(job).await.unwrap();
        job.owner = username;
        result
            .entry(filename.clone())
            .or_insert_with(Vec::new)
            .push(job.clone());
    }

    Ok(result)
}


#[cfg(test)]

mod tests {
    use crate::gitblame::blame;
    use crate::parseflow::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_do_parse() -> Result<(), Box<dyn std::error::Error>> {
        let config_path = PathBuf::from("/Users/heise/enterprise/playground/new/ware/cron/");

        let map: HashMap<String, HashMap<String, HashMap<String, Job>>> =
            do_parse(&config_path).await?;

        for x in map.values().into_iter() {
            for y in x.values().into_iter() {
                for z in y.values().into_iter() {
                    let bz = blame(z).await;
                }
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_parse() -> Result<(), Box<dyn std::error::Error>> {
        let parse = parse().await?;

        let i = parse.len();

        println!("{}", i);

        Ok(())
    }

    #[tokio::test]
    async fn test_parse_single_project() -> Result<(), Box<dyn std::error::Error>> {
        let config_path = PathBuf::from("/Users/heise/enterprise/playground/new/ware/cron/");

        let parse = r_list(config_path).await.unwrap();

        let i = parse.len();
        println!("{}", i);

        Ok(())
    }
}
