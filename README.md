# Azkaban Monitor

A monitoring system for Azkaban tasks that sends alerts to Feishu when tasks fail or encounter issues.

## Features

- Monitors for missing task executions
- Detects failed tasks
- Tracks retrying tasks
- Monitors error logs
- Sends alerts to Feishu

## Setup

1. Clone the repository
2. Copy `.env.example` to `.env` and fill in your configuration:
   ```bash
   cp .env.example .env
   ```

3. Edit `.env` with your settings:
   - `AZKABAN_DB_URL`: MySQL connection string for your Azkaban database
   - `FEISHU_WEBHOOK`: Your Feishu webhook URL
   - `RUST_LOG`: Logging level (debug, info, warn, error)

4. Build the project:
   ```bash
   cargo build --release 
   cargo build --release --target x86_64-unknown-linux-gnu
   ```

## Usage

Run the monitor:
```bash
cargo run --release
```

The monitor will:
- Check for issues every 5 minutes
- Send alerts to Feishu when problems are detected
- Log all activities to stdout

## Alert Types

1. Missing Executions: Tasks that should have started but haven't
2. Failed Tasks: Tasks that have failed
3. Retrying Tasks: Tasks that are on their second or later attempt
4. Error Logs: Tasks with error messages in their logs

## Requirements

- Rust 1.56 or later
- MySQL database with Azkaban tables
- Feishu webhook URL 