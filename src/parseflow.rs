//! parse flow file by git blame
//! so get the final owner

use crate::bean::{InitConfig, Job};
use crate::gitblame::blame;
use crate::utli::starts_with_regex;
use anyhow::Result;
use futures::future::join_all;
use futures::stream::{Stream, StreamExt};
use regex::Regex;
use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use tokio::fs;
use tokio::io;

pub async fn cut_flow_into_each_task(config_path: &PathBuf) -> Result<HashMap<String, Vec<Job>>> {
    let mut result: HashMap<String, Vec<Job>> = HashMap::new();
    let content = fs::read_to_string(config_path).await?;
    let filename = config_path
        .file_stem()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let lines: Vec<&str> = content.lines().collect();

    let mut jobs: Vec<Job> = Vec::new();
    let mut current_job: Option<JobBuilder> = None;

    #[derive(Debug)]
    struct JobBuilder {
        name: String,
        start_line: u16,
        end_line: i16,
        other: String,
        desc: String,
    }

    for (idx, line) in lines.iter().enumerate() {
        let lno = (idx + 1) as u16;
        let trimmed = line.trim();

        // 匹配任务开始
        if let Some(cap) = Regex::new(r"^\s*-\s*name:\s*(\S+)").unwrap().captures(line) {
            let job_name = cap.get(1).unwrap().as_str().to_string();

            if job_name == "dwd_v_income_zy_pdf" {
                print!("dwd_v_income_zy_pdf")
            }

            if let Some(mut builder) = current_job.take() {
                if builder.end_line == -1 {
                    builder.end_line = (lno - 1) as i16
                }

                // 保存上一个任务
                jobs.push(Job {
                    start: builder.start_line,
                    end: (builder.end_line - 1) as u16,
                    flow: filename.clone(),
                    job: builder.name,
                    other: builder.desc.trim().to_string(),
                    flow_file: config_path.clone(),
                    owner: String::new(),
                    desc: String::new(), // todo
                });
            }

            // 开始新任务
            current_job = Some(JobBuilder {
                name: job_name,
                start_line: lno,
                end_line: -1,
                other: String::new(),
                desc: String::new(),
            });
        } else if let Some(builder) = &mut current_job {
            // 如果是command行，更新end_line
            if trimmed.trim().is_empty() && builder.end_line == -1 {
                builder.end_line = (lno - 1) as i16;
            }
            if starts_with_regex(trimmed, "^\\s*#(?:\\s+[^\\s=][^=]*|\\S+)")
                && builder.desc.trim().is_empty()
            {
                builder
                    .desc
                    .push_str(trimmed.trim_start_matches('#').trim());
                builder.desc.push('\n');
            }
        }
    }

    // 处理最后一个任务
    if let Some(builder) = current_job.take() {
        jobs.push(Job {
            start: builder.start_line,
            end: lines.len() as u16,
            flow: filename.clone(),
            job: builder.name,
            other: builder.desc.trim().to_string(),
            flow_file: config_path.clone(),
            owner: String::new(),
            desc: builder.desc,
        });
    }

    // 获取每个任务的owner
    for job in jobs.iter_mut() {
        job.owner = blame(job).await.unwrap_or_default();
        result
            .entry(filename.clone())
            .or_default()
            .push(job.clone());
    }

    Ok(result)
}

pub async fn parse_project_file() -> Result<HashMap<String, HashMap<String, HashMap<String, Job>>>>
{
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
) -> Result<HashMap<String, HashMap<String, HashMap<String, Job>>>> {
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

        // Create a vector to store all the futures
        let mut futures = Vec::new();

        for f in files.iter() {
            if f.eq(project_file.unwrap()) {
                continue;
            }

            let f = f.clone();
            // Spawn a new task for each file
            let future = tokio::spawn(async move { cut_flow_into_each_task(&f).await });
            futures.push(future);
        }

        // Wait for all futures to complete
        let results = join_all(futures).await;

        // Process results
        for task_result in results {
            match task_result {
                Ok(Ok(cuts)) => {
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
                    });
                }
                Ok(Err(e)) => eprintln!("Error processing file: {}", e),
                Err(e) => eprintln!("Task failed: {}", e),
            }
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

#[cfg(test)]

mod tests {
    use crate::gitblame::blame;
    use crate::parseflow::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_do_parse() -> Result<()> {
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
    async fn test_parse() -> Result<()> {
        let parse = parse_project_file().await?;

        let i = parse.len();

        println!("{}", i);

        Ok(())
    }

    #[tokio::test]
    async fn test_parse_single_project() -> Result<()> {
        let config_path = PathBuf::from("/Users/heise/enterprise/playground/new/ware/cron/");

        let parse = r_list(config_path).await.unwrap();

        let i = parse.len();
        println!("{}", i);

        Ok(())
    }
}
