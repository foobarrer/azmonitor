use crate::bean::{InitConfig, Job, Task};
use crate::config::read_config;
use crate::notice::send;
use crate::parseflow::parse;
use crate::utli::{core_sql, get_datetime, get_output};
use alloc::boxed::Box;
use alloc::string::String;
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
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
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

    async fn process(&self) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
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

            let output_params: String = get_output(&row).await.unwrap_or_default();

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
                output_params: output_params,
                owner: "".to_string(),
            };

            result.push(task);
        }

        Ok(result)
    }

    async fn generate_message(
        tasks: HashMap<String, Vec<Task>>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut messages = vec![];

        for t in tasks {
            let message = Self::do_generate_message(t.0.as_str(), t.1).await.unwrap();
            messages.push(message);
        }

        Ok(messages.join("----------"))
    }

    async fn do_generate_message(
        project: &str,
        tasks: Vec<Task>,
    ) -> Result<String, Box<dyn std::error::Error>> {
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
            **{}** 项目下有任务异常:\n{}
            ",
            project, details
        )
        .trim_start()
        .to_string())
    }

    async fn merge_git_and_azkaban(
        &self,
        name_mapping: HashMap<String, String>,
        git_job: HashMap<String, HashMap<String, HashMap<String, Job>>>,
        az_task: Vec<Task>,
    ) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
        let mut tasks = az_task;
        let mut jobs = git_job;

        for t in tasks.iter_mut() {
            let owner_name = match jobs
                .get(&t.project_name)
                .and_then(|flow| flow.get(&t.flow_id))
                .and_then(|job| job.get(&t.job_id))
            {
                Some(j) => &j.owner,
                None => "",
            };

            let owner_fei_id = name_mapping
                .get(owner_name)
                .cloned()
                .unwrap_or("".to_string());

            t.owner = owner_fei_id
        }

        Ok(tasks)
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mapping_file = &self.config.mapping_file as &str;
        let mappings = read_config(mapping_file).await.unwrap_or_default();

        let parse_jobs = parse().await?;

        let tasks = self.process().await.unwrap_or(vec![]);

        let tasks = self
            .merge_git_and_azkaban(mappings, parse_jobs, tasks)
            .await?;

        if tasks.is_empty() {
            println!("No failed tasks found jump");
            return Ok(());
        }

        let mut groups: HashMap<String, HashMap<String, Vec<Task>>> = HashMap::new();

        for t in tasks {
            let key = t.owner.clone();
            let flow = t.flow_id.clone();

            let time = t.end_time;

            groups
                .entry(key)
                .or_insert_with(HashMap::new)
                .entry(flow)
                .or_insert_with(Vec::new)
                .push(t)
        }

        for g in groups {
            let user_id = g.0.as_str();

            let message = Self::generate_message(g.1).await.unwrap();

            let urls = &self.config.feishu_url;

            for url in urls {
                send(url, user_id, message.as_str()).await;
            }
        }

        Ok(())
    }
}
