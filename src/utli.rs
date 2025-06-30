use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use flate2::read::GzDecoder;
use mysql::{Row, Value};
use serde_json::Value as JsonValue;
use std::io::Read;



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

pub async fn get_output(row: &Row) -> Option<String> {
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
    --    p.name IN ('-fan', '_test') AND
        ej.start_time > (UNIX_TIMESTAMP(NOW(3)) * 1000) - 24 * 60 * 60 * 1000
        AND ej.status NOT IN (30, 50)
) t
WHERE t.rn = 1
ORDER BY t.name, t.flow_id, t.job_id;

      ";

    Some(sql)
}
