use crate::bean::{InitConfig, Job, Task};
use crate::config::read_config;
use crate::notice::send_with_struct_data;
use crate::parseflow::parse_project_file;
use crate::utli::{core_sql, decode_field, duration, get_datetime};
use alloc::string::String;
use anyhow::Result;
use mysql::prelude::*;
use mysql::*;
use reqwest::Client;
use std::collections::HashMap;

pub struct AzkabanMonitor {
    pool: Pool,
    config: InitConfig,
    client: Client,
}

impl AzkabanMonitor {
    pub fn new() -> Result<Self> {
        let config: &InitConfig = InitConfig::global();
        let db_url = &config.db_full_url as &str;

        let pool = Pool::new(db_url)?;
        let client = Client::new();

        Ok(Self {
            pool: pool,
            config: config.clone(),
            client: client,
        })
    }

    async fn process_execute_record(&self) -> Result<Vec<Task>> {
        let mut conn = self.pool.get_conn()?;

        let query = core_sql().await.unwrap_or_default();

        let query_results: Vec<Row> = conn.query(query)?;
        println!("Total results: {}", query_results.len());

        let mut result: Vec<Task> = vec![];

        for row in query_results {
            let exec_id: String = row.get("exec_id").unwrap_or_default();
            let project_name: String = row.get("name").unwrap_or_default();
            let flow_id: String = row.get("flow_id").unwrap_or_default();
            let job_id: String = row.get("job_id").unwrap_or_default();
            let attempt: u8 = row.get("attempt").unwrap_or_default();

            println!(
                "Processing task: project={}, exec_id={}, job_id={}",
                project_name, exec_id, job_id
            );

            // let owner = get_identity_from_flow(&row).await.unwrap_or_default();

            let input_params: String = decode_field(&row, "input_params").await.unwrap_or_default();
            let output_params: String = decode_field(&row, "output_params")
                .await
                .unwrap_or_default();

            let start_time = get_datetime(&row, "start_time").await.unwrap_or_default();

            let end_time = get_datetime(&row, "end_time").await.unwrap_or_default();

            let task = Task {
                exec_id: exec_id,
                project_name: project_name,
                flow_id: flow_id,
                job_id: job_id,
                attempt: attempt,
                start_time: start_time,
                end_time: end_time,
                input_params: input_params,
                output_params: output_params,
                owner: "".to_string(),
                duration: duration(end_time, start_time),
                desc: "".to_string(),
            };

            result.push(task);
        }

        Ok(result)
    }

    async fn generate_message(
        tasks: HashMap<String, HashMap<String, Vec<Task>>>,
    ) -> Result<String> {
        let mut messages = vec![];

        for t in tasks {
            let project = t.0;
            let flows = t.1;
            for f in flows {
                let flow = f.0;
                let tasks = f.1;
                let message = Self::do_generate_message(flow, tasks).await?;
                messages.push(message);
            }
        }

        Ok(messages.join("----------"))
    }

    async fn do_generate_message(flow: String, tasks: Vec<Task>) -> Result<String> {
        let mut multi_detail: Vec<String> = vec![];

        for x in tasks {
            let single = format!(
                "
                **flow_id** : {}\n\
                **job_id**: {}\n\
                **attempt**: {}\n\
                **start_time**: {}\n\
                **end_time**: {}\n\
                ",
                x.flow_id, x.job_id, x.attempt, x.start_time, x.end_time
            )
            .trim_start()
            .to_string();
            multi_detail.push(single)
        }

        let details = multi_detail.join("\n");

        Ok(format!(
            "
            flow:**{}**下有任务异常:\n{}
            ",
            flow, details
        )
        .trim_start()
        .to_string())
    }

    async fn merge_git_and_azkaban(
        &self,
        name_mapping: HashMap<String, String>,
        git_job: HashMap<String, HashMap<String, HashMap<String, Job>>>,
        az_task: Vec<Task>,
    ) -> Result<Vec<Task>> {
        let mut tasks = az_task;
        let mut jobs = git_job;

        for t in tasks.iter_mut() {
            let target_job = jobs
                .get(&t.project_name)
                .and_then(|flow| flow.get(&t.flow_id))
                .and_then(|job| job.get(&t.job_id));

            match target_job {
                Some(job) => {
                    let owner_name = &job.owner;
                    t.owner = name_mapping
                        .get(owner_name)
                        .map(Clone::clone)
                        .unwrap_or_else(|| {
                            println!("Warning: No mapping found for owner '{}'", owner_name);
                            String::new()
                        });
                    t.desc = job.desc.to_string()
                }
                None => {
                    println!(
                        "Warning: No matching job found for project '{}', flow '{}', job '{}'",
                        t.project_name, t.flow_id, t.job_id
                    );
                }
            }
        }

        Ok(tasks)
    }

    pub async fn run(&self) -> Result<()> {
        let mapping_file = &self.config.mapping_file as &str;
        let mappings = read_config(mapping_file).await.unwrap_or_default();

        let parse_jobs = parse_project_file().await?;

        let mut tasks = self.process_execute_record().await?;

        tasks = self
            .merge_git_and_azkaban(mappings, parse_jobs, tasks)
            .await?;

        if tasks.is_empty() {
            println!("No failed tasks found jump");
            return Ok(());
        }

        println!("Get total {} need-alert task ", tasks.len());

        let mut groups: HashMap<String, HashMap<String, HashMap<String, Vec<Task>>>> =
            HashMap::new();

        for t in tasks {
            let owner = t.owner.clone();
            let project = t.project_name.clone();
            let flow = t.flow_id.clone();

            groups
                .entry(owner)
                .or_insert_with(HashMap::new)
                .entry(project)
                .or_insert_with(HashMap::new)
                .entry(flow)
                .or_insert_with(Vec::new)
                .push(t)
        }

        for u in groups {
            let user_id = u.0.as_str();
            let projects = u.1;

            if user_id.is_empty() {
                println!("User id is empty jump all the task {:?}", projects);
                continue;
            }

            let urls = &self.config.feishu_url;
            for url in urls {
                send_with_struct_data(url, user_id, &projects)
                    .await
                    .expect("");
            }
        }
        Ok(())
    }
}
