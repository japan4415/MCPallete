use std::env;
use std::fs;
use std::path::PathBuf;
use regex::Regex;
use crate::model::*;

pub fn ensure_config() -> Result<(), Box<dyn std::error::Error>> {
    let config_dir = match env::var("XDG_CONFIG_HOME") {
        Ok(val) => PathBuf::from(val).join("mcpallete"),
        Err(_) => {
            let home = env::var("HOME")?;
            PathBuf::from(home).join(".config/mcpallete")
        }
    };
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }
    let config_file = config_dir.join("basic_config.json");
    if !config_file.exists() {
        use std::io::Write;
        let mut file = fs::File::create(config_file)?;
        file.write_all(b"{\n  \"mcpServers\": {}\n}\n")?;
    }
    Ok(())
}

pub fn get_config_file_path() -> PathBuf {
    let config_dir = match env::var("XDG_CONFIG_HOME") {
        Ok(val) => PathBuf::from(val).join("mcpallete"),
        Err(_) => {
            let home = env::var("HOME").expect("HOME環境変数が必要です");
            PathBuf::from(home).join(".config/mcpallete")
        }
    };
    config_dir.join("basic_config.json")
}

pub fn load_config() -> Result<McpServersConfig, Box<dyn std::error::Error>> {
    let path = get_config_file_path();
    let content = std::fs::read_to_string(&path)?;
    let cfg = serde_json::from_str::<McpServersConfig>(&content)?;
    Ok(cfg)
}

pub fn expand_env_vars(s: &str) -> Result<String, Box<dyn std::error::Error>> {
    let re = Regex::new(r"\$([A-Za-z_][A-Za-z0-9_]*)|\$\{([A-Za-z_][A-Za-z0-9_]*)\}")?;
    Ok(re.replace_all(s, |caps: &regex::Captures| {
        let var = caps.get(1).or_else(|| caps.get(2)).map(|m| m.as_str()).unwrap_or("");
        std::env::var(var).unwrap_or_else(|_| "".to_string())
    }).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_expand_env_vars_basic() {
        unsafe { env::set_var("TEST_EXPAND", "hello"); }
        let input = "Value: $TEST_EXPAND!";
        let expanded = expand_env_vars(input).unwrap();
        assert_eq!(expanded, "Value: hello!");
    }

    #[test]
    fn test_expand_env_vars_braces() {
        unsafe { env::set_var("TEST_EXPAND2", "world"); }
        let input = "Value: ${TEST_EXPAND2}!";
        let expanded = expand_env_vars(input).unwrap();
        assert_eq!(expanded, "Value: world!");
    }

    #[test]
    fn test_expand_env_vars_missing() {
        let input = "Value: $NOT_SET!";
        let expanded = expand_env_vars(input).unwrap();
        assert_eq!(expanded, "Value: !");
    }

    #[test]
    fn test_ensure_config_and_load_config() {
        let tmp_dir = tempfile::tempdir().unwrap();
        unsafe { env::set_var("XDG_CONFIG_HOME", tmp_dir.path()); }
        ensure_config().unwrap();
        let path = get_config_file_path();
        assert!(path.exists());
        let cfg = load_config().unwrap();
        assert!(cfg.mcp_servers.is_empty());
    }
}
