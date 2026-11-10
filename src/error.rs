use thiserror::Error;

#[derive(Error, Debug)]
pub enum K8sError {
    #[error("Kubernetes error: {0}")]
    Kube(#[from] kube::Error),

    #[error("Config error: {0}")]
    Config(#[from] kube::config::KubeconfigError),

    #[error("Infer config error: {0}")]
    InferConfig(#[from] kube::config::InferConfigError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("No kubeconfig found")]
    NoKubeconfig,
}

impl serde::Serialize for K8sError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
