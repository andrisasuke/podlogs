mod error;
mod k8s;
mod models;

use k8s::{clusters, deployments, logs, namespaces, pods};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            clusters::get_clusters,
            namespaces::get_namespaces,
            deployments::get_deployments,
            pods::get_pods,
            pods::get_pod_details,
            logs::get_pod_logs,
            logs::search_deployment_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
