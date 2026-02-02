use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use flate2::read::GzDecoder;
use mysql::{Row, Value};
use regex::Regex;
use serde_json::Value as JsonValue;
use std::io::Read;
use std::time::Duration;

async fn decode(content: Vec<u8>) -> Option<Vec<u8>> {
    let mut decompressed_data = Vec::new();

    let mut decoder = GzDecoder::new(content.as_slice());

    let result = decoder
        .read_to_end(&mut decompressed_data)
        .expect("gzip read failed");

    Some(decompressed_data)
}

pub async fn get_identity_from_flow(row: &Row) -> Option<String> {
    match row.get("input_params") {
        Some(Value::Bytes(bytes)) => {
            if bytes.is_empty() {
                None
            } else {
                let mut decoder = GzDecoder::new(&bytes[..]);
                let mut s = String::new();
                if let Ok(_) = decoder.read_to_string(&mut s) {
                    match serde_json::from_str::<JsonValue>(&s) {
                        Ok(json) => {
                            if let Some(prop) = json.get("props") {
                                if let Some(command) = prop.get("command") {
                                    let t = command.to_string();
                                    let parts: Vec<_> = t.split_whitespace().collect();
                                    let option = parts.last().cloned();
                                    Some(option.unwrap().trim_matches('"').to_string())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        }
                        Err(_) => None,
                    }
                } else {
                    None
                }
            }
        }
        _ => None,
    }
}

pub async fn decode_field(row: &Row, column: &str) -> Option<String> {
    match row.get("output_params") {
        Some(Value::NULL) => None,
        Some(Value::Bytes(bytes)) => {
            if bytes.is_empty() {
                None
            } else {
                let mut decoder = GzDecoder::new(&bytes[..]);
                let mut s = String::new();
                if let Ok(_) = decoder.read_to_string(&mut s) {
                    Some(s)
                } else {
                    None
                }
            }
        }
        _ => None,
    }
}

pub async fn get_datetime(row: &Row, column_name: &str) -> Option<DateTime<Utc>> {
    row.get(column_name).and_then(|v: Value| {
        if v == Value::NULL {
            return None;
        }
        let mut hit = v.as_sql(true);

        let hit = hit.trim_matches('\'');

        NaiveDateTime::parse_from_str(hit, "%Y-%m-%d %H:%M:%S%.f")
            .ok()
            .map(|ndt| Utc.from_utc_datetime(&ndt))
    })
}

pub fn duration(later: DateTime<Utc>, earlier: DateTime<Utc>) -> Duration {
    let diff = later - earlier;
    Duration::from_secs(diff.num_seconds() as u64)
        + Duration::from_nanos((diff.num_nanoseconds().unwrap() % 1_000_000_000) as u64)
}

pub fn starts_with_regex(s: &str, pattern: &str) -> bool {
    Regex::new(pattern) // ^ 表示字符串开始
        .unwrap()
        .is_match(s)
}

pub async fn core_sql() -> Option<&'static str> {
    let sql = r"

SELECT
    t.exec_id,
    t.name,
    t.flow_id,
    t.job_id,
    t.attempt,
    t.input_params,
    t.output_params,
    FROM_UNIXTIME(t.start_time / 1000) AS start_time,
    FROM_UNIXTIME(t.end_time / 1000) AS end_time
FROM (
    SELECT
        ej.exec_id,
        p.name,
        ej.flow_id,
        ej.job_id,
        ej.attempt,
        ej.input_params,
        ej.output_params,
        ej.start_time,
        ej.end_time,
        ROW_NUMBER() OVER (
            PARTITION BY p.name, ej.flow_id, ej.job_id
            ORDER BY ej.start_time DESC, ej.attempt DESC
        ) AS rn
    FROM azkaban.execution_jobs ej
    JOIN azkaban.projects p
        ON ej.project_id = p.id
    WHERE
        -- p.name IN ('warehouse', '_test') AND
        ej.start_time > (UNIX_TIMESTAMP(NOW(3)) * 1000) - 24 * 60 * 60 * 1000
        AND ej.status NOT IN (30, 50)
) t
WHERE t.rn = 1
ORDER BY t.name, t.flow_id, t.job_id;

      ";

    Some(sql)
}

pub fn format_duration_chinese(d: Duration) -> String {
    let total_secs = d.as_secs();
    match total_secs {
        0..=59 => format!("{}秒", total_secs),
        60..=3599 => {
            let mins = total_secs / 60;
            let secs = total_secs % 60;
            if secs == 0 {
                format!("{}分钟", mins)
            } else {
                format!("{}分钟{}秒", mins, secs)
            }
        }
        _ => {
            let hours = total_secs / 3600;
            let remaining = total_secs % 3600;
            let mins = remaining / 60;
            let secs = remaining % 60;

            match (mins, secs) {
                (0, 0) => format!("{}小时", hours),
                (_, 0) => format!("{}小时{}分钟", hours, mins),
                (0, _) => format!("{}小时{}秒", hours, secs),
                _ => format!("{}小时{}分钟{}秒", hours, mins, secs),
            }
        }
    }
}
