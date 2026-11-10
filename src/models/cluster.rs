use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClusterInfo {
    pub name: String,
    pub server: String,
    pub is_current: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NamespaceInfo {
    pub name: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeploymentInfo {
    pub name: String,
    pub namespace: String,
    pub replicas: i32,
    pub available_replicas: i32,
    pub ready_replicas: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeploymentDetails {
    pub name: String,
    pub namespace: String,
    pub replicas: i32,
    pub available_replicas: i32,
    pub ready_replicas: i32,
    pub updated_replicas: i32,
    pub strategy: String,
    pub min_ready_seconds: i32,
    pub revision_history_limit: i32,
    pub creation_timestamp: Option<String>,
    pub labels: std::collections::BTreeMap<String, String>,
    pub annotations: std::collections::BTreeMap<String, String>,
    pub selector: std::collections::BTreeMap<String, String>,
    pub conditions: Vec<DeploymentCondition>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeploymentCondition {
    pub condition_type: String,
    pub status: String,
    pub reason: Option<String>,
    pub message: Option<String>,
    pub last_update_time: Option<String>,
    pub last_transition_time: Option<String>,
}
