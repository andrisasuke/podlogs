pub mod deployment_details;
pub mod deployments;
pub mod error_dialog;
pub mod log_detail_modal;
pub mod log_viewer;
pub mod menus;
pub mod pod_details;
pub mod pods;
pub mod query;
pub mod root;
pub mod settings;
pub mod sidebar;
pub mod status_bar;
pub mod store;

/// Shared harness for the headless UI tests of the app shell and screens (FakeApi-backed Store).
/// Imports are explicit: a `gpui_kit::*` glob would shadow `#[test]` with gpui's own `test` macro.
#[cfg(test)]
pub mod testing {
    use std::{path::PathBuf, sync::Arc, time::Duration};

    use gpui_kit::{AnyWindowHandle, AppContext, Bounds, Entity, TestAppContext, WindowBounds, WindowOptions, point, px, size};
    use serde_json::{Value, json};

    use super::{root::PodLogsApp, store::Store};
    use crate::api::fake::{Endpoint as E, FakeApi};
    use crate::persist::PersistedState;
    use crate::ui::theme::{palette, set_palette};

    pub struct Env {
        pub api: Arc<FakeApi>,
        pub store: Entity<Store>,
        pub app: Entity<PodLogsApp>,
        pub handle: AnyWindowHandle,
    }

    pub fn many(api: &FakeApi, ep: E, v: Value, n: usize) {
        for _ in 0..n {
            api.reply(ep, Ok(v.clone()));
        }
    }

    pub fn deployment(name: &str, replicas: i32, available: i32, ready: i32) -> Value {
        json!({"name": name, "namespace": "default", "replicas": replicas, "available_replicas": available, "ready_replicas": ready})
    }

    pub fn pod(name: &str, status: &str) -> Value {
        json!({"name": name, "namespace": "default", "status": status, "ready": "1/1",
            "restarts": 0, "age": "2h", "ip": "10.0.0.1", "node": "worker-1", "containers": ["app"]})
    }

    pub fn with_pods(pods: Vec<Value>) -> Arc<FakeApi> {
        let api = Arc::new(FakeApi::default());
        many(&api, E::Pods, Value::Array(pods), 50);
        seed(&api, vec![]);
        api
    }

    /// One cluster "c1" (current), namespace "default", the given deployments, no pods; replies queued for a long session.
    pub fn seeded(deployments: Vec<Value>) -> Arc<FakeApi> {
        let api = Arc::new(FakeApi::default());
        seed(&api, deployments);
        api
    }

    pub fn seed(api: &FakeApi, deployments: Vec<Value>) {
        many(api, E::Clusters, json!([{"name": "c1", "server": "s", "is_current": true}]), 5);
        many(api, E::Namespaces, json!([{"name": "default", "status": "Active"}]), 50);
        many(api, E::Deployments, Value::Array(deployments), 50);
        many(api, E::Pods, json!([]), 50);
    }

    pub fn temp_state(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("podlogs-ui-{}-{name}", std::process::id())).join("state.json")
    }

    pub fn persisted() -> PersistedState {
        PersistedState { context: "c1".into(), namespace: "default".into(), ..Default::default() }
    }

    pub fn boot(cx: &mut TestAppContext, api: Arc<FakeApi>, persist_path: Option<PathBuf>) -> Env {
        let a = api.clone();
        let (handle, app) = cx.update(|cx| {
            gpui_kit::init(cx);
            crate::ui::init(cx);
            set_palette(palette(true), cx);
            let bounds = Bounds { origin: point(px(0.), px(0.)), size: size(px(1200.), px(800.)) };
            let (w, v) = gpui_kit::open_window(
                WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), ..Default::default() },
                cx,
                move |window, cx| {
                    let store = cx.new(|cx| Store::new(a, persisted(), persist_path, cx));
                    cx.new(|cx| PodLogsApp::new(store, window, cx))
                },
            )
            .expect("open test window");
            (w, v)
        });
        cx.run_until_parked();
        let store = app.read_with(cx, |a, _| a.store.clone());
        Env { api, store, app, handle }
    }

    /// Advance the test clock past the 300 ms drawer animation.
    pub fn settle_animation(cx: &mut TestAppContext) {
        cx.executor().advance_clock(Duration::from_millis(350));
        cx.run_until_parked();
    }
}
