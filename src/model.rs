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