use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct McpServersConfig {
    #[serde(rename = "mcpServers")]
    pub mcp_servers: HashMap<String, McpServerConfig>,
    #[serde(rename = "environments")]
    pub environments: HashMap<String, EnvironmentConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpServerConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    #[serde(rename = "configPath")]
    pub config_path: String,
    pub enable: Option<Vec<String>>,
    pub preset: Option<HashMap<String, Vec<String>>>,
    pub mode: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClaudeDesktopConfig {
    #[serde(rename = "mcpServers")]
    pub mcp_servers: HashMap<String, McpServerConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_serialize_deserialize_mcpserversconfig() {
        let json = r#"{
            \"mcpServers\": {
                \"test\": {\"command\": \"echo\", \"args\": [\"hi\"], \"env\": {\"A\": \"B\"}}
            },
            \"environments\": {
                \"env1\": {\"configPath\": \"/tmp/test.json\", \"enable\": [\"test\"], \"preset\": {\"p1\": [\"test\"]}, \"mode\": \"testmode\"}
            }
        }"#;
        let cfg: McpServersConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.mcp_servers.len(), 1);
        assert_eq!(cfg.environments.len(), 1);
        let out = serde_json::to_string(&cfg).unwrap();
        assert!(out.contains("mcpServers"));
    }
}