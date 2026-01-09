use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::Pod;
use kube::api::{ListParams, LogParams};
use kube::Api;

use crate::error::K8sError;
use crate::k8s::client::get_client_for_context;
use crate::models::{LogEntry, LogSearchResult};

#[tauri::command]
pub async fn get_pod_logs(
    context: String,
    namespace: String,
    pod_name: String,
    container: Option<String>,
    since_seconds: Option<i64>,
    tail_lines: Option<i64>,
) -> Result<Vec<LogEntry>, K8sError> {
    let client = get_client_for_context(&context).await?;
    let pods: Api<Pod> = Api::namespaced(client.clone(), &namespace);

    // Get the pod to find container name if not specified
    let pod = pods.get(&pod_name).await?;
    let container_name = container.or_else(|| {
        pod.spec
            .as_ref()
            .and_then(|s| s.containers.first())
            .map(|c| c.name.clone())
    });

    let mut params = LogParams {
        timestamps: true,
        ..Default::default()
    };

    if let Some(c) = &container_name {
        params.container = Some(c.clone());
    }

    if let Some(since) = since_seconds {
        params.since_seconds = Some(since);
    }

    if let Some(tail) = tail_lines {
        params.tail_lines = Some(tail);
    }

    let logs = pods.logs(&pod_name, &params).await?;
    let container_name = container_name.unwrap_or_default();

    let entries: Vec<LogEntry> = logs
        .lines()
        .map(|line| parse_log_line(line, &pod_name, &container_name))
        .collect();

    Ok(entries)
}

#[tauri::command]
pub async fn search_deployment_logs(
    context: String,
    namespace: String,
    deployment: String,
    keyword: Option<String>,
    log_level: Option<String>,
    since_seconds: Option<i64>,
) -> Result<Vec<LogSearchResult>, K8sError> {
    let client = get_client_for_context(&context).await?;

    // Get the deployment to find its label selector
    let deployments: Api<Deployment> = Api::namespaced(client.clone(), &namespace);
    let deploy = deployments.get(&deployment).await?;

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

    // Get all pods for this deployment
    let pods: Api<Pod> = Api::namespaced(client.clone(), &namespace);
    let pod_list = pods.list(&ListParams::default().labels(&selector)).await?;

    let mut results: Vec<LogSearchResult> = Vec::new();

    for pod in pod_list.items {
        let pod_name = pod.metadata.name.clone().unwrap_or_default();

        // Get containers
        let containers: Vec<String> = pod
            .spec
            .as_ref()
            .map(|s| s.containers.iter().map(|c| c.name.clone()).collect())
            .unwrap_or_default();

        for container_name in containers {
            let mut params = LogParams {
                timestamps: true,
                container: Some(container_name.clone()),
                ..Default::default()
            };

            if let Some(since) = since_seconds {
                params.since_seconds = Some(since);
            }

            // Limit to reasonable number of lines for search
            params.tail_lines = Some(1000);

            match pods.logs(&pod_name, &params).await {
                Ok(logs) => {
                    let entries: Vec<LogEntry> = logs
                        .lines()
                        .map(|line| parse_log_line(line, &pod_name, &container_name))
                        .filter(|entry| {
                            let keyword_match = keyword
                                .as_ref()
                                .map(|kw| {
                                    entry.message.to_lowercase().contains(&kw.to_lowercase())
                                        || entry.raw.to_lowercase().contains(&kw.to_lowercase())
                                })
                                .unwrap_or(true);

                            let level_match = log_level
                                .as_ref()
                                .map(|lv| {
                                    entry
                                        .level
                                        .as_ref()
                                        .map(|l| l.to_uppercase() == lv.to_uppercase())
                                        .unwrap_or(false)
                                })
                                .unwrap_or(true);

                            keyword_match && level_match
                        })
                        .collect();

                    if !entries.is_empty() {
                        results.push(LogSearchResult {
                            pod_name: pod_name.clone(),
                            container_name: container_name.clone(),
                            total_matches: entries.len() as i32,
                            entries,
                        });
                    }
                }
                Err(_) => continue, // Skip pods we can't get logs from
            }
        }
    }

    Ok(results)
}

/// Parse a single log line, detecting JSON vs plain text
fn parse_log_line(line: &str, pod_name: &str, container_name: &str) -> LogEntry {
    // Extract timestamp if present (kubectl adds it with --timestamps)
    let (timestamp, rest) = extract_timestamp(line);

    // Try to parse as JSON
    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(rest) {
        parse_json_log(json_value, timestamp, pod_name, container_name, line)
    } else {
        parse_plain_log(rest, timestamp, pod_name, container_name, line)
    }
}

/// Extract timestamp from the beginning of a log line
fn extract_timestamp(line: &str) -> (Option<String>, &str) {
    // Kubernetes timestamps are in RFC3339 format at the start of the line
    // e.g., "2024-12-26T10:23:45.123456789Z log message"
    if line.len() > 30 && line.chars().nth(4) == Some('-') && line.chars().nth(7) == Some('-') {
        if let Some(space_idx) = line.find(' ') {
            let ts = &line[..space_idx];
            // Validate it looks like a timestamp
            if ts.contains('T') && (ts.ends_with('Z') || ts.contains('+')) {
                return (Some(ts.to_string()), &line[space_idx + 1..]);
            }
        }
    }
    (None, line)
}

/// Parse JSON structured log
fn parse_json_log(
    json: serde_json::Value,
    fallback_timestamp: Option<String>,
    pod_name: &str,
    container_name: &str,
    raw: &str,
) -> LogEntry {
    // Extract level from common fields
    let level = json
        .get("level")
        .or_else(|| json.get("severity"))
        .or_else(|| json.get("log_level"))
        .or_else(|| json.get("lvl"))
        .and_then(|v| v.as_str())
        .map(|s| normalize_log_level(s));

    // Extract message from common fields
    let message = json
        .get("message")
        .or_else(|| json.get("msg"))
        .or_else(|| json.get("log"))
        .and_then(|v| v.as_str())
        .unwrap_or(raw)
        .to_string();

    // Extract timestamp from JSON or use kubectl timestamp
    let timestamp = json
        .get("timestamp")
        .or_else(|| json.get("time"))
        .or_else(|| json.get("ts"))
        .or_else(|| json.get("@timestamp"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or(fallback_timestamp);

    LogEntry {
        timestamp,
        level,
        message,
        raw: raw.to_string(),
        is_json: true,
        pod_name: pod_name.to_string(),
        container_name: container_name.to_string(),
    }
}

/// Parse plain text log (try to detect level from content)
fn parse_plain_log(
    content: &str,
    timestamp: Option<String>,
    pod_name: &str,
    container_name: &str,
    raw: &str,
) -> LogEntry {
    // Try to detect log level from content
    let level = detect_log_level(content);

    LogEntry {
        timestamp,
        level,
        message: content.to_string(),
        raw: raw.to_string(),
        is_json: false,
        pod_name: pod_name.to_string(),
        container_name: container_name.to_string(),
    }
}

/// Normalize log level to standard values
fn normalize_log_level(level: &str) -> String {
    let upper = level.to_uppercase();
    match upper.as_str() {
        "ERROR" | "ERR" | "FATAL" | "CRITICAL" | "CRIT" => "ERROR".to_string(),
        "WARN" | "WARNING" => "WARN".to_string(),
        "INFO" | "INFORMATION" => "INFO".to_string(),
        "DEBUG" | "TRACE" | "VERBOSE" => "DEBUG".to_string(),
        _ => upper,
    }
}

/// Detect log level from plain text content
fn detect_log_level(content: &str) -> Option<String> {
    let upper = content.to_uppercase();

    // Check for common log level patterns
    let patterns = [
        ("[ERROR]", "ERROR"),
        ("[ERR]", "ERROR"),
        ("ERROR:", "ERROR"),
        ("ERROR ", "ERROR"),
        (" ERROR ", "ERROR"),
        ("FATAL:", "ERROR"),
        ("[FATAL]", "ERROR"),
        ("[WARN]", "WARN"),
        ("[WARNING]", "WARN"),
        ("WARN:", "WARN"),
        ("WARNING:", "WARN"),
        (" WARN ", "WARN"),
        ("[INFO]", "INFO"),
        ("INFO:", "INFO"),
        (" INFO ", "INFO"),
        ("[DEBUG]", "DEBUG"),
        ("DEBUG:", "DEBUG"),
        (" DEBUG ", "DEBUG"),
        ("[TRACE]", "DEBUG"),
    ];

    for (pattern, level) in patterns {
        if upper.contains(pattern) {
            return Some(level.to_string());
        }
    }

    None
}
