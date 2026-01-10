use std::collections::BTreeMap;

use k8s_openapi::api::apps::v1::Deployment;
use kube::api::ListParams;
use kube::Api;

use crate::error::K8sError;
use crate::k8s::client::get_client_for_context;
use crate::models::{DeploymentInfo, DeploymentDetails, DeploymentCondition};

#[tauri::command]
pub async fn get_deployments(
    context: String,
    namespace: String,
) -> Result<Vec<DeploymentInfo>, K8sError> {
    let client = get_client_for_context(&context).await?;
    let deployments: Api<Deployment> = Api::namespaced(client, &namespace);

    let deploy_list = deployments.list(&ListParams::default()).await?;

    let result: Vec<DeploymentInfo> = deploy_list
        .items
        .iter()
        .map(|d| {
            let name = d.metadata.name.clone().unwrap_or_default();
            let namespace = d.metadata.namespace.clone().unwrap_or_default();

            let spec = d.spec.as_ref();
            let status = d.status.as_ref();

            let replicas = spec.and_then(|s| s.replicas).unwrap_or(0);
            let available_replicas = status.and_then(|s| s.available_replicas).unwrap_or(0);
            let ready_replicas = status.and_then(|s| s.ready_replicas).unwrap_or(0);

            DeploymentInfo {
                name,
                namespace,
                replicas,
                available_replicas,
                ready_replicas,
            }
        })
        .collect();

    Ok(result)
}

#[tauri::command]
pub async fn get_deployment_details(
    context: String,
    namespace: String,
    deployment_name: String,
) -> Result<DeploymentDetails, K8sError> {
    let client = get_client_for_context(&context).await?;
    let deployments: Api<Deployment> = Api::namespaced(client, &namespace);

    let deployment = deployments.get(&deployment_name).await?;

    let metadata = &deployment.metadata;
    let spec = deployment.spec.as_ref();
    let status = deployment.status.as_ref();

    let name = metadata.name.clone().unwrap_or_default();
    let namespace = metadata.namespace.clone().unwrap_or_default();
    let creation_timestamp = metadata
        .creation_timestamp
        .as_ref()
        .map(|t| t.0.to_rfc3339());

    let labels: BTreeMap<String, String> = metadata.labels.clone().unwrap_or_default();
    let annotations: BTreeMap<String, String> = metadata.annotations.clone().unwrap_or_default();

    let replicas = spec.and_then(|s| s.replicas).unwrap_or(0);
    let strategy = spec
        .and_then(|s| s.strategy.as_ref())
        .and_then(|s| s.type_.clone())
        .unwrap_or_else(|| "RollingUpdate".to_string());
    let min_ready_seconds = spec.and_then(|s| s.min_ready_seconds).unwrap_or(0);
    let revision_history_limit = spec.and_then(|s| s.revision_history_limit).unwrap_or(10);

    let selector: BTreeMap<String, String> = spec
        .and_then(|s| s.selector.match_labels.clone())
        .unwrap_or_default();

    let available_replicas = status.and_then(|s| s.available_replicas).unwrap_or(0);
    let ready_replicas = status.and_then(|s| s.ready_replicas).unwrap_or(0);
    let updated_replicas = status.and_then(|s| s.updated_replicas).unwrap_or(0);

    let conditions: Vec<DeploymentCondition> = status
        .and_then(|s| s.conditions.as_ref())
        .map(|conds| {
            conds
                .iter()
                .map(|c| DeploymentCondition {
                    condition_type: c.type_.clone(),
                    status: c.status.clone(),
                    reason: c.reason.clone(),
                    message: c.message.clone(),
                    last_update_time: c.last_update_time.as_ref().map(|t| t.0.to_rfc3339()),
                    last_transition_time: c.last_transition_time.as_ref().map(|t| t.0.to_rfc3339()),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(DeploymentDetails {
        name,
        namespace,
        replicas,
        available_replicas,
        ready_replicas,
        updated_replicas,
        strategy,
        min_ready_seconds,
        revision_history_limit,
        creation_timestamp,
        labels,
        annotations,
        selector,
        conditions,
    })
}
