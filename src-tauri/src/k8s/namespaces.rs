use k8s_openapi::api::core::v1::Namespace;
use kube::api::ListParams;
use kube::Api;

use crate::error::K8sError;
use crate::k8s::client::get_client_for_context;
use crate::models::NamespaceInfo;

#[tauri::command]
pub async fn get_namespaces(context: String) -> Result<Vec<NamespaceInfo>, K8sError> {
    let client = get_client_for_context(&context).await?;
    let namespaces: Api<Namespace> = Api::all(client);

    let ns_list = namespaces.list(&ListParams::default()).await?;

    let result: Vec<NamespaceInfo> = ns_list
        .items
        .iter()
        .map(|ns| {
            let name = ns.metadata.name.clone().unwrap_or_default();
            let status = ns
                .status
                .as_ref()
                .and_then(|s| s.phase.clone())
                .unwrap_or_else(|| "Unknown".to_string());

            NamespaceInfo { name, status }
        })
        .collect();

    Ok(result)
}
