//! Pod status → badge variant (`Badge.tsx` getStatusVariant) and deployment health.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BadgeVariant {
    Success,
    Warning,
    Error,
    Terminating,
    Default,
}

pub fn pod_status_variant(status: &str, ready: Option<&str>) -> BadgeVariant {
    if status == "Terminating" {
        return BadgeVariant::Terminating;
    }
    if status == "Running"
        && let Some((r, t)) = ready.filter(|s| !s.is_empty()).and_then(|s| s.split_once('/'))
        && let (Ok(r), Ok(t)) = (r.parse::<f64>(), t.parse::<f64>())
        && r < t
    {
        return BadgeVariant::Warning;
    }
    match status {
        "Running" | "Succeeded" => BadgeVariant::Success,
        "Pending" | "ContainerCreating" => BadgeVariant::Warning,
        "Failed" | "CrashLoopBackOff" | "ImagePullBackOff" | "ErrImagePull" | "Evicted"
        | "UnexpectedAdmissionError" | "ContainerStatusUnknown" => BadgeVariant::Error,
        _ => BadgeVariant::Default,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Health {
    Healthy,
    Degraded,
    Unhealthy,
}

impl Health {
    pub fn label(self) -> &'static str {
        match self {
            Health::Healthy => "Healthy",
            Health::Degraded => "Degraded",
            Health::Unhealthy => "Unhealthy",
        }
    }
}

pub fn health(replicas: i32, available: i32) -> Health {
    if available == replicas {
        Health::Healthy
    } else if available > 0 {
        Health::Degraded
    } else {
        Health::Unhealthy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_values() {
        assert_eq!(pod_status_variant("Running", Some("0/1")), BadgeVariant::Warning);
        assert_eq!(pod_status_variant("Running", Some("1/1")), BadgeVariant::Success);
        assert_eq!(pod_status_variant("Terminating", Some("1/1")), BadgeVariant::Terminating);
        assert_eq!(pod_status_variant("ContainerCreating", None), BadgeVariant::Warning);
        assert_eq!(pod_status_variant("ContainerStatusUnknown", None), BadgeVariant::Error);
        assert_eq!(pod_status_variant("Unknown", None), BadgeVariant::Default);
        assert_eq!(health(0, 0), Health::Healthy);
        assert_eq!(health(3, 1), Health::Degraded);
        assert_eq!(health(3, 0), Health::Unhealthy);
    }
}
