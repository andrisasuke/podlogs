use super::pods::{get_pod_details_with_client, resolve_configmaps};
use http::{Request, Response};
use kube::client::{Body, Client};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use tower::service_fn;

fn client(config: Arc<Mutex<Value>>) -> (Client, Arc<Mutex<Vec<String>>>) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let seen = requests.clone();
    let client = Client::new(service_fn(move |request: Request<Body>| {
        let seen = seen.clone();
        let config = config.clone();
        async move {
            let path = request.uri().path().to_string();
            seen.lock().unwrap().push(path.clone());
            let (status, payload) = if path == "/api/v1/namespaces/fixture/pods/app" {
                (200, json!({"apiVersion":"v1","kind":"Pod","metadata":{"name":"app","namespace":"fixture"},"spec":{"containers":[
                    {"name":"one","image":"x","envFrom":[{"configMapRef":{"name":"shared"}},{"configMapRef":{"name":"denied"}}]},
                    {"name":"two","image":"x","env":[{"name":"ALIAS","valueFrom":{"configMapKeyRef":{"name":"shared","key":"A"}}},{"name":"OPTIONAL","valueFrom":{"configMapKeyRef":{"name":"missing","key":"X","optional":true}}}]}
                ]}}))
            } else if path == "/api/v1/namespaces/fixture/configmaps/shared" {
                (200, config.lock().unwrap().clone())
            } else if path == "/api/v1/namespaces/fixture/configmaps/denied" {
                (403, json!({"apiVersion":"v1","kind":"Status","status":"Failure","code":403,"reason":"Forbidden","message":"denied"}))
            } else if path == "/api/v1/namespaces/fixture/configmaps/missing" {
                (404, json!({"apiVersion":"v1","kind":"Status","status":"Failure","code":404,"reason":"NotFound","message":"missing"}))
            } else {
                panic!("unexpected Kubernetes request: {path}")
            };
            Ok::<_, std::io::Error>(Response::builder().status(status).body(Body::from(payload.to_string().into_bytes())).unwrap())
        }
    }), "fixture");
    (client, requests)
}

#[tokio::test]
async fn resolves_only_referenced_configmaps_once_per_request_and_keeps_partial_data() {
    let (client, requests) = client(Arc::new(Mutex::new(json!({"apiVersion":"v1","kind":"ConfigMap","metadata":{"name":"shared"},"data":{"A":"one","B":"two"}}))));
    let details = get_pod_details_with_client(client, "fixture", "app").await.unwrap();
    assert_eq!(details.containers[0].env_vars.len(), 2);
    assert_eq!(details.containers[0].env_vars[0].name, "A");
    assert_eq!(details.containers[1].env_vars[0].value, "one");
    assert_eq!(details.containers[1].env_vars[1].resolution, crate::models::EnvResolution::Unavailable);
    assert!(details.containers[1].env_issues[0].optional);
    assert_eq!(details.containers[1].env_issues[0].reason, crate::models::EnvResolutionReason::NotFound);
    assert_eq!(details.containers[0].env_issues.len(), 1);
    let requests = requests.lock().unwrap();
    assert_eq!(requests.iter().filter(|p| p.ends_with("/configmaps/shared")).count(), 1);
    assert!(requests.iter().any(|p| p.ends_with("/configmaps/denied")));
    assert!(requests.iter().any(|p| p.ends_with("/configmaps/missing")));
    assert!(!requests.iter().any(|p| p.contains("/secrets/") || p.ends_with("/configmaps")));
}

#[tokio::test]
async fn pod_without_configmap_references_causes_no_configmap_request() {
    let pod: k8s_openapi::api::core::v1::Pod = serde_json::from_value(json!({"spec":{"containers":[{"name":"app","image":"x","env":[{"name":"A","value":"direct"}]}]}})).unwrap();
    let client = Client::new(tower::service_fn(|_: Request<Body>| async {
        panic!("unexpected request for pod without ConfigMap references");
        #[allow(unreachable_code)]
        Ok::<_, std::io::Error>(Response::new(Body::from(Vec::new())))
    }), "fixture");
    assert!(resolve_configmaps(&client, "fixture", &pod).await.is_empty());
}

#[tokio::test]
async fn each_details_request_refreshes_configmap_data() {
    let config = Arc::new(Mutex::new(json!({"apiVersion":"v1","kind":"ConfigMap","data":{"A":"one","B":"two"}})));
    let (client, requests) = client(config.clone());
    let first = get_pod_details_with_client(client.clone(), "fixture", "app").await.unwrap();
    assert_eq!(first.containers[0].env_vars[0].value, "one");
    config.lock().unwrap()["data"]["A"] = json!("two");
    let second = get_pod_details_with_client(client, "fixture", "app").await.unwrap();
    assert_eq!(second.containers[0].env_vars[0].value, "two");
    assert_eq!(requests.lock().unwrap().iter().filter(|path| path.ends_with("/configmaps/shared")).count(), 2);
}

struct Active(Arc<AtomicUsize>);
impl Drop for Active { fn drop(&mut self) { self.0.fetch_sub(1, Ordering::SeqCst); } }

#[tokio::test(start_paused = true)]
async fn shared_deadline_keeps_completed_success_and_limits_active_gets() {
    let refs: Vec<Value> = (0..13).map(|i| json!({"configMapRef":{"name":if i == 0 {"a-good".to_string()} else {format!("map-{i}")}}})).collect();
    let pod: k8s_openapi::api::core::v1::Pod = serde_json::from_value(json!({"spec":{"containers":[{"name":"app","image":"x","envFrom":refs}]}})).unwrap();
    let active = Arc::new(AtomicUsize::new(0));
    let max_active = Arc::new(AtomicUsize::new(0));
    let service = tower::service_fn({
        let active = active.clone(); let max_active = max_active.clone();
        move |request: Request<Body>| {
            let active = active.clone(); let max_active = max_active.clone();
            async move {
                let path = request.uri().path();
                if !path.ends_with("/a-good") {
                    let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                    max_active.fetch_max(current, Ordering::SeqCst);
                    let _guard = Active(active.clone());
                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                }
                let body = if path.ends_with("/a-good") {
                    json!({"apiVersion":"v1","kind":"ConfigMap","metadata":{"name":"a-good"},"data":{"X":"kept"}})
                } else { json!({"apiVersion":"v1","kind":"ConfigMap","data":{}}) };
                Ok::<_, std::io::Error>(Response::builder().status(200).body(Body::from(body.to_string().into_bytes())).unwrap())
            }
        }
    });
    let client = Client::new(service, "fixture");
    let started = tokio::time::Instant::now();
    let lookups = resolve_configmaps(&client, "fixture", &pod).await;
    assert_eq!(started.elapsed(), std::time::Duration::from_secs(8), "the shared resolution deadline must be exactly 8 seconds");
    assert_eq!(lookups["a-good"].as_ref().unwrap().data.as_ref().unwrap()["X"], "kept");
    assert_eq!(lookups.len(), 13);
    assert!(lookups.iter().filter(|(name, _)| name.as_str() != "a-good").all(|(_, result)| matches!(result, Err(crate::models::EnvResolutionReason::Timeout))));
    assert_eq!(max_active.load(Ordering::SeqCst), 4, "four ConfigMap GETs should run concurrently");
    assert_eq!(active.load(Ordering::SeqCst), 0, "timed-out HTTP futures must be dropped with the resolution request");
}

#[tokio::test(start_paused = true)]
async fn one_stalled_configmap_get_times_out_at_three_seconds() {
    let pod: k8s_openapi::api::core::v1::Pod = serde_json::from_value(json!({"spec":{"containers":[{"name":"app","image":"x","envFrom":[{"configMapRef":{"name":"stalled"}}]}]}})).unwrap();
    let active = Arc::new(AtomicUsize::new(0));
    let service = tower::service_fn({
        let active = active.clone();
        move |_: Request<Body>| {
            let active = active.clone();
            async move {
                active.fetch_add(1, Ordering::SeqCst);
                let _guard = Active(active.clone());
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                Ok::<_, std::io::Error>(Response::new(Body::from(Vec::new())))
            }
        }
    });
    let client = Client::new(service, "fixture");
    let started = tokio::time::Instant::now();
    let lookups = resolve_configmaps(&client, "fixture", &pod).await;
    assert_eq!(started.elapsed(), std::time::Duration::from_secs(3), "a per-GET timeout must be exactly 3 seconds");
    assert!(matches!(lookups["stalled"], Err(crate::models::EnvResolutionReason::Timeout)));
    assert_eq!(active.load(Ordering::SeqCst), 0);
}
