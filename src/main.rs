extern crate alloc;

mod monitor;
mod utli;

mod bean;
mod config;
mod gitblame;
mod notice;
mod parseflow;

use crate::bean::InitConfig;
use dotenv::dotenv;
use log::info;
use monitor::AzkabanMonitor as Monitor;
use serde_json::json;
use std::env;
use std::time::Duration;
use tokio::time;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Load environment variables
    dotenv().ok();

    InitConfig::do_parse()?;

    // Create monitor instance
    let monitor = Monitor::new()?;

    info!("Starting Azkaban monitoring service...");

    // Check for failed tasks
    if let Err(e) = monitor.run().await {
        log::error!("Error checking failed tasks: {}", e);
    }

    Ok(())
}
