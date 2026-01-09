use crate::error::K8sError;
use crate::k8s::client::load_kubeconfig;
use crate::models::ClusterInfo;

#[tauri::command]
pub async fn get_clusters() -> Result<Vec<ClusterInfo>, K8sError> {
    let kubeconfig = load_kubeconfig()?;
    let current_context = kubeconfig.current_context.clone();

    let clusters: Vec<ClusterInfo> = kubeconfig
        .contexts
        .iter()
        .filter_map(|ctx| {
            let name = ctx.name.clone();
            let cluster_name = ctx.context.as_ref()?.cluster.clone();

            // Find the cluster to get the server URL
            let server = kubeconfig
                .clusters
                .iter()
                .find(|c| c.name == cluster_name)
                .and_then(|c| c.cluster.as_ref())
                .map(|c| c.server.clone().unwrap_or_default())
                .unwrap_or_default();

            Some(ClusterInfo {
                name: name.clone(),
                server,
                is_current: Some(&name) == current_context.as_ref(),
            })
        })
        .collect();

    Ok(clusters)
}
