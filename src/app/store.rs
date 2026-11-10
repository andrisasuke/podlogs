//! The app's state: UI + cluster selection (Zustand stores today), the query cache with its polling
//! rules (TanStack Query), navigation side effects (`MainLayout.tsx`) and error reporting (`errorStore`).
//! Time always comes from `cx.background_executor().now()` so tests can drive a fake clock.
use std::collections::BTreeMap;
use std::hash::Hash;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui_kit::*;

use super::query::{CacheEmpty, Outcome, Query};
use crate::api::Api;
use crate::format::TimeRange;
use crate::models::{
    ClusterInfo, DeploymentDetails, DeploymentInfo, LogEntry, NamespaceInfo, PodDetails, PodInfo,
};
use crate::persist::{self, PersistedState, ThemePref};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    Deployments,
    Pods,
    Logs,
    Search,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum DrawerKind {
    Pod(String),
    Deployment(String),
    Settings,
}

pub type NsKey = (String, String);
pub type PodsKey = (String, String, Option<String>);
pub type DetailsKey = (String, String, String);
/// context, namespace, pod, container, since seconds
pub type LogsKey = (String, String, String, String, i64);

/// Errors are ignored for this long after a context/namespace switch (`endTransition` timer).
const TRANSITION: Duration = Duration::from_secs(1);
/// React Query's default `retryDelay` for the first retry.
const LOGS_RETRY_DELAY: Duration = Duration::from_secs(1);

pub struct Store {
    api: Arc<dyn Api>,
    persist_path: Option<PathBuf>,

    pub theme: ThemePref,
    pub sidebar_collapsed: bool,
    pub refresh_interval_ms: u32,
    pub view: View,
    pub drawer: Option<DrawerKind>,
    pub log_viewer_pod: Option<String>,
    pub context: String,
    pub namespace: String,
    pub deployment: Option<String>,
    pub last_namespace_by_cluster: BTreeMap<String, String>,
    pub log_container: Option<String>,
    pub log_time_range: TimeRange,
    pub error: Option<String>,
    pub countdown_s: u32,
    pub os_dark: bool,

    pub clusters: Query<(), Vec<ClusterInfo>>,
    pub namespaces: Query<String, Vec<NamespaceInfo>>,
    pub deployments: Query<NsKey, Vec<DeploymentInfo>>,
    pub pods: Query<PodsKey, Vec<PodInfo>>,
    pub pod_details: Query<DetailsKey, PodDetails>,
    pub deployment_details: Query<DetailsKey, DeploymentDetails>,
    pub pod_logs: Query<LogsKey, Vec<LogEntry>>,

    transition_until: Option<Instant>,
    /// 0 = first try of a pod-logs fetch, 1 = the single retry.
    logs_attempt: u8,
}

/// `reportErrorIfNoCache`: only a failure with nothing cached (or an empty list) reaches the dialog.
fn report<K: Clone + Eq + Hash, T: Clone + CacheEmpty>(q: &Query<K, T>, transitioning: bool, error: &mut Option<String>) {
    if !transitioning && q.data().is_none_or(CacheEmpty::cache_empty) {
        *error = q.error().map(str::to_owned);
    }
}

fn finish<K: Clone + Eq + Hash, T: Clone + CacheEmpty>(
    q: &mut Query<K, T>,
    key: &K,
    generation: u64,
    result: Result<T, String>,
    now: Instant,
    transitioning: bool,
    error: &mut Option<String>,
) -> Outcome {
    let out = q.finish(key, generation, result, now);
    if out == Outcome::Err {
        report(q, transitioning, error);
    }
    out
}

impl Store {
    pub fn new(api: Arc<dyn Api>, persisted: PersistedState, persist_path: Option<PathBuf>, cx: &mut Context<Self>) -> Self {
        let mut s = Self {
            api,
            persist_path,
            theme: persisted.theme,
            sidebar_collapsed: persisted.sidebar_collapsed,
            refresh_interval_ms: persisted.refresh_interval_ms,
            view: View::Deployments,
            drawer: None,
            log_viewer_pod: None,
            context: persisted.context,
            namespace: persisted.namespace,
            deployment: persisted.deployment,
            last_namespace_by_cluster: persisted.last_namespace_by_cluster,
            log_container: None,
            log_time_range: TimeRange::M30,
            error: None,
            countdown_s: persisted.refresh_interval_ms / 1000,
            os_dark: false,
            clusters: Query::new(Duration::MAX, false),
            namespaces: Query::new(Duration::from_secs(60), false),
            deployments: Query::new(Duration::from_secs(30), true),
            pods: Query::new(Duration::ZERO, true),
            pod_details: Query::new(Duration::ZERO, false),
            deployment_details: Query::new(Duration::ZERO, false),
            pod_logs: Query::new(Duration::ZERO, true),
            transition_until: None,
            logs_attempt: 0,
        };
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                if this.update(cx, |s, cx| s.tick(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        s.refresh(cx);
        s
    }

    // ---------- time loop ----------

    /// One second: count down, recompute active keys, start due fetches.
    pub fn tick(&mut self, cx: &mut Context<Self>) {
        self.countdown_s = self.countdown_s.saturating_sub(1);
        self.refresh(cx);
    }

    fn now(cx: &App) -> Instant {
        cx.background_executor().now()
    }

    fn transitioning(&self, now: Instant) -> bool {
        self.transition_until.is_some_and(|u| now < u)
    }

    // ---------- derived state ----------

    pub fn is_dark(&self) -> bool {
        self.theme == ThemePref::Dark || (self.theme == ThemePref::System && self.os_dark)
    }

    fn pod_details_name(&self) -> Option<String> {
        match &self.drawer {
            Some(DrawerKind::Pod(p)) => Some(p.clone()),
            _ if self.view == View::Logs => self.log_viewer_pod.clone(),
            _ => None,
        }
    }

    fn pod_logs_key(&self) -> Option<LogsKey> {
        let (c, n) = self.scope()?;
        if self.view != View::Logs {
            return None;
        }
        Some((c, n, self.log_viewer_pod.clone()?, self.log_container.clone()?, self.log_time_range.seconds()))
    }

    /// `(context, namespace)` once both are non-empty.
    fn scope(&self) -> Option<(String, String)> {
        (!self.context.is_empty() && !self.namespace.is_empty()).then(|| (self.context.clone(), self.namespace.clone()))
    }

    fn sync_active(&mut self, now: Instant) {
        let scope = self.scope();
        self.clusters.set_active(Some(()), now);
        self.namespaces.set_active((!self.context.is_empty()).then(|| self.context.clone()), now);
        self.deployments.set_active(scope.clone(), now);
        self.pods.set_active(scope.clone().map(|(c, n)| (c, n, self.deployment.clone())), now);
        let details = |name: Option<String>| scope.clone().zip(name).map(|((c, n), name)| (c, n, name));
        self.pod_details.set_active(details(self.pod_details_name()), now);
        let dep = match &self.drawer {
            Some(DrawerKind::Deployment(d)) => Some(d.clone()),
            _ => None,
        };
        self.deployment_details.set_active(details(dep), now);
        self.pod_logs.set_active(self.pod_logs_key(), now);
    }

    /// Loaded-data defaults (Sidebar effects, LogViewer container default). Returns whether state changed.
    fn derive(&mut self, now: Instant) -> bool {
        let mut changed = false;
        if self.context.is_empty()
            && let Some(cl) = self.clusters.data()
            && let Some(pick) = cl.iter().find(|c| c.is_current).or(cl.first())
        {
            let name = pick.name.clone();
            self.change_context(name, now);
            changed = true;
        }
        if !self.context.is_empty()
            && let Some(ns) = self.namespaces.data()
            && !ns.iter().any(|n| n.name == self.namespace)
            && let Some(pick) = ns.iter().find(|n| n.name == "default").or(ns.first())
        {
            let name = pick.name.clone();
            self.change_namespace(name, now);
            changed = true;
        }
        if self.view == View::Logs
            && self.log_container.is_none()
            && let Some(d) = self.pod_details.data()
            && Some(&d.name) == self.log_viewer_pod.as_ref()
            && let Some(first) = d.containers.first()
        {
            self.log_container = Some(first.name.clone());
            changed = true;
        }
        changed
    }

    /// Recompute active keys, apply defaults, start whatever is due.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        let now = Self::now(cx);
        self.sync_active(now);
        if self.derive(now) {
            self.persist();
            self.sync_active(now);
        }
        self.start_due(now, cx);
        cx.notify();
    }

    fn start_due(&mut self, now: Instant, cx: &mut Context<Self>) {
        let interval = Duration::from_millis(self.refresh_interval_ms as u64);
        macro_rules! go {
            ($q:ident, $on:ident, |$k:ident| $call:expr) => {
                if self.$q.needs_fetch(now, interval)
                    && let Some(($k, generation)) = self.$q.start(now)
                {
                    let fut = $call;
                    cx.spawn(async move |this, cx| {
                        let r = fut.await;
                        let _ = this.update(cx, |s, cx| s.$on($k, generation, r, cx));
                    })
                    .detach();
                }
            };
        }
        go!(clusters, on_clusters, |k| self.api.clusters());
        go!(namespaces, on_namespaces, |k| self.api.namespaces(k.clone()));
        go!(deployments, on_deployments, |k| self.api.deployments(k.0.clone(), k.1.clone()));
        go!(pods, on_pods, |k| self.api.pods(k.0.clone(), k.1.clone(), k.2.clone()));
        go!(pod_details, on_pod_details, |k| self.api.pod_details(k.0.clone(), k.1.clone(), k.2.clone()));
        go!(deployment_details, on_deployment_details, |k| {
            self.api.deployment_details(k.0.clone(), k.1.clone(), k.2.clone())
        });
        go!(pod_logs, on_pod_logs, |k| {
            self.logs_attempt = 0;
            self.api.pod_logs(k.0.clone(), k.1.clone(), k.2.clone(), Some(k.3.clone()), Some(k.4))
        });
    }

    // ---------- fetch results ----------

    fn on_clusters(&mut self, key: (), generation: u64, r: Result<Vec<ClusterInfo>, String>, cx: &mut Context<Self>) {
        let now = Self::now(cx);
        let tr = self.transitioning(now);
        finish(&mut self.clusters, &key, generation, r, now, tr, &mut self.error);
        self.refresh(cx);
    }

    fn on_namespaces(&mut self, key: String, generation: u64, r: Result<Vec<NamespaceInfo>, String>, cx: &mut Context<Self>) {
        let now = Self::now(cx);
        let tr = self.transitioning(now);
        finish(&mut self.namespaces, &key, generation, r, now, tr, &mut self.error);
        self.refresh(cx);
    }

    fn on_deployments(&mut self, key: NsKey, generation: u64, r: Result<Vec<DeploymentInfo>, String>, cx: &mut Context<Self>) {
        let now = Self::now(cx);
        let tr = self.transitioning(now);
        finish(&mut self.deployments, &key, generation, r, now, tr, &mut self.error);
        self.refresh(cx);
    }

    fn on_pods(&mut self, key: PodsKey, generation: u64, r: Result<Vec<PodInfo>, String>, cx: &mut Context<Self>) {
        let now = Self::now(cx);
        let tr = self.transitioning(now);
        let out = finish(&mut self.pods, &key, generation, r, now, tr, &mut self.error);
        if out != Outcome::Stale {
            self.countdown_s = self.refresh_interval_ms / 1000;
        }
        self.refresh(cx);
    }

    fn on_pod_details(&mut self, key: DetailsKey, generation: u64, r: Result<PodDetails, String>, cx: &mut Context<Self>) {
        let now = Self::now(cx);
        let tr = self.transitioning(now);
        finish(&mut self.pod_details, &key, generation, r, now, tr, &mut self.error);
        self.refresh(cx);
    }

    fn on_deployment_details(&mut self, key: DetailsKey, generation: u64, r: Result<DeploymentDetails, String>, cx: &mut Context<Self>) {
        let now = Self::now(cx);
        let tr = self.transitioning(now);
        finish(&mut self.deployment_details, &key, generation, r, now, tr, &mut self.error);
        self.refresh(cx);
    }

    /// `retry: 1`: the first failure of a fetch is retried once after 1 s; only the second is reported.
    fn on_pod_logs(&mut self, key: LogsKey, generation: u64, r: Result<Vec<LogEntry>, String>, cx: &mut Context<Self>) {
        let now = Self::now(cx);
        let out = self.pod_logs.finish(&key, generation, r, now);
        if out == Outcome::Err {
            if self.logs_attempt == 0 {
                cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(LOGS_RETRY_DELAY).await;
                    let _ = this.update(cx, |s, cx| s.retry_pod_logs(key, cx));
                })
                .detach();
            } else {
                let tr = self.transitioning(now);
                report(&self.pod_logs, tr, &mut self.error);
            }
        }
        self.refresh(cx);
    }

    fn retry_pod_logs(&mut self, key: LogsKey, cx: &mut Context<Self>) {
        if self.pod_logs_key().as_ref() != Some(&key) || self.pod_logs.is_fetching() {
            return; // canceled, key changed, or a newer fetch already runs
        }
        let now = Self::now(cx);
        if let Some((k, generation)) = self.pod_logs.start(now) {
            self.logs_attempt = 1;
            let fut = self.api.pod_logs(k.0.clone(), k.1.clone(), k.2.clone(), Some(k.3.clone()), Some(k.4));
            cx.spawn(async move |this, cx| {
                let r = fut.await;
                let _ = this.update(cx, |s, cx| s.on_pod_logs(k, generation, r, cx));
            })
            .detach();
        }
        cx.notify();
    }

    // ---------- persistence ----------

    fn persist(&self) {
        let Some(path) = &self.persist_path else { return };
        let state = PersistedState {
            theme: self.theme,
            sidebar_collapsed: self.sidebar_collapsed,
            refresh_interval_ms: self.refresh_interval_ms,
            context: self.context.clone(),
            namespace: self.namespace.clone(),
            deployment: self.deployment.clone(),
            last_namespace_by_cluster: self.last_namespace_by_cluster.clone(),
        };
        // ponytail: synchronous write on the UI thread; the file is tiny. Move off-thread if it ever shows up in a profile.
        if let Err(e) = persist::save(path, &state) {
            eprintln!("podlogs: could not save state: {e}");
        }
    }

    // ---------- scope changes (MainLayout effect) ----------

    fn change_context(&mut self, ctx: String, now: Instant) {
        if ctx == self.context {
            return;
        }
        self.namespace = self.last_namespace_by_cluster.get(&ctx).filter(|s| !s.is_empty()).cloned().unwrap_or_else(|| "default".into());
        self.context = ctx;
        self.deployment = None;
        self.scope_changed(now);
    }

    fn change_namespace(&mut self, ns: String, now: Instant) {
        self.last_namespace_by_cluster.insert(self.context.clone(), ns.clone());
        self.deployment = None;
        let changed = ns != self.namespace;
        self.namespace = ns;
        if changed {
            self.scope_changed(now);
        }
    }

    /// Ignore errors for 1 s, cancel in-flight logs/details, leave Logs/Pods, close a pod drawer.
    fn scope_changed(&mut self, now: Instant) {
        self.transition_until = Some(now + TRANSITION);
        self.error = None;
        self.pod_logs.cancel();
        self.pod_details.cancel();
        if matches!(self.view, View::Logs | View::Pods) {
            self.view = View::Deployments;
            self.log_viewer_pod = None;
        }
        if matches!(self.drawer, Some(DrawerKind::Pod(_))) {
            self.drawer = None;
        }
    }

    // ---------- actions ----------

    fn changed(&mut self, persist: bool, cx: &mut Context<Self>) {
        if persist {
            self.persist();
        }
        self.refresh(cx);
    }

    pub fn set_context(&mut self, ctx: String, cx: &mut Context<Self>) {
        let now = Self::now(cx);
        self.change_context(ctx, now);
        self.changed(true, cx);
    }

    pub fn set_namespace(&mut self, ns: String, cx: &mut Context<Self>) {
        let now = Self::now(cx);
        self.change_namespace(ns, now);
        self.changed(true, cx);
    }

    /// Sidebar click on a deployment: toggle the filter and show Pods.
    pub fn toggle_deployment_and_show_pods(&mut self, name: String, cx: &mut Context<Self>) {
        self.deployment = if self.deployment.as_deref() == Some(name.as_str()) { None } else { Some(name) };
        self.view = View::Pods;
        self.log_viewer_pod = None;
        self.changed(true, cx);
    }

    pub fn set_deployment(&mut self, deployment: Option<String>, cx: &mut Context<Self>) {
        self.deployment = deployment;
        self.changed(true, cx);
    }

    pub fn set_view(&mut self, view: View, cx: &mut Context<Self>) {
        self.view = view;
        self.log_viewer_pod = None;
        self.changed(false, cx);
    }

    pub fn open_log_viewer(&mut self, pod: String, cx: &mut Context<Self>) {
        self.view = View::Logs;
        self.log_viewer_pod = Some(pod);
        self.log_container = None;
        self.log_time_range = TimeRange::M30;
        self.changed(false, cx);
    }

    pub fn close_log_viewer(&mut self, cx: &mut Context<Self>) {
        self.view = View::Pods;
        self.log_viewer_pod = None;
        self.changed(false, cx);
    }

    pub fn select_pod(&mut self, pod: Option<String>, cx: &mut Context<Self>) {
        self.drawer = pod.map(DrawerKind::Pod);
        self.changed(false, cx);
    }

    pub fn select_deployment_info(&mut self, deployment: Option<String>, cx: &mut Context<Self>) {
        self.drawer = deployment.map(DrawerKind::Deployment);
        self.changed(false, cx);
    }

    pub fn open_settings(&mut self, cx: &mut Context<Self>) {
        self.drawer = Some(DrawerKind::Settings);
        self.changed(false, cx);
    }

    pub fn close_drawer(&mut self, cx: &mut Context<Self>) {
        self.drawer = None;
        self.changed(false, cx);
    }

    pub fn set_theme(&mut self, theme: ThemePref, cx: &mut Context<Self>) {
        self.theme = theme;
        self.changed(true, cx);
    }

    /// Dark → Light, anything else → Dark.
    pub fn toggle_theme(&mut self, cx: &mut Context<Self>) {
        let next = if self.theme == ThemePref::Dark { ThemePref::Light } else { ThemePref::Dark };
        self.set_theme(next, cx);
    }

    pub fn set_os_dark(&mut self, dark: bool, cx: &mut Context<Self>) {
        self.os_dark = dark;
        self.changed(false, cx);
    }

    pub fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_collapsed = !self.sidebar_collapsed;
        self.changed(true, cx);
    }

    pub fn set_refresh_interval(&mut self, ms: u32, cx: &mut Context<Self>) {
        self.refresh_interval_ms = ms;
        self.countdown_s = ms / 1000;
        self.changed(true, cx);
    }

    pub fn set_log_container(&mut self, container: String, cx: &mut Context<Self>) {
        self.log_container = Some(container);
        self.changed(false, cx);
    }

    pub fn set_log_time_range(&mut self, range: TimeRange, cx: &mut Context<Self>) {
        self.log_time_range = range;
        self.changed(false, cx);
    }

    pub fn dismiss_error(&mut self, cx: &mut Context<Self>) {
        self.error = None;
        cx.notify();
    }

    /// Retry from the Connection Error dialog: forget the core queries and refetch them now.
    pub fn retry_connection(&mut self, cx: &mut Context<Self>) {
        self.error = None;
        self.clusters.clear();
        self.namespaces.clear();
        self.deployments.clear();
        self.pods.clear();
        self.changed(false, cx);
    }
}

#[cfg(test)]
mod tests {
    // Explicit imports: a `gpui_kit::*` glob would shadow `#[test]` with gpui's `test` macro.
    use std::sync::Arc;
    use std::time::Duration;

    use std::path::PathBuf;

    use gpui_kit::{AppContext, Context, Entity, TestAppContext};
    use serde_json::{Value, json};

    use super::{DrawerKind, Store, View};
    use crate::persist::{self, PersistedState, ThemePref};
    use crate::api::fake::{Endpoint as E, FakeApi};

    fn clusters(api: &FakeApi, names: &[(&str, bool)]) {
        let v: Vec<Value> = names.iter().map(|(n, cur)| json!({"name": n, "server": "s", "is_current": cur})).collect();
        api.reply(E::Clusters, Ok(Value::Array(v)));
    }

    fn namespaces(api: &FakeApi, names: &[&str]) {
        let v: Vec<Value> = names.iter().map(|n| json!({"name": n, "status": "Active"})).collect();
        api.reply(E::Namespaces, Ok(Value::Array(v)));
    }

    fn pod(name: &str) -> Value {
        json!({"name": name, "namespace": "n", "status": "Running", "ready": "1/1", "restarts": 0,
               "age": "1d", "ip": "1.1.1.1", "node": "n1", "containers": []})
    }

    fn deployment(name: &str) -> Value {
        json!({"name": name, "namespace": "n", "replicas": 1, "available_replicas": 1, "ready_replicas": 1})
    }

    fn details(name: &str, containers: &[&str]) -> Value {
        let cs: Vec<Value> = containers
            .iter()
            .map(|c| {
                json!({"name": c, "image": "img", "ready": true, "restart_count": 0, "state": "Running",
                       "env_vars": [], "ports": [],
                       "resources": {"cpu_request": null, "cpu_limit": null, "memory_request": null, "memory_limit": null}})
            })
            .collect();
        json!({"name": name, "namespace": "n", "status": "Running", "node": "n1", "ip": "1.1.1.1",
               "start_time": null, "labels": {}, "annotations": {}, "containers": cs, "conditions": []})
    }

    /// `n` identical successful replies, so background polling never hits "no fake reply".
    fn many(api: &FakeApi, ep: E, v: Value, n: usize) {
        for _ in 0..n {
            api.reply(ep, Ok(v.clone()));
        }
    }

    fn persisted(context: &str, namespace: &str) -> PersistedState {
        PersistedState { context: context.into(), namespace: namespace.into(), ..Default::default() }
    }

    fn start(cx: &mut TestAppContext, api: &Arc<FakeApi>, p: PersistedState) -> Entity<Store> {
        start_at(cx, api, p, None)
    }

    fn start_at(cx: &mut TestAppContext, api: &Arc<FakeApi>, p: PersistedState, path: Option<PathBuf>) -> Entity<Store> {
        let api = api.clone();
        let store = cx.new(|cx| Store::new(api, p, path, cx));
        cx.run_until_parked();
        store
    }

    /// Advance the fake clock one second at a time so the 1 s tick loop re-arms between steps.
    fn advance(cx: &mut TestAppContext, secs: u64) {
        for _ in 0..secs {
            cx.executor().advance_clock(Duration::from_secs(1));
            cx.run_until_parked();
        }
    }

    fn act(cx: &mut TestAppContext, store: &Entity<Store>, f: impl FnOnce(&mut Store, &mut Context<Store>)) {
        store.update(cx, f);
        cx.run_until_parked();
    }

    fn read<R>(cx: &TestAppContext, store: &Entity<Store>, f: impl FnOnce(&Store) -> R) -> R {
        store.read_with(cx, |s, _| f(s))
    }

    #[gpui_kit::test]
    fn startup_picks_current_context_and_fetches_namespaces(cx: &mut TestAppContext) {
        let api = Arc::new(FakeApi::default());
        clusters(&api, &[("a", false), ("b", true)]);
        namespaces(&api, &["default"]);
        let store = start(cx, &api, PersistedState::default());
        assert_eq!(read(cx, &store, |s| s.context.clone()), "b");
        assert_eq!(api.calls(E::Namespaces), 1);
        assert_eq!(api.last_args(E::Namespaces), vec!["\"b\"".to_string()]);
        assert_eq!(read(cx, &store, |s| s.error.clone()), None);
    }

    #[gpui_kit::test]
    fn namespace_fallback_to_default_then_first(cx: &mut TestAppContext) {
        let api = Arc::new(FakeApi::default());
        clusters(&api, &[("c", true)]);
        namespaces(&api, &["kube-system", "default"]);
        let store = start(cx, &api, persisted("c", "gone"));
        assert_eq!(read(cx, &store, |s| s.namespace.clone()), "default");

        let api = Arc::new(FakeApi::default());
        clusters(&api, &[("c", true)]);
        namespaces(&api, &["x", "y"]);
        let store = start(cx, &api, persisted("c", "gone"));
        assert_eq!(read(cx, &store, |s| s.namespace.clone()), "x");
    }

    #[gpui_kit::test]
    fn store_reports_missing_kubeconfig_on_startup(cx: &mut TestAppContext) {
        let api = Arc::new(FakeApi::default());
        api.reply(E::Clusters, Err("No kubeconfig found".into()));
        let store = start(cx, &api, PersistedState::default());
        assert_eq!(read(cx, &store, |s| s.error.clone()), Some("No kubeconfig found".to_string()));
    }

    #[gpui_kit::test]
    fn refetch_error_with_cache_is_silent(cx: &mut TestAppContext) {
        let api = Arc::new(FakeApi::default());
        clusters(&api, &[("c", true)]);
        namespaces(&api, &["default"]);
        many(&api, E::Deployments, json!([deployment("d")]), 5);
        api.reply(E::Pods, Ok(json!([pod("p")])));
        api.reply(E::Pods, Err("boom".into()));
        let store = start(cx, &api, persisted("c", "default"));
        assert_eq!(read(cx, &store, |s| s.pods.data().map(|p| p.len())), Some(1));
        advance(cx, 5);
        assert_eq!(api.calls(E::Pods), 2);
        assert_eq!(read(cx, &store, |s| s.pods.error().map(str::to_owned)), Some("boom".to_string()));
        assert_eq!(read(cx, &store, |s| s.error.clone()), None);
        assert_eq!(read(cx, &store, |s| s.pods.data().map(|p| p.len())), Some(1));
    }

    #[gpui_kit::test]
    fn errors_ignored_during_transition(cx: &mut TestAppContext) {
        let api = Arc::new(FakeApi::default());
        clusters(&api, &[("c", true)]);
        namespaces(&api, &["default", "prod"]);
        many(&api, E::Deployments, json!([deployment("d")]), 8);
        api.reply(E::Pods, Ok(json!([pod("p")])));
        let store = start(cx, &api, persisted("c", "default"));
        // First poll for "prod" fails inside the 1 s window; the next one fails after it.
        api.reply(E::Pods, Err("boom".into()));
        api.reply(E::Pods, Err("boom again".into()));
        act(cx, &store, |s, cx| s.set_namespace("prod".into(), cx));
        assert_eq!(api.calls(E::Pods), 2);
        assert_eq!(read(cx, &store, |s| s.pods.error().map(str::to_owned)), Some("boom".to_string()));
        assert_eq!(read(cx, &store, |s| s.error.clone()), None);
        advance(cx, 5);
        assert_eq!(read(cx, &store, |s| s.error.clone()), Some("boom again".to_string()));
    }

    #[gpui_kit::test]
    fn late_pods_after_namespace_switch_are_dropped(cx: &mut TestAppContext) {
        let api = Arc::new(FakeApi::default());
        clusters(&api, &[("c", true)]);
        namespaces(&api, &["A", "B"]);
        many(&api, E::Deployments, json!([deployment("d")]), 4);
        let gate_a = api.gate(E::Pods);
        api.reply(E::Pods, Ok(json!([pod("pod-b")])));
        let store = start(cx, &api, persisted("c", "A"));
        assert_eq!(api.calls(E::Pods), 1);
        act(cx, &store, |s, cx| s.set_namespace("B".into(), cx));
        assert_eq!(api.calls(E::Pods), 2);
        gate_a.send(Ok(json!([pod("pod-a")]))).unwrap();
        cx.run_until_parked();
        let names = read(cx, &store, |s| s.pods.data().map(|p| p.iter().map(|p| p.name.clone()).collect::<Vec<_>>()));
        assert_eq!(names, Some(vec!["pod-b".to_string()]));
        assert_eq!(read(cx, &store, |s| s.error.clone()), None);
    }

    #[gpui_kit::test]
    fn slow_fetch_does_not_overlap_and_resets_countdown(cx: &mut TestAppContext) {
        let api = Arc::new(FakeApi::default());
        clusters(&api, &[("c", true)]);
        namespaces(&api, &["default"]);
        many(&api, E::Deployments, json!([deployment("d")]), 4);
        let gate = api.gate(E::Pods);
        let store = start(cx, &api, persisted("c", "default")); // refresh interval 5 s
        advance(cx, 12);
        assert_eq!(api.calls(E::Pods), 1);
        assert_eq!(read(cx, &store, |s| s.countdown_s), 0);
        gate.send(Ok(json!([pod("p")]))).unwrap();
        cx.run_until_parked();
        assert_eq!(read(cx, &store, |s| s.countdown_s), 5);
    }

    #[gpui_kit::test]
    fn namespace_change_returns_to_deployments_and_closes_pod_drawer(cx: &mut TestAppContext) {
        let api = Arc::new(FakeApi::default());
        clusters(&api, &[("c", true), ("d", false)]);
        namespaces(&api, &["default", "prod"]);
        let store = start(cx, &api, persisted("c", "default"));
        act(cx, &store, |s, cx| {
            s.set_view(View::Pods, cx);
            s.select_pod(Some("p".into()), cx);
        });
        assert_eq!(read(cx, &store, |s| (s.view, s.drawer.clone())), (View::Pods, Some(DrawerKind::Pod("p".into()))));
        act(cx, &store, |s, cx| s.set_namespace("prod".into(), cx));
        assert_eq!(read(cx, &store, |s| (s.view, s.drawer.clone())), (View::Deployments, None));

        // Same for a context change, and a non-pod drawer stays open.
        act(cx, &store, |s, cx| {
            s.set_view(View::Pods, cx);
            s.open_settings(cx);
        });
        act(cx, &store, |s, cx| s.set_context("d".into(), cx));
        assert_eq!(read(cx, &store, |s| (s.view, s.drawer.clone())), (View::Deployments, Some(DrawerKind::Settings)));
        assert_eq!(read(cx, &store, |s| (s.context.clone(), s.namespace.clone(), s.deployment.clone())), ("d".into(), "default".into(), None));
    }

    #[gpui_kit::test]
    fn log_container_defaults_to_first_container(cx: &mut TestAppContext) {
        let api = Arc::new(FakeApi::default());
        clusters(&api, &[("c", true)]);
        namespaces(&api, &["default"]);
        many(&api, E::Deployments, json!([deployment("d")]), 4);
        many(&api, E::Pods, json!([pod("p")]), 4);
        api.reply(E::PodDetails, Ok(details("p", &["app", "sidecar"])));
        api.reply(E::PodLogs, Ok(json!([])));
        let store = start(cx, &api, persisted("c", "default"));
        act(cx, &store, |s, cx| s.open_log_viewer("p".into(), cx));
        assert_eq!(read(cx, &store, |s| s.log_container.clone()), Some("app".to_string()));
        assert_eq!(api.calls(E::PodLogs), 1);
        assert!(api.last_args(E::PodLogs).contains(&"Some(\"app\")".to_string()));
        assert!(api.last_args(E::PodLogs).contains(&"Some(1800)".to_string()));
    }

    #[gpui_kit::test]
    fn pod_logs_inactive_without_container(cx: &mut TestAppContext) {
        let api = Arc::new(FakeApi::default());
        clusters(&api, &[("c", true)]);
        namespaces(&api, &["default"]);
        many(&api, E::Deployments, json!([deployment("d")]), 8);
        many(&api, E::Pods, json!([pod("p")]), 8);
        api.reply(E::PodDetails, Ok(details("p", &[])));
        let store = start(cx, &api, persisted("c", "default"));
        act(cx, &store, |s, cx| s.open_log_viewer("p".into(), cx));
        advance(cx, 20);
        assert_eq!(api.calls(E::PodDetails), 1);
        assert_eq!(api.calls(E::PodLogs), 0);
        assert_eq!(read(cx, &store, |s| s.log_container.clone()), None);
    }

    #[gpui_kit::test]
    fn pod_logs_retries_once_then_reports(cx: &mut TestAppContext) {
        let api = Arc::new(FakeApi::default());
        clusters(&api, &[("c", true)]);
        namespaces(&api, &["default"]);
        many(&api, E::Deployments, json!([deployment("d")]), 4);
        many(&api, E::Pods, json!([pod("p")]), 4);
        api.reply(E::PodDetails, Ok(details("p", &["app"])));
        api.reply(E::PodLogs, Err("first".into()));
        api.reply(E::PodLogs, Err("second".into()));
        let store = start(cx, &api, persisted("c", "default"));
        act(cx, &store, |s, cx| s.open_log_viewer("p".into(), cx));
        assert_eq!(api.calls(E::PodLogs), 1);
        assert_eq!(read(cx, &store, |s| s.error.clone()), None);
        advance(cx, 1);
        assert_eq!(api.calls(E::PodLogs), 2);
        assert_eq!(read(cx, &store, |s| s.error.clone()), Some("second".to_string()));
    }

    #[gpui_kit::test]
    fn retry_connection_refetches_core_queries(cx: &mut TestAppContext) {
        let api = Arc::new(FakeApi::default());
        for _ in 0..2 {
            clusters(&api, &[("c", true)]);
            namespaces(&api, &["default"]);
        }
        many(&api, E::Deployments, json!([deployment("d")]), 2);
        many(&api, E::Pods, json!([pod("p")]), 2);
        let store = start(cx, &api, persisted("c", "default"));
        for ep in [E::Clusters, E::Namespaces, E::Deployments, E::Pods] {
            assert_eq!(api.calls(ep), 1, "{ep:?}");
        }
        act(cx, &store, |s, cx| {
            s.error = Some("old".into());
            s.retry_connection(cx);
        });
        for ep in [E::Clusters, E::Namespaces, E::Deployments, E::Pods] {
            assert_eq!(api.calls(ep), 2, "{ep:?}");
        }
        assert_eq!(read(cx, &store, |s| s.error.clone()), None);
    }

    #[gpui_kit::test]
    fn toggle_theme_cycle(cx: &mut TestAppContext) {
        let api = Arc::new(FakeApi::default());
        let store = start(cx, &api, PersistedState { theme: ThemePref::System, ..Default::default() });
        let mut seen = vec![read(cx, &store, |s| s.theme)];
        for _ in 0..3 {
            act(cx, &store, |s, cx| s.toggle_theme(cx));
            seen.push(read(cx, &store, |s| s.theme));
        }
        assert_eq!(seen, [ThemePref::System, ThemePref::Dark, ThemePref::Light, ThemePref::Dark]);
        act(cx, &store, |s, cx| s.set_theme(ThemePref::System, cx));
        assert!(!read(cx, &store, Store::is_dark));
        act(cx, &store, |s, cx| s.set_os_dark(true, cx));
        assert!(read(cx, &store, Store::is_dark));
    }

    #[gpui_kit::test]
    fn persists_changes_to_disk(cx: &mut TestAppContext) {
        let dir = std::env::temp_dir().join(format!("podlogs-store-{}", std::process::id()));
        let path = dir.join("state.json");
        let api = Arc::new(FakeApi::default());
        clusters(&api, &[("c", true)]);
        namespaces(&api, &["default", "prod"]);
        let store = start_at(cx, &api, persisted("c", "default"), Some(path.clone()));
        act(cx, &store, |s, cx| s.set_namespace("prod".into(), cx));
        let saved = persist::load(&path);
        assert_eq!(saved.namespace, "prod");
        assert_eq!(saved.last_namespace_by_cluster.get("c").map(String::as_str), Some("prod"));
        act(cx, &store, |s, cx| s.toggle_sidebar(cx));
        assert!(persist::load(&path).sidebar_collapsed);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
