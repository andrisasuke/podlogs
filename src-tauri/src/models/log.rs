use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogEntry {
    pub timestamp: Option<String>,
    pub level: Option<String>,
    pub message: String,
    pub raw: String,
    pub is_json: bool,
    pub pod_name: String,
    pub container_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogSearchResult {
    pub pod_name: String,
    pub container_name: String,
    pub total_matches: i32,
    pub entries: Vec<LogEntry>,
}
