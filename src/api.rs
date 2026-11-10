//! Backend calls behind a trait so the Store can be tested with a fake.
use std::{future::Future, time::Duration};

use futures::{FutureExt, future::BoxFuture};

use crate::error::K8sError;
use crate::k8s;
use crate::models::{
    ClusterInfo, DeploymentDetails, DeploymentInfo, LogEntry, LogSearchResponse, NamespaceInfo,
    PodDetails, PodInfo,
};

pub type ApiResult<T> = Result<T, String>;
pub const TIMEOUT_MESSAGE: &str = "Connection timed out after 30 seconds";

pub async fn with_timeout<T>(fut: impl Future<Output = Result<T, K8sError>>) -> ApiResult<T> {
    match tokio::time::timeout(Duration::from_secs(30), fut).await {
        Ok(r) => r.map_err(|e| e.to_string()),
        Err(_) => Err(TIMEOUT_MESSAGE.to_string()),
    }
}

type Fut<T> = BoxFuture<'static, ApiResult<T>>;

pub trait Api: Send + Sync + 'static {
    fn clusters(&self) -> Fut<Vec<ClusterInfo>>;
    fn namespaces(&self, context: String) -> Fut<Vec<NamespaceInfo>>;
    fn deployments(&self, context: String, namespace: String) -> Fut<Vec<DeploymentInfo>>;
    fn deployment_details(&self, context: String, namespace: String, name: String) -> Fut<DeploymentDetails>;
    fn pods(&self, context: String, namespace: String, deployment: Option<String>) -> Fut<Vec<PodInfo>>;
    fn pod_details(&self, context: String, namespace: String, pod: String) -> Fut<PodDetails>;
    fn pod_logs(
        &self,
        context: String,
        namespace: String,
        pod: String,
        container: Option<String>,
        since_seconds: Option<i64>,
    ) -> Fut<Vec<LogEntry>>;
    #[allow(clippy::too_many_arguments)]
    fn search_logs(
        &self,
        context: String,
        namespace: String,
        deployment: String,
        keyword: Option<String>,
        log_level: Option<String>,
        since_seconds: Option<i64>,
    ) -> Fut<LogSearchResponse>;
}

/// Runs every call on the app's tokio runtime (kube-rs needs tokio; gpui has its own executor).
pub struct KubeApi {
    rt: tokio::runtime::Handle,
}

impl KubeApi {
    pub fn new(rt: tokio::runtime::Handle) -> Self {
        Self { rt }
    }

    fn run<T: Send + 'static>(&self, f: impl Future<Output = ApiResult<T>> + Send + 'static) -> Fut<T> {
        let handle = self.rt.spawn(f);
        async move { handle.await.map_err(|e| e.to_string())? }.boxed()
    }
}

impl Api for KubeApi {
    fn clusters(&self) -> Fut<Vec<ClusterInfo>> {
        // Local kubeconfig read: no timeout (same as today).
        self.run(async { k8s::clusters::get_clusters().await.map_err(|e| e.to_string()) })
    }
    fn namespaces(&self, context: String) -> Fut<Vec<NamespaceInfo>> {
        self.run(with_timeout(k8s::namespaces::get_namespaces(context)))
    }
    fn deployments(&self, context: String, namespace: String) -> Fut<Vec<DeploymentInfo>> {
        self.run(with_timeout(k8s::deployments::get_deployments(context, namespace)))
    }
    fn deployment_details(&self, context: String, namespace: String, name: String) -> Fut<DeploymentDetails> {
        self.run(with_timeout(k8s::deployments::get_deployment_details(context, namespace, name)))
    }
    fn pods(&self, context: String, namespace: String, deployment: Option<String>) -> Fut<Vec<PodInfo>> {
        self.run(with_timeout(k8s::pods::get_pods(context, namespace, deployment)))
    }
    fn pod_details(&self, context: String, namespace: String, pod: String) -> Fut<PodDetails> {
        self.run(with_timeout(k8s::pods::get_pod_details(context, namespace, pod)))
    }
    fn pod_logs(
        &self,
        context: String,
        namespace: String,
        pod: String,
        container: Option<String>,
        since_seconds: Option<i64>,
    ) -> Fut<Vec<LogEntry>> {
        self.run(with_timeout(k8s::logs::get_pod_logs(
            context, namespace, pod, container, since_seconds, None,
        )))
    }
    fn search_logs(
        &self,
        context: String,
        namespace: String,
        deployment: String,
        keyword: Option<String>,
        log_level: Option<String>,
        since_seconds: Option<i64>,
    ) -> Fut<LogSearchResponse> {
        self.run(with_timeout(k8s::logs::search_deployment_logs(
            context, namespace, deployment, keyword, log_level, since_seconds,
        )))
    }
}

#[cfg(test)]
pub mod fake {
    use std::collections::{HashMap, VecDeque};
    use std::sync::Mutex;

    use futures::channel::oneshot;
    use serde::de::DeserializeOwned;
    use serde_json::Value;

    use super::*;

    #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
    pub enum Endpoint {
        Clusters,
        Namespaces,
        Deployments,
        DeploymentDetails,
        Pods,
        PodDetails,
        PodLogs,
        SearchLogs,
    }

    enum Reply {
        Ready(Result<Value, String>),
        Gate(oneshot::Receiver<Result<Value, String>>),
    }

    #[derive(Default)]
    struct State {
        replies: HashMap<Endpoint, VecDeque<Reply>>,
        calls: HashMap<Endpoint, usize>,
        last_args: HashMap<Endpoint, Vec<String>>,
    }

    /// Queued replies per endpoint; an unqueued call fails with "no fake reply".
    #[derive(Default)]
    pub struct FakeApi {
        state: Mutex<State>,
    }

    impl FakeApi {
        pub fn reply(&self, ep: Endpoint, r: Result<Value, String>) {
            self.push(ep, Reply::Ready(r));
        }

        pub fn gate(&self, ep: Endpoint) -> oneshot::Sender<Result<Value, String>> {
            let (tx, rx) = oneshot::channel();
            self.push(ep, Reply::Gate(rx));
            tx
        }

        pub fn calls(&self, ep: Endpoint) -> usize {
            self.state.lock().unwrap().calls.get(&ep).copied().unwrap_or(0)
        }

        pub fn last_args(&self, ep: Endpoint) -> Vec<String> {
            self.state.lock().unwrap().last_args.get(&ep).cloned().unwrap_or_default()
        }

        fn push(&self, ep: Endpoint, r: Reply) {
            self.state.lock().unwrap().replies.entry(ep).or_default().push_back(r);
        }

        fn call<T: DeserializeOwned + Send + 'static>(&self, ep: Endpoint, args: Vec<String>) -> Fut<T> {
            let reply = {
                let mut s = self.state.lock().unwrap();
                *s.calls.entry(ep).or_default() += 1;
                s.last_args.insert(ep, args);
                s.replies.get_mut(&ep).and_then(|q| q.pop_front())
            };
            async move {
                let value = match reply {
                    None => Err("no fake reply".to_string()),
                    Some(Reply::Ready(r)) => r,
                    Some(Reply::Gate(rx)) => rx.await.unwrap_or_else(|_| Err("gate dropped".into())),
                }?;
                serde_json::from_value(value).map_err(|e| e.to_string())
            }
            .boxed()
        }
    }

    macro_rules! args {
        ($($a:expr),*) => { vec![$(format!("{:?}", $a)),*] };
    }

    impl Api for FakeApi {
        fn clusters(&self) -> Fut<Vec<ClusterInfo>> {
            self.call(Endpoint::Clusters, vec![])
        }
        fn namespaces(&self, context: String) -> Fut<Vec<NamespaceInfo>> {
            self.call(Endpoint::Namespaces, args!(context))
        }
        fn deployments(&self, context: String, namespace: String) -> Fut<Vec<DeploymentInfo>> {
            self.call(Endpoint::Deployments, args!(context, namespace))
        }
        fn deployment_details(&self, context: String, namespace: String, name: String) -> Fut<DeploymentDetails> {
            self.call(Endpoint::DeploymentDetails, args!(context, namespace, name))
        }
        fn pods(&self, context: String, namespace: String, deployment: Option<String>) -> Fut<Vec<PodInfo>> {
            self.call(Endpoint::Pods, args!(context, namespace, deployment))
        }
        fn pod_details(&self, context: String, namespace: String, pod: String) -> Fut<PodDetails> {
            self.call(Endpoint::PodDetails, args!(context, namespace, pod))
        }
        fn pod_logs(
            &self,
            context: String,
            namespace: String,
            pod: String,
            container: Option<String>,
            since_seconds: Option<i64>,
        ) -> Fut<Vec<LogEntry>> {
            self.call(Endpoint::PodLogs, args!(context, namespace, pod, container, since_seconds))
        }
        fn search_logs(
            &self,
            context: String,
            namespace: String,
            deployment: String,
            keyword: Option<String>,
            log_level: Option<String>,
            since_seconds: Option<i64>,
        ) -> Fut<LogSearchResponse> {
            self.call(
                Endpoint::SearchLogs,
                args!(context, namespace, deployment, keyword, log_level, since_seconds),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::K8sError;

    #[tokio::test(start_paused = true)]
    async fn timeout_maps_to_exact_message() {
        let r: ApiResult<()> = with_timeout(std::future::pending()).await;
        assert_eq!(r.unwrap_err(), "Connection timed out after 30 seconds");
    }
    #[tokio::test]
    async fn backend_error_uses_display_text() {
        let r: ApiResult<()> = with_timeout(async { Err(K8sError::NoKubeconfig) }).await;
        assert_eq!(r.unwrap_err(), "No kubeconfig found");
    }
    #[tokio::test]
    async fn fake_gate_resolves_when_sent() {
        let fake = fake::FakeApi::default();
        let tx = fake.gate(fake::Endpoint::Pods);
        let fut = fake.pods("c".into(), "ns".into(), None);
        tx.send(Ok(serde_json::json!([]))).unwrap();
        assert!(fut.await.unwrap().is_empty());
        assert_eq!(fake.calls(fake::Endpoint::Pods), 1);
    }
}
