use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PodInfo {
    pub name: String,
    pub namespace: String,
    pub status: String,
    pub ready: String,
    pub restarts: i32,
    pub age: String,
    pub ip: String,
    pub node: String,
    pub containers: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PodDetails {
    pub name: String,
    pub namespace: String,
    pub status: String,
    pub node: String,
    pub ip: String,
    pub start_time: Option<String>,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
    pub containers: Vec<ContainerDetails>,
    pub conditions: Vec<PodCondition>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContainerDetails {
    pub name: String,
    pub image: String,
    pub ready: bool,
    pub restart_count: i32,
    pub state: String,
    pub env_vars: Vec<EnvVar>,
    pub ports: Vec<ContainerPort>,
    pub resources: ResourceRequirements,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContainerPort {
    pub name: Option<String>,
    pub container_port: i32,
    pub protocol: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResourceRequirements {
    pub cpu_request: Option<String>,
    pub cpu_limit: Option<String>,
    pub memory_request: Option<String>,
    pub memory_limit: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PodCondition {
    pub condition_type: String,
    pub status: String,
    pub reason: Option<String>,
    pub message: Option<String>,
    pub last_transition_time: Option<String>,
}
