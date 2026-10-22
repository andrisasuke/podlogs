use super::*;
use http::{Request, Response};
use kube::client::Body;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::Duration;
use tokio::sync::{mpsc, Semaphore};
use tower::service_fn;

// An in-memory Kubernetes HTTP fixture. No kubeconfig, socket, or cluster is used.
#[derive(Clone)]
enum LogReply {
    Logs(String),
    Forbidden,
    Timeout,
}

struct Fixture {
    pods: Vec<Value>,
    logs: BTreeMap<(String, String), LogReply>,
    requests: Arc<Mutex<Vec<String>>>,
    deployment_status: u16,
    pods_status: u16,
    gate: Option<Arc<Semaphore>>,
    started: Option<mpsc::UnboundedSender<()>>,
    active: Arc<AtomicUsize>,
    max_active: Arc<AtomicUsize>,
}

impl Fixture {
    fn new(targets: &[(&str, &str, LogReply)]) -> Self {
        let mut containers: BTreeMap<&str, Vec<Value>> = BTreeMap::new();
        let mut logs = BTreeMap::new();
        for (pod, container, reply) in targets {
            containers.entry(pod).or_default().push(json!({
                "name": container, "image": "fixture"
            }));
            logs.insert((pod.to_string(), container.to_string()), reply.clone());
        }
        Self {
            // Deliberately return reverse order; completion must not determine display order.
            pods: containers
                .into_iter()
                .rev()
                .map(|(pod, containers)| {
                    json!({
                        "metadata": {"name": pod},
                        "spec": {
                            "containers": containers,
                            "initContainers": [{"name": "init", "image": "fixture"}],
                            "ephemeralContainers": [{"name": "debugger", "image": "fixture"}]
                        }
                    })
                })
                .collect(),
            logs,
            requests: Arc::default(),
            deployment_status: 200,
            pods_status: 200,
            gate: None,
            started: None,
            active: Arc::default(),
            max_active: Arc::default(),
        }
    }

    fn client(self) -> Client {
        let fixture = Arc::new(self);
        Client::new(
            service_fn(move |request: Request<Body>| {
                let fixture = fixture.clone();
                async move {
                    assert_eq!(request.method(), http::Method::GET);
                    let uri = request.uri();
                    fixture.requests.lock().unwrap().push(uri.to_string());
                    let (status, body) =
                        if uri.path() == "/apis/apps/v1/namespaces/spp/deployments/queue" {
                            (
                                fixture.deployment_status,
                                json!({
                                    "apiVersion": "apps/v1", "kind": "Deployment",
                                    "metadata": {"name": "queue"},
                                    "spec": {
                                        "selector": {"matchLabels": {"app": "queue"}},
                                        "template": {"metadata": {}, "spec": {"containers": []}}
                                    }
                                })
                                .to_string(),
                            )
                        } else if uri.path() == "/api/v1/namespaces/spp/pods" {
                            assert!(uri.query().unwrap().contains("labelSelector=app%3Dqueue"));
                            (
                                fixture.pods_status,
                                json!({
                                    "apiVersion": "v1", "kind": "PodList", "metadata": {},
                                    "items": fixture.pods
                                })
                                .to_string(),
                            )
                        } else {
                            assert!(uri.path().ends_with("/log"), "Unexpected request: {uri}");
                            let pod = uri.path().split('/').nth(6).unwrap();
                            let query: BTreeMap<_, _> = uri
                                .query()
                                .unwrap()
                                .split('&')
                                .filter_map(|param| param.split_once('='))
                                .collect();
                            let container = query["container"];
                            let active = fixture.active.fetch_add(1, Ordering::SeqCst) + 1;
                            fixture.max_active.fetch_max(active, Ordering::SeqCst);
                            if let Some(started) = &fixture.started {
                                started.send(()).unwrap();
                            }
                            if let Some(gate) = &fixture.gate {
                                gate.acquire().await.unwrap().forget();
                            }
                            fixture.active.fetch_sub(1, Ordering::SeqCst);
                            match &fixture.logs[&(pod.to_string(), container.to_string())] {
                                LogReply::Logs(logs) => {
                                    // Honor tailLines so the old implementation would actually lose the match.
                                    let body = if let Some(tail) = query.get("tailLines") {
                                        let lines: Vec<_> = logs.lines().collect();
                                        lines[lines.len().saturating_sub(tail.parse().unwrap())..]
                                            .join("\n")
                                    } else {
                                        logs.clone()
                                    };
                                    (200, body)
                                }
                                LogReply::Forbidden => (403, String::new()),
                                LogReply::Timeout => {
                                    return Err(std::io::Error::new(
                                        std::io::ErrorKind::TimedOut,
                                        "container request timed out",
                                    ))
                                }
                            }
                        };
                    let body = if status == 200 {
                        body
                    } else {
                        json!({
                        "apiVersion": "v1", "kind": "Status", "status": "Failure",
                        "code": status, "reason": "Forbidden", "message": "fixture access denied"
                    }).to_string()
                    };
                    Ok(Response::builder()
                        .status(status)
                        .body(Body::from(body.into_bytes()))
                        .unwrap())
                }
            }),
            "spp",
        )
    }
}

async fn search(client: Client, keyword: Option<&str>, level: Option<&str>) -> LogSearchResponse {
    search_deployment_logs_with_client(client, "spp", "queue", keyword, level, Some(10800))
        .await
        .unwrap()
}

#[tokio::test]
async fn finds_forecast_before_last_thousand_lines_and_matches_pod_log_parser() {
    let first = "2026-09-30T12:00:06.000000000Z ERROR Forecast sync failed for warehouse jk01";
    let mut logs = format!("{first}\n");
    for i in 0..1500 {
        logs.push_str(&format!(
            "2026-09-30T13:00:00.000000000Z INFO heartbeat {i}\n"
        ));
    }
    let fixture = Fixture::new(&[("queue-a", "master", LogReply::Logs(logs))]);
    let requests = fixture.requests.clone();
    let client = fixture.client();
    let response = search(client.clone(), Some("forecast sync"), None).await;
    assert_eq!(response.total_containers, 1);
    assert_eq!(response.successful_containers, 1);
    assert!(response.failures.is_empty());
    assert_eq!(response.results[0].total_matches, 1);
    assert_eq!(response.results[0].entries[0].raw, first);
    let search_requests = requests.lock().unwrap().clone();
    let log_request = search_requests
        .iter()
        .find(|uri| uri.contains("/log?"))
        .unwrap();
    assert!(!log_request.contains("tailLines"));
    assert!(log_request.contains("sinceSeconds=10800"));
    assert!(log_request.contains("timestamps=true"));
    assert!(log_request.contains("container=master"));

    // Stable comparison with the same unbounded API read and parser as Pod Logs.
    let pods: Api<Pod> = Api::namespaced(client, "spp");
    let logs = pods
        .logs(
            "queue-a",
            &LogParams {
                container: Some("master".into()),
                timestamps: true,
                since_seconds: Some(10800),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(logs.lines().count(), 1501);
    let viewer_entry = parse_log_line(logs.lines().next().unwrap(), "queue-a", "master");
    assert_eq!(
        serde_json::to_value(viewer_entry).unwrap(),
        serde_json::to_value(&response.results[0].entries[0]).unwrap()
    );
}

#[tokio::test]
async fn filters_plain_json_raw_case_empty_keyword_and_any_or_error_level() {
    let logs = [
        "2026-09-30T12:00:06.000000000Z ERROR Forecast sync failed",
        r#"2026-09-30T12:01:00.000000000Z {"level":"info","message":"FORECAST SYNC complete"}"#,
        r#"2026-09-30T12:02:00.000000000Z {"severity":"err","msg":"worker failed","job":"forecast"}"#,
        "2026-09-30T12:03:00.000000000Z forecast task without level",
        "2026-09-30T12:04:00.000000000Z INFO heartbeat",
    ].join("\n");
    for (keyword, level, expected) in [
        (Some("forecast"), None, 4),
        (Some("forecast sync"), None, 2),
        (Some("FoReCaSt SyNc"), Some("Any"), 2),
        (Some("forecast"), Some("error"), 2),
        (Some("forecast sync"), Some("ERROR"), 1),
        (Some(""), None, 5),
        (None, Some(""), 5),
        (None, Some("ERROR"), 2),
        (Some("absent"), None, 0),
    ] {
        let fixture = Fixture::new(&[("queue-a", "master", LogReply::Logs(logs.clone()))]);
        let response = search(fixture.client(), keyword, level).await;
        let matches: i32 = response.results.iter().map(|r| r.total_matches).sum();
        assert_eq!(matches, expected, "keyword={keyword:?} level={level:?}");
        assert_eq!(response.successful_containers, 1);
        assert!(response.failures.is_empty());
    }
}

#[tokio::test]
async fn reports_partial_failures_and_counts_successes_without_matches() {
    let fixture = Fixture::new(&[
        (
            "queue-a",
            "master",
            LogReply::Logs("ERROR forecast failed".into()),
        ),
        (
            "queue-a",
            "sidecar",
            LogReply::Logs("INFO heartbeat".into()),
        ),
        ("queue-b", "master", LogReply::Forbidden),
        ("queue-c", "master", LogReply::Timeout),
    ]);
    let response = search(fixture.client(), Some("forecast"), None).await;
    assert_eq!(response.total_containers, 4);
    assert_eq!(response.successful_containers, 2);
    assert_eq!(response.results.len(), 1);
    assert_eq!(response.failures.len(), 2);
    assert_eq!(response.failures[0].pod_name, "queue-b");
    assert_eq!(response.failures[0].container_name, "master");
    assert!(response.failures[0]
        .message
        .contains("fixture access denied"));
    assert!(response.failures[1].message.contains("timed out"));
}

#[tokio::test]
async fn preserves_all_failures_and_partial_without_matches() {
    for all_failed in [true, false] {
        let reply = if all_failed {
            LogReply::Forbidden
        } else {
            LogReply::Logs(String::new())
        };
        let response = search(
            Fixture::new(&[
                ("queue-a", "master", reply),
                ("queue-b", "master", LogReply::Forbidden),
            ])
            .client(),
            None,
            None,
        )
        .await;
        assert_eq!(response.total_containers, 2);
        assert_eq!(response.successful_containers, usize::from(!all_failed));
        assert_eq!(response.failures.len(), if all_failed { 2 } else { 1 });
        assert!(response.results.is_empty());
    }
}

#[tokio::test]
async fn returns_zero_targets_for_no_pods_or_no_regular_containers() {
    for no_pods in [true, false] {
        let mut fixture = Fixture::new(&[]);
        if !no_pods {
            fixture
                .pods
                .push(json!({"metadata": {"name": "empty"}, "spec": {"containers": []}}));
        }
        let response = search(fixture.client(), None, None).await;
        assert_eq!(response.total_containers, 0);
        assert_eq!(response.successful_containers, 0);
        assert!(response.results.is_empty());
        assert!(response.failures.is_empty());
    }
}

#[tokio::test]
async fn deployment_and_pod_list_failures_fail_the_entire_search() {
    for deployment_fails in [true, false] {
        let mut fixture = Fixture::new(&[]);
        if deployment_fails {
            fixture.deployment_status = 403;
        } else {
            fixture.pods_status = 403;
        }
        let error =
            search_deployment_logs_with_client(fixture.client(), "spp", "queue", None, None, None)
                .await
                .unwrap_err();
        assert!(error.to_string().contains("fixture access denied"));
    }
}

#[tokio::test]
async fn searches_all_regular_containers_with_at_most_four_requests_and_stable_order() {
    let log = LogReply::Logs("ERROR forecast failed".into());
    let mut fixture = Fixture::new(&[
        ("queue-c", "master", log.clone()),
        ("queue-a", "sidecar", log.clone()),
        ("queue-a", "master", log.clone()),
        ("queue-b", "master", log.clone()),
        ("queue-d", "master", log.clone()),
        ("queue-e", "master", log),
    ]);
    let gate = Arc::new(Semaphore::new(0));
    let (sender, mut started) = mpsc::unbounded_channel();
    fixture.gate = Some(gate.clone());
    fixture.started = Some(sender);
    let max_active = fixture.max_active.clone();
    let task = tokio::spawn(search(fixture.client(), Some("forecast"), None));
    for _ in 0..4 {
        tokio::time::timeout(Duration::from_secs(5), started.recv())
            .await
            .unwrap()
            .unwrap();
    }
    assert!(started.try_recv().is_err());
    assert_eq!(max_active.load(Ordering::SeqCst), 4);
    gate.add_permits(1);
    tokio::time::timeout(Duration::from_secs(5), started.recv())
        .await
        .unwrap()
        .unwrap();
    gate.add_permits(6);
    let response = tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(max_active.load(Ordering::SeqCst), 4);
    assert_eq!(response.total_containers, 6);
    assert_eq!(response.successful_containers, 6);
    let order: Vec<_> = response
        .results
        .iter()
        .map(|r| (r.pod_name.as_str(), r.container_name.as_str()))
        .collect();
    assert_eq!(
        order,
        vec![
            ("queue-a", "master"),
            ("queue-a", "sidecar"),
            ("queue-b", "master"),
            ("queue-c", "master"),
            ("queue-d", "master"),
            ("queue-e", "master"),
        ]
    );
}
