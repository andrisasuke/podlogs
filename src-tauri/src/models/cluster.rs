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
