//! ConfigMap environment declaration expansion, kept independent from Kubernetes I/O.
use std::collections::HashMap;

use k8s_openapi::api::core::v1::{ConfigMap, Pod};

use crate::models::{
    ContainerDetails, EnvResolution, EnvResolutionIssue, EnvResolutionReason, EnvVar,
};

pub type ConfigMapLookups = HashMap<String, Result<ConfigMap, EnvResolutionReason>>;

pub fn expand_configmap_env(pod: &Pod, lookups: &ConfigMapLookups) -> Vec<ContainerDetails> {
    let mut containers = crate::k8s::pods::get_container_details(pod);
    let Some(spec) = &pod.spec else { return containers };
    for (details, container) in containers.iter_mut().zip(&spec.containers) {
        if let Some(envs) = &container.env {
            for (row, declaration) in details.env_vars.iter_mut().zip(envs) {
                let Some(selector) = declaration.value_from.as_ref().and_then(|v| v.config_map_key_ref.as_ref()) else { continue };
                let reference = format!("{}:{}", selector.name, selector.key);
                row.reference = Some(reference);
                row.resolution = match lookups.get(&selector.name) {
                    Some(Ok(cm)) => match cm.data.as_ref().and_then(|data| data.get(&selector.key)) {
                        Some(value) => { row.value = value.clone(); EnvResolution::ConfigMapResolved }
                        None => {
                            details.env_issues.push(EnvResolutionIssue { source: selector.name.clone(), key: Some(selector.key.clone()), optional: selector.optional.unwrap_or(false), reason: EnvResolutionReason::MissingKey });
                            EnvResolution::Unavailable
                        }
                    },
                    Some(Err(reason)) => {
                        details.env_issues.push(EnvResolutionIssue { source: selector.name.clone(), key: Some(selector.key.clone()), optional: selector.optional.unwrap_or(false), reason: *reason });
                        EnvResolution::Unavailable
                    }
                    None => EnvResolution::Unavailable,
                };
            }
        }
        if let Some(env_from) = &container.env_from {
            for from in env_from {
                let prefix = from.prefix.as_deref().unwrap_or_default();
                if let Some(selector) = &from.config_map_ref {
                    match lookups.get(&selector.name) {
                        Some(Ok(cm)) => {
                            if let Some(data) = &cm.data {
                                let mut entries: Vec<_> = data.iter().collect();
                                entries.sort_by(|a,b| a.0.cmp(b.0));
                                details.env_vars.extend(entries.into_iter().map(|(key,value)| EnvVar {
                                    name: format!("{prefix}{key}"), value: value.clone(), source: "ConfigMap".into(),
                                    reference: Some(format!("{}:{key}", selector.name)), resolution: EnvResolution::ConfigMapResolved,
                                }));
                            }
                        }
                        Some(Err(reason)) => details.env_issues.push(EnvResolutionIssue {
                            source: selector.name.clone(), key: None, optional: selector.optional.unwrap_or(false), reason: *reason,
                        }),
                        None => {}
                    }
                } else if let Some(selector) = &from.secret_ref {
                    details.env_vars.push(EnvVar {
                        name: format!("{prefix}* (all keys)"), value: format!("from Secret: {}", selector.name),
                        source: "Secret".into(), reference: Some(selector.name.clone()), resolution: EnvResolution::Declared,
                    });
                }
            }
        }
    }
    containers
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn pod(value: serde_json::Value) -> Pod { serde_json::from_value(value).unwrap() }
    fn config(data: serde_json::Value) -> ConfigMap { serde_json::from_value(json!({"apiVersion":"v1","kind":"ConfigMap","data":data})).unwrap() }

    #[test]
    fn expands_configmap_sources_in_declaration_order_and_keeps_duplicates() {
        let p = pod(json!({"spec":{"containers":[{"name":"app","image":"x","envFrom":[
            {"prefix":"A_","configMapRef":{"name":"cfg"}}, {"prefix":"A_","configMapRef":{"name":"cfg"}}, {"prefix":"S_","secretRef":{"name":"secret"}},
            {"prefix":"B_","configMapRef":{"name":"cfg"}}],"env":[{"name":"PORT_ALIAS","valueFrom":{"configMapKeyRef":{"name":"cfg","key":"port"}}}]}]}}));
        let maps = HashMap::from([("cfg".into(), Ok(config(json!({"port":"","application.yaml":"a\nb","dot.key":"v"}))))]);
        let result = expand_configmap_env(&p, &maps);
        let vars = &result[0].env_vars;
        assert_eq!(vars.iter().map(|v|v.name.as_str()).collect::<Vec<_>>(), ["PORT_ALIAS","A_application.yaml","A_dot.key","A_port","A_application.yaml","A_dot.key","A_port","S_* (all keys)","B_application.yaml","B_dot.key","B_port"]);
        assert_eq!(vars[0].value, "");
        assert_eq!(vars[0].reference.as_deref(), Some("cfg:port"));
        assert_eq!(vars[1].value, "a\nb");
        assert_eq!(vars[8].reference.as_deref(), Some("cfg:application.yaml"));
        assert_eq!(vars[0].resolution, EnvResolution::ConfigMapResolved);
        assert!(vars.iter().any(|v| v.name == "S_* (all keys)" && v.source == "Secret"));
    }

    #[test]
    fn reports_missing_and_unavailable_configmap_keys_without_faking_values() {
        let p = pod(json!({"spec":{"containers":[{"name":"app","image":"x","env":[
            {"name":"MISSING","valueFrom":{"configMapKeyRef":{"name":"cfg","key":"absent"}}},
            {"name":"OPTIONAL","valueFrom":{"configMapKeyRef":{"name":"gone","key":"x","optional":true}}}],
            "envFrom":[{"configMapRef":{"name":"forbidden"}}]}]}}));
        let maps = HashMap::from([
            ("cfg".into(), Ok(config(json!({"present":"yes"})))),
            ("gone".into(), Err(EnvResolutionReason::NotFound)),
            ("forbidden".into(), Err(EnvResolutionReason::Forbidden)),
        ]);
        let details = &expand_configmap_env(&p, &maps)[0];
        assert_eq!(details.env_vars[0].resolution, EnvResolution::Unavailable);
        assert_eq!(details.env_vars[1].reference.as_deref(), Some("gone:x"));
        assert_eq!(details.env_issues.len(), 3);
        assert!(details.env_issues.iter().any(|i| i.reason == EnvResolutionReason::MissingKey));
        assert!(details.env_issues.iter().any(|i| i.optional && i.reason == EnvResolutionReason::NotFound));
        assert!(details.env_issues.iter().any(|i| i.reason == EnvResolutionReason::Forbidden));
    }

    #[test]
    fn old_container_fixture_deserializes_with_defaults() {
        let c: ContainerDetails = serde_json::from_value(json!({"name":"app","image":"x","ready":true,"restart_count":0,"state":"Running","env_vars":[{"name":"A","value":"v","source":"Direct"}],"ports":[],"resources":{"cpu_request":null,"cpu_limit":null,"memory_request":null,"memory_limit":null}})).unwrap();
        assert!(c.env_issues.is_empty());
        assert_eq!(c.env_vars[0].resolution, EnvResolution::Declared);
        assert_eq!(c.env_vars[0].reference, None);
    }

    #[test]
    fn null_or_empty_data_succeeds_and_binary_data_is_never_decoded() {
        let p = pod(json!({"spec":{"containers":[{"name":"app","image":"x","envFrom":[{"configMapRef":{"name":"empty"}},{"configMapRef":{"name":"binary"}}]}]}}));
        let maps = HashMap::from([
            ("empty".into(), Ok(config(json!(null)))),
            ("binary".into(), Ok(serde_json::from_value(json!({"apiVersion":"v1","kind":"ConfigMap","binaryData":{"B":"YmluYXJ5"}})).unwrap())),
        ]);
        let details = &expand_configmap_env(&p, &maps)[0];
        assert!(details.env_vars.is_empty());
        assert!(details.env_issues.is_empty());
    }

    #[test]
    fn direct_field_resource_and_secret_declarations_keep_their_existing_sources() {
        let p = pod(json!({"spec":{"containers":[{"name":"app","image":"x","env":[
            {"name":"DIRECT","value":"literal"},
            {"name":"FIELD","valueFrom":{"fieldRef":{"fieldPath":"metadata.name"}}},
            {"name":"RESOURCE","valueFrom":{"resourceFieldRef":{"containerName":"app","resource":"limits.cpu"}}},
            {"name":"SECRET","valueFrom":{"secretKeyRef":{"name":"credentials","key":"token"}}}
        ],"envFrom":[{"prefix":"S_","secretRef":{"name":"bundle"}}]}]}}));
        let details = &expand_configmap_env(&p, &HashMap::new())[0];
        assert_eq!(details.env_vars.iter().map(|v|v.source.as_str()).collect::<Vec<_>>(), ["Direct","FieldRef","ResourceRef","Secret","Secret"]);
        assert_eq!(details.env_vars[0].value, "literal");
        assert_eq!(details.env_vars[1].value, "metadata.name");
        assert_eq!(details.env_vars[2].value, "limits.cpu");
        assert_eq!(details.env_vars[3].value, "<secret>");
        assert_eq!(details.env_vars[4].value, "from Secret: bundle");
        assert!(details.env_vars.iter().all(|env| env.resolution == EnvResolution::Declared));
        assert_eq!(details.env_vars[0].reference, None);
        assert_eq!(details.env_vars[3].reference.as_deref(), Some("credentials:token"));
        assert_eq!(details.env_vars[4].reference.as_deref(), Some("bundle"));
    }
}
