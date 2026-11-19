use kube::config::{KubeConfigOptions, Kubeconfig};
use kube::{Client, Config};
use std::path::PathBuf;
use std::time::Duration;

use crate::error::K8sError;

/// Connection timeout in seconds
const CONNECTION_TIMEOUT_SECS: u64 = 30;

/// Get path to kubeconfig file
pub fn get_kubeconfig_path() -> Option<PathBuf> {
    // Check KUBECONFIG env var first
    if let Ok(path) = std::env::var("KUBECONFIG") {
        return Some(PathBuf::from(path));
    }
    // Fall back to default location
    dirs::home_dir().map(|h| h.join(".kube").join("config"))
}

/// Load kubeconfig from file
pub fn load_kubeconfig() -> Result<Kubeconfig, K8sError> {
    let path = get_kubeconfig_path().ok_or(K8sError::NoKubeconfig)?;
    Ok(Kubeconfig::read_from(&path)?)
}

/// Create a Kubernetes client for a specific context
pub async fn get_client_for_context(context_name: &str) -> Result<Client, K8sError> {
    let kubeconfig = load_kubeconfig()?;

    let options = KubeConfigOptions {
        context: Some(context_name.to_string()),
        ..Default::default()
    };

    let mut config = Config::from_custom_kubeconfig(kubeconfig, &options).await?;

    // Set connection timeout
    config.connect_timeout = Some(Duration::from_secs(CONNECTION_TIMEOUT_SECS));
    config.read_timeout = Some(Duration::from_secs(CONNECTION_TIMEOUT_SECS));
    config.write_timeout = Some(Duration::from_secs(CONNECTION_TIMEOUT_SECS));

    Ok(Client::try_from(config)?)
}
