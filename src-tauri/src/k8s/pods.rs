use chrono::Utc;
use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::Pod;
use kube::api::ListParams;
use kube::Api;
use std::collections::HashMap;

use crate::error::K8sError;
use crate::k8s::client::get_client_for_context;
use crate::models::{
    ContainerDetails, ContainerPort, EnvVar, PodCondition, PodDetails, PodInfo,
    ResourceRequirements,
};

#[tauri::command]
pub async fn get_pods(
    context: String,
    namespace: String,
    deployment: Option<String>,
) -> Result<Vec<PodInfo>, K8sError> {
    let client = get_client_for_context(&context).await?;
    let pods: Api<Pod> = Api::namespaced(client.clone(), &namespace);

    let list_params = if let Some(deploy_name) = deployment {
        // Get the deployment to find its label selector
        let deployments: Api<Deployment> = Api::namespaced(client, &namespace);
        let deploy = deployments.get(&deploy_name).await?;

        let selector = deploy
            .spec
            .as_ref()
            .and_then(|s| s.selector.match_labels.as_ref())
            .map(|labels| {
                labels
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .unwrap_or_default();

        ListParams::default().labels(&selector)
    } else {
        ListParams::default()
    };

    let pod_list = pods.list(&list_params).await?;

    let result: Vec<PodInfo> = pod_list
        .items
        .iter()
        .map(|pod| {
            let name = pod.metadata.name.clone().unwrap_or_default();
            let namespace = pod.metadata.namespace.clone().unwrap_or_default();

            let status = get_pod_status(pod);
            let (ready_count, total_count) = get_ready_count(pod);
            let restarts = get_restart_count(pod);
            let age = get_age(pod);
            let ip = pod
                .status
                .as_ref()
                .and_then(|s| s.pod_ip.clone())
                .unwrap_or_default();
            let node = pod
                .spec
                .as_ref()
                .and_then(|s| s.node_name.clone())
                .unwrap_or_default();
            let containers = pod
                .spec
                .as_ref()
                .map(|s| s.containers.iter().map(|c| c.name.clone()).collect())
                .unwrap_or_default();

            PodInfo {
                name,
                namespace,
                status,
                ready: format!("{}/{}", ready_count, total_count),
                restarts,
                age,
                ip,
                node,
                containers,
            }
        })
        .collect();

    Ok(result)
}

#[tauri::command]
pub async fn get_pod_details(
    context: String,
    namespace: String,
    pod_name: String,
) -> Result<PodDetails, K8sError> {
    let client = get_client_for_context(&context).await?;
    let pods: Api<Pod> = Api::namespaced(client, &namespace);

    let pod = pods.get(&pod_name).await?;

    let name = pod.metadata.name.clone().unwrap_or_default();
    let ns = pod.metadata.namespace.clone().unwrap_or_default();
    let status = get_pod_status(&pod);
    let node = pod
        .spec
        .as_ref()
        .and_then(|s| s.node_name.clone())
        .unwrap_or_default();
    let ip = pod
        .status
        .as_ref()
        .and_then(|s| s.pod_ip.clone())
        .unwrap_or_default();
    let start_time = pod
        .status
        .as_ref()
        .and_then(|s| s.start_time.as_ref())
        .map(|t| t.0.to_rfc3339());

    let labels: HashMap<String, String> = pod.metadata.labels.clone().unwrap_or_default().into_iter().collect();
    let annotations: HashMap<String, String> = pod.metadata.annotations.clone().unwrap_or_default().into_iter().collect();

    // Get container details
    let containers = get_container_details(&pod);

    // Get conditions
    let conditions = pod
        .status
        .as_ref()
        .and_then(|s| s.conditions.as_ref())
        .map(|conds| {
            conds
                .iter()
                .map(|c| PodCondition {
                    condition_type: c.type_.clone(),
                    status: c.status.clone(),
                    reason: c.reason.clone(),
                    message: c.message.clone(),
                    last_transition_time: c.last_transition_time.as_ref().map(|t| t.0.to_rfc3339()),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(PodDetails {
        name,
        namespace: ns,
        status,
        node,
        ip,
        start_time,
        labels,
        annotations,
        containers,
        conditions,
    })
}

fn get_pod_status(pod: &Pod) -> String {
    // Check if pod is being deleted (has deletion timestamp)
    if pod.metadata.deletion_timestamp.is_some() {
        return "Terminating".to_string();
    }

    let status = pod.status.as_ref();

    // Check container statuses first for specific states
    if let Some(container_statuses) = status.and_then(|s| s.container_statuses.as_ref()) {
        for cs in container_statuses {
            if let Some(state) = &cs.state {
                if let Some(waiting) = &state.waiting {
                    if let Some(reason) = &waiting.reason {
                        if reason == "CrashLoopBackOff"
                            || reason == "ImagePullBackOff"
                            || reason == "ErrImagePull"
                            || reason == "ContainerCreating"
                            || reason == "ContainerStatusUnknown" {
                            return reason.clone();
                        }
                    }
                }
                // Check terminated state for ContainerStatusUnknown
                if let Some(terminated) = &state.terminated {
                    if let Some(reason) = &terminated.reason {
                        if reason == "ContainerStatusUnknown" {
                            return reason.clone();
                        }
                    }
                }
            }
        }
    }

    // Check pod reason (for Evicted, UnexpectedAdmissionError, etc.)
    if let Some(reason) = status.and_then(|s| s.reason.as_ref()) {
        if reason == "Evicted" || reason == "UnexpectedAdmissionError" {
            return reason.clone();
        }
    }

    // Fall back to phase
    status
        .and_then(|s| s.phase.clone())
        .unwrap_or_else(|| "Unknown".to_string())
}

fn get_ready_count(pod: &Pod) -> (i32, i32) {
    let containers = pod
        .spec
        .as_ref()
        .map(|s| s.containers.len() as i32)
        .unwrap_or(0);

    let ready = pod
        .status
        .as_ref()
        .and_then(|s| s.container_statuses.as_ref())
        .map(|cs| cs.iter().filter(|c| c.ready).count() as i32)
        .unwrap_or(0);

    (ready, containers)
}

fn get_restart_count(pod: &Pod) -> i32 {
    pod.status
        .as_ref()
        .and_then(|s| s.container_statuses.as_ref())
        .map(|cs| cs.iter().map(|c| c.restart_count).sum())
        .unwrap_or(0)
}

fn get_age(pod: &Pod) -> String {
    let created = pod
        .metadata
        .creation_timestamp
        .as_ref()
        .map(|t| t.0)
        .unwrap_or_else(Utc::now);

    let duration = Utc::now().signed_duration_since(created);

    if duration.num_days() > 0 {
        format!("{}d", duration.num_days())
    } else if duration.num_hours() > 0 {
        format!("{}h", duration.num_hours())
    } else if duration.num_minutes() > 0 {
        format!("{}m", duration.num_minutes())
    } else {
        format!("{}s", duration.num_seconds())
    }
}

fn get_container_details(pod: &Pod) -> Vec<ContainerDetails> {
    let spec = match &pod.spec {
        Some(s) => s,
        None => return vec![],
    };

    let status_map: HashMap<String, _> = pod
        .status
        .as_ref()
        .and_then(|s| s.container_statuses.as_ref())
        .map(|statuses| {
            statuses
                .iter()
                .map(|cs| (cs.name.clone(), cs.clone()))
                .collect()
        })
        .unwrap_or_default();

    spec.containers
        .iter()
        .map(|container| {
            let cs = status_map.get(&container.name);

            let state = cs
                .and_then(|s| s.state.as_ref())
                .map(|state| {
                    if state.running.is_some() {
                        "Running".to_string()
                    } else if let Some(waiting) = &state.waiting {
                        waiting.reason.clone().unwrap_or_else(|| "Waiting".to_string())
                    } else if let Some(terminated) = &state.terminated {
                        terminated
                            .reason
                            .clone()
                            .unwrap_or_else(|| "Terminated".to_string())
                    } else {
                        "Unknown".to_string()
                    }
                })
                .unwrap_or_else(|| "Unknown".to_string());

            // Collect env vars from 'env' field
            let mut env_vars: Vec<EnvVar> = container
                .env
                .as_ref()
                .map(|envs| {
                    envs.iter()
                        .map(|e| {
                            let (source, value) = if let Some(vf) = &e.value_from {
                                if let Some(secret_ref) = &vf.secret_key_ref {
                                    ("Secret", format!("{}:{}", secret_ref.name, secret_ref.key))
                                } else if let Some(cm_ref) = &vf.config_map_key_ref {
                                    ("ConfigMap", format!("{}:{}", cm_ref.name, cm_ref.key))
                                } else if let Some(field_ref) = &vf.field_ref {
                                    ("FieldRef", field_ref.field_path.clone())
                                } else if let Some(resource_ref) = &vf.resource_field_ref {
                                    ("ResourceRef", resource_ref.resource.clone())
                                } else {
                                    ("Reference", String::new())
                                }
                            } else {
                                ("Direct", e.value.clone().unwrap_or_default())
                            };

                            EnvVar {
                                name: e.name.clone(),
                                value,
                                source: source.to_string(),
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();

            // Collect env vars from 'envFrom' field (ConfigMaps and Secrets)
            if let Some(env_from) = &container.env_from {
                for ef in env_from {
                    if let Some(cm_ref) = &ef.config_map_ref {
                        let prefix = ef.prefix.clone().unwrap_or_default();
                        env_vars.push(EnvVar {
                            name: format!("{}* (all keys)", prefix),
                            value: format!("from ConfigMap: {}", cm_ref.name),
                            source: "ConfigMap".to_string(),
                        });
                    }
                    if let Some(secret_ref) = &ef.secret_ref {
                        let prefix = ef.prefix.clone().unwrap_or_default();
                        env_vars.push(EnvVar {
                            name: format!("{}* (all keys)", prefix),
                            value: format!("from Secret: {}", secret_ref.name),
                            source: "Secret".to_string(),
                        });
                    }
                }
            }

            let ports = container
                .ports
                .as_ref()
                .map(|ps| {
                    ps.iter()
                        .map(|p| ContainerPort {
                            name: p.name.clone(),
                            container_port: p.container_port,
                            protocol: p.protocol.clone().unwrap_or_else(|| "TCP".to_string()),
                        })
                        .collect()
                })
                .unwrap_or_default();

            let resources = container.resources.as_ref().map_or_else(
                || ResourceRequirements {
                    cpu_request: None,
                    cpu_limit: None,
                    memory_request: None,
                    memory_limit: None,
                },
                |r| ResourceRequirements {
                    cpu_request: r.requests.as_ref().and_then(|req| {
                        req.get("cpu").map(|q| q.0.clone())
                    }),
                    cpu_limit: r.limits.as_ref().and_then(|lim| {
                        lim.get("cpu").map(|q| q.0.clone())
                    }),
                    memory_request: r.requests.as_ref().and_then(|req| {
                        req.get("memory").map(|q| q.0.clone())
                    }),
                    memory_limit: r.limits.as_ref().and_then(|lim| {
                        lim.get("memory").map(|q| q.0.clone())
                    }),
                },
            );

            ContainerDetails {
                name: container.name.clone(),
                image: container.image.clone().unwrap_or_default(),
                ready: cs.map(|s| s.ready).unwrap_or(false),
                restart_count: cs.map(|s| s.restart_count).unwrap_or(0),
                state,
                env_vars,
                ports,
                resources,
            }
        })
        .collect()
}
