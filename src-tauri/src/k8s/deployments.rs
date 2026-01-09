use k8s_openapi::api::apps::v1::Deployment;
use kube::api::ListParams;
use kube::Api;

use crate::error::K8sError;
use crate::k8s::client::get_client_for_context;
use crate::models::DeploymentInfo;

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
