//! Connection Error dialog. This task only has the message mapping (`ErrorDialog.tsx` ConnectionError).

/// Maps a raw backend error to (message, details), checked in the React order.
pub fn describe_connection_error(raw: &str) -> (String, String) {
    let has = |s: &str| raw.contains(s);
    let (msg, details) = if has("No kubeconfig found") {
        (
            "Kubernetes configuration not found",
            "Please ensure kubectl is configured and ~/.kube/config exists.".to_string(),
        )
    } else if has("connection refused") || has("ECONNREFUSED") {
        (
            "Cannot reach Kubernetes cluster",
            "The cluster may be offline or unreachable. Check your network connection and cluster status."
                .to_string(),
        )
    } else if has("Unauthorized") || has("401") {
        (
            "Authentication failed",
            "Your Kubernetes credentials may have expired. Try running: kubectl get pods".to_string(),
        )
    } else if has("Forbidden") || has("403") {
        (
            "Access denied",
            "You do not have permission to access this resource. Check your RBAC settings.".to_string(),
        )
    } else if has("timeout") || has("ETIMEDOUT") || has("timed out") {
        (
            "Connection timed out",
            "The cluster took too long to respond. Check your network connection.".to_string(),
        )
    } else if has("certificate") {
        (
            "Certificate error",
            format!("There was a problem with the cluster certificate. {raw}"),
        )
    } else {
        ("Failed to connect to Kubernetes cluster", raw.to_string())
    };
    (msg.to_string(), details)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_values() {
        assert_eq!(describe_connection_error("No kubeconfig found").0, "Kubernetes configuration not found");
        assert_eq!(describe_connection_error("Connection timed out after 30 seconds").0, "Connection timed out");
        assert_eq!(describe_connection_error("Kubernetes error: ApiError: Unauthorized (401)").0, "Authentication failed");
        assert_eq!(describe_connection_error("Kubernetes error: Forbidden").0, "Access denied");
        assert_eq!(describe_connection_error("weird").0, "Failed to connect to Kubernetes cluster");
        assert_eq!(describe_connection_error("weird").1, "weird");
    }
}
