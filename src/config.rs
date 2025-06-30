use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::OnceLock;

use crate::bean::InitConfig;
use std::fs;

static CONFIG: OnceLock<InitConfig> = OnceLock::new();

pub fn init() -> Result<InitConfig, Box<dyn std::error::Error>> {
    let pwd = std::env::current_dir()?;

    println!("pwd : {:?}", pwd);

    let preferred_config_path = pwd.join("config").join("monitor.config");
    let fallback_config_path = pwd.join("monitor.config");

    let config_path = if preferred_config_path.exists() {
        preferred_config_path
    } else if fallback_config_path.exists() {
        fallback_config_path
    } else {
        return Err(format!(
            "Could not find monitor.config in either:\n- {}\n- {}",
            preferred_config_path.display(),
            fallback_config_path.display()
        )
        .into());

        // todo
        //      return Err(
        //          format!("Not config was foudn")
        //      ).into();
        // }
    };

    let content = fs::read_to_string(config_path)?;

    let filter_content: String = content
        .lines()
        .filter(|l| !l.trim().starts_with("//"))
        .collect();

    let init_config: InitConfig = serde_json::from_str(&filter_content)?;

    println!("get the config : {:?}", init_config);

    Ok(init_config)
}

impl InitConfig {

    pub fn do_parse() -> Result<(), Box<dyn std::error::Error>> {
        let config = init()?;

        CONFIG.set(config).expect("Config already initialized");

        Ok(())
    }
    pub fn global() -> &'static InitConfig {
        CONFIG.get().expect("Config not initialized")
    }
}

pub async fn read_config(
    file_path: &str,
) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let config = InitConfig::global(); 

    let config_full_name = &config.mapping_file;

    let mut config = HashMap::new();

    let file = File::open(config_full_name);

    let reader = BufReader::new(file?);

    for line in reader.lines() {
        let line = line?;
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.splitn(4, ",").collect();

        if parts.len() == 2 {
            let key = parts[0].to_string();
            let value = parts[1].to_string();
            config.insert(key, value);
            
        } else if parts.len() == 3 {
            let key1 = parts[0].to_string();
            let key2 = parts[1].to_string();
            let value = parts[2].to_string();
            config.insert(key1, value.clone());
            config.insert(key2, value);
            
        } else if parts.len() == 4 {
            let key1 = parts[0].to_string();
            let key2 = parts[1].to_string();
            let key3 = parts[2].to_string();
            let value = parts[3].to_string();
            
            config.insert(key1, value.clone());
            config.insert(key2, value.clone());
            config.insert(key3, value);
        }
    }
    Ok(config)
}

#[cfg(test)]
mod tests {
    use crate::config::{init, read_config};

    #[tokio::test]
    async fn test_init_config() -> Result<(), Box<dyn std::error::Error>> {
        let result = init()?;

        Ok(())
    }

    #[tokio::test]
    async fn test_read_config() -> Result<(), Box<dyn std::error::Error>> {
        let file_path = "/Users/heise/source/config/taskr.csv";
        let config = read_config(file_path).await.unwrap();
        let value = config
            .get("9527")
            .cloned()
            .unwrap_or("NOT_FOUND".into());

        Ok(())
    }
}
