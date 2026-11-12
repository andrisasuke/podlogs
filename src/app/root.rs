//! `MainLayout.tsx`: sidebar + main view + status bar, with drawers and dialogs overlaid.
use std::rc::Rc;

use gpui_kit::component::menu::AppMenuBar;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::error_dialog::connection_error;
use super::deployment_details::DeploymentDetailsView;
use super::deployments::DeploymentsView;
use super::log_viewer::LogViewerView;
use super::pod_details::PodDetailsView;
use super::pods::PodsView;
use super::settings::SettingsView;
use super::sidebar::SidebarView;
use super::status_bar::status_bar;
use super::store::{DrawerKind, Store, View};
use crate::ui::drawer::drawer;
use crate::ui::modal::{ModalSize, OnClose, modal};
use crate::ui::theme::{Tw, UI_FONT, palette, pal, set_palette};

/// Lets app-level actions (the About menu item) reach the root view.
struct RootHandle(WeakEntity<PodLogsApp>);
impl Global for RootHandle {}

pub fn show_about(cx: &mut App) {
    if let Some(root) = cx.try_global::<RootHandle>().and_then(|h| h.0.upgrade()) {
        root.update(cx, |r, cx| {
            r.about_open = true;
            r.about_epoch += 1; // fresh overlay focus state, so the modal grabs focus on every open
            cx.notify();
        });
    }
}

pub struct PodLogsApp {
    pub store: Entity<Store>,
    pub sidebar: Entity<SidebarView>,
    pub(crate) deployments: Entity<DeploymentsView>,
    pub(crate) pods: Entity<PodsView>,
    pod_details: Entity<PodDetailsView>,
    pub(crate) log_viewer: Entity<LogViewerView>,
    deployment_details: Entity<DeploymentDetailsView>,
    settings: Entity<SettingsView>,
    menu_bar: Option<Entity<AppMenuBar>>,
    about_open: bool,
    about_epoch: usize,
    _subs: Vec<Subscription>,
}

fn is_dark(a: WindowAppearance) -> bool {
    matches!(a, WindowAppearance::Dark | WindowAppearance::VibrantDark)
}

impl PodLogsApp {
    pub fn new(store: Entity<Store>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        cx.set_global(RootHandle(cx.entity().downgrade()));
        let sidebar = cx.new(|cx| SidebarView::new(store.clone(), window, cx));
        let deployments = cx.new(|cx| DeploymentsView::new(store.clone(), window, cx));
        let pods = cx.new(|cx| PodsView::new(store.clone(), window, cx));
        let pod_details = cx.new(|cx| PodDetailsView::new(store.clone(), window, cx));
        let log_viewer = cx.new(|cx| LogViewerView::new(store.clone(), window, cx));
        let deployment_details = cx.new(|cx| DeploymentDetailsView::new(store.clone(), window, cx));
        let settings = cx.new(|cx| SettingsView::new(store.clone(), window, cx));
        let menu_bar = (!cfg!(target_os = "macos")).then(|| super::menus::menu_bar(window, cx));
        let os_dark = store.clone();
        let subs = vec![
            cx.observe(&store, |this, _, cx| {
                this.apply_palette(cx);
                cx.notify();
            }),
            window.observe_window_appearance(move |window, cx| {
                let dark = is_dark(window.appearance());
                os_dark.update(cx, |s, cx| s.set_os_dark(dark, cx));
            }),
        ];
        let dark = is_dark(window.appearance());
        store.update(cx, |s, cx| s.set_os_dark(dark, cx));
        let mut this = Self { store, sidebar, deployments, pods, pod_details, log_viewer, deployment_details, settings, menu_bar, about_open: false, about_epoch: 0, _subs: subs };
        this.apply_palette(cx);
        this
    }

    /// Theme changed (user pick or OS appearance): push the palette only when it actually differs.
    fn apply_palette(&mut self, cx: &mut Context<Self>) {
        let p = palette(self.store.read(cx).is_dark());
        if pal(cx) != p {
            set_palette(p, cx);
        }
    }
}

impl Render for PodLogsApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = pal(cx);
        let status = status_bar(self.store.read(cx), cx);
        let (view, drawer_kind, error) = {
            let s = self.store.read(cx);
            (s.view, s.drawer.clone(), s.error.clone())
        };
        let store = self.store.clone();
        let close_drawer: OnClose = Rc::new(move |_, cx| store.update(cx, |s, cx| s.close_drawer(cx)));

        let main = match view {
            View::Deployments => self.deployments.clone().into_any_element(),
            View::Pods => self.pods.clone().into_any_element(),
            View::Logs => self.log_viewer.clone().into_any_element(),
            View::Search => div().size_full().into_any_element(),
        };

        let pod_title = match &drawer_kind {
            Some(DrawerKind::Pod(n)) => n.clone(),
            _ => String::new(),
        };
        let pod_drawer = drawer(
            "drawer-pod",
            &pod_title,
            matches!(drawer_kind, Some(DrawerKind::Pod(_))),
            close_drawer.clone(),
            self.pod_details.clone().into_any_element(),
            window,
            cx,
        );

        let deployment_title = match &drawer_kind {
            Some(DrawerKind::Deployment(name)) => name.as_str(),
            _ => "",
        };
        let deployment_drawer = drawer(
            "drawer-deployment", deployment_title,
            matches!(drawer_kind, Some(DrawerKind::Deployment(_))),
            close_drawer.clone(), self.deployment_details.clone().into_any_element(), window, cx,
        );
        let settings_drawer = drawer(
            "drawer-settings", "Settings", matches!(drawer_kind, Some(DrawerKind::Settings)),
            close_drawer, self.settings.clone().into_any_element(), window, cx,
        );

        let about = self.about_open.then(|| {
            let this = cx.entity();
            let close: OnClose = Rc::new(move |_, cx| {
                this.update(cx, |r, cx| {
                    r.about_open = false;
                    cx.notify();
                })
            });
            let body = div()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(8.))
                .p(px(24.))
                .child(img("assets/icon_256x256.png").size(px(64.)))
                .child(div().t_lg().font_weight(FontWeight::SEMIBOLD).text_color(p.text1).child("PodLogs"))
                .child(div().t_sm().text_color(p.text2).child(format!("Version {}", env!("CARGO_PKG_VERSION"))))
                .into_any_element();
            modal(("about", self.about_epoch), "About PodLogs", ModalSize::Sm, false, close, body, window, cx)
        });

        let error_dialog = error.map(|raw| {
            let (a, b) = (self.store.clone(), self.store.clone());
            connection_error(
                &raw,
                Rc::new(move |_, cx| a.update(cx, |s, cx| s.dismiss_error(cx))),
                Rc::new(move |_, cx| b.update(cx, |s, cx| s.retry_connection(cx))),
                window,
                cx,
            )
        });

        div()
            .size_full()
            .flex()
            .flex_col()
            .font_family(UI_FONT)
            .text_size(px(14.))
            .line_height(relative(1.5))
            .text_color(p.text1)
            .bg(p.bg1)
            .when_some(self.menu_bar.clone(), |d, bar| {
                d.child(div().h(px(32.)).flex_none().border_b_1().border_color(p.border).bg(p.bg2).child(bar))
            })
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(self.sidebar.clone())
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .child(div().flex_1().min_h_0().overflow_hidden().child(main))
                            .child(status),
                    ),
            )
            .child(pod_drawer)
            .child(deployment_drawer)
            .child(settings_drawer)
            .children(about)
            .children(error_dialog)
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{AppContext, TestAppContext};
    use serde_json::json;

    use crate::api::fake::Endpoint as E;
    use crate::app::store::View;
    use crate::app::testing::{boot, deployment, many, seed, seeded, temp_state};
    use crate::persist;

    macro_rules! ui {
        ($cx:expr, $env:expr, |$w:ident, $c:ident| $body:expr) => {
            $cx.update_window($env.handle, |_, $w, $c| $body).unwrap()
        };
    }

    fn st<R>(cx: &TestAppContext, env: &crate::app::testing::Env, f: impl FnOnce(&crate::app::store::Store) -> R) -> R {
        env.store.read_with(cx, |s, _| f(s))
    }

    #[gpui_kit::test]
    fn sidebar_click_deployment_selects_and_shows_pods_then_toggles_off(cx: &mut TestAppContext) {
        let env = boot(cx, seeded(vec![deployment("api", 1, 1, 1), deployment("web", 1, 1, 1)]), None);
        ui!(cx, env, |w, c| {
            w.render_frame(c);
            w.click("dep-api", c);
        });
        assert_eq!(st(cx, &env, |s| (s.deployment.clone(), s.view)), (Some("api".into()), View::Pods));
        ui!(cx, env, |w, c| w.click("dep-api", c));
        assert_eq!(st(cx, &env, |s| (s.deployment.clone(), s.view)), (None, View::Pods));
    }

    #[gpui_kit::test]
    fn sidebar_filter_hides_non_matching_and_shows_empty_text(cx: &mut TestAppContext) {
        let env = boot(cx, seeded(vec![deployment("api", 1, 1, 1), deployment("web", 1, 1, 1)]), None);
        let filter = env.app.read_with(cx, |a, cx| a.sidebar.read(cx).filter.clone());
        ui!(cx, env, |w, c| {
            w.render_frame(c);
            filter.update(c, |f, c| f.focus(w, c));
            w.input("AP", c); // case-insensitive
            assert!(w.try_find("dep-api").is_some());
            assert!(w.try_find("dep-web").is_none());
            assert!(w.try_find("sidebar-empty").is_none());
            w.input("zzz", c);
            assert!(w.try_find("dep-api").is_none());
            assert!(w.try_find("sidebar-empty").is_some());
        });
    }

    #[gpui_kit::test]
    fn collapse_toggle_switches_rail_and_persists(cx: &mut TestAppContext) {
        let path = temp_state("collapse");
        let env = boot(cx, seeded(vec![]), Some(path.clone()));
        ui!(cx, env, |w, c| {
            w.render_frame(c);
            assert!(w.try_find("sidebar-expanded").is_some() && w.try_find("sidebar-rail").is_none());
            w.click("collapse-sidebar", c);
            assert!(w.try_find("sidebar-expanded").is_none() && w.try_find("sidebar-rail").is_some());
        });
        assert!(persist::load(&path).sidebar_collapsed);
        ui!(cx, env, |w, c| {
            w.click("expand-sidebar", c);
            assert!(w.try_find("sidebar-expanded").is_some() && w.try_find("sidebar-rail").is_none());
        });
        assert!(!persist::load(&path).sidebar_collapsed);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[gpui_kit::test]
    fn status_bar_countdown_hidden_while_fetching(cx: &mut TestAppContext) {
        let api = std::sync::Arc::new(crate::api::fake::FakeApi::default());
        let gate = api.gate(E::Pods); // the first pods fetch stays in flight
        seed(&api, vec![]);
        let env = boot(cx, api.clone(), None);
        ui!(cx, env, |w, c| {
            w.render_frame(c);
            assert!(w.try_find("status-syncing").is_some());
            assert!(w.try_find("status-live").is_none());
            assert!(w.try_find("status-countdown").is_none());
        });
        gate.send(Ok(json!([]))).unwrap();
        cx.run_until_parked();
        ui!(cx, env, |w, c| {
            w.render_frame(c);
            assert!(w.try_find("status-live").is_some());
            assert!(w.try_find("status-countdown").is_some());
        });
        assert_eq!(st(cx, &env, |s| s.countdown_s), 5);
    }

    #[gpui_kit::test]
    fn error_dialog_shows_mapped_message_and_retry_refetches(cx: &mut TestAppContext) {
        let api = std::sync::Arc::new(crate::api::fake::FakeApi::default());
        api.reply(E::Clusters, Err("No kubeconfig found".into()));
        many(&api, E::Clusters, json!([{"name": "c1", "server": "s", "is_current": true}]), 5);
        many(&api, E::Namespaces, json!([{"name": "default", "status": "Active"}]), 50);
        many(&api, E::Deployments, json!([]), 50);
        many(&api, E::Pods, json!([]), 50);
        let env = boot(cx, api.clone(), None);
        assert_eq!(st(cx, &env, |s| s.error.clone()), Some("No kubeconfig found".into()));
        ui!(cx, env, |w, c| {
            w.render_frame(c);
            assert_eq!(w.find("err-message").label(), Some("Kubernetes configuration not found"));
            assert_eq!(w.find("err-details").label(), Some("Please ensure kubectl is configured and ~/.kube/config exists."));
            w.click("err-retry", c);
        });
        cx.run_until_parked();
        assert_eq!(api.calls(E::Clusters), 2);
        assert_eq!(st(cx, &env, |s| s.error.clone()), None);
        // Dismiss closes the dialog without refetching.
        env.store.update(cx, |s, cx| {
            s.error = Some("Connection timed out after 30 seconds".into());
            cx.notify();
        });
        ui!(cx, env, |w, c| {
            w.render_frame(c);
            assert_eq!(w.find("err-message").label(), Some("Connection timed out"));
            w.click("err-dismiss", c);
        });
        assert_eq!(st(cx, &env, |s| s.error.clone()), None);
        ui!(cx, env, |w, c| {
            w.render_frame(c);
            assert!(w.try_find("err-message").is_none());
        });
    }

    #[gpui_kit::test]
    fn escape_closes_drawer_but_not_error_dialog(cx: &mut TestAppContext) {
        let api = seeded(vec![]);
        many(&api, E::PodDetails, json!({"name": "p1", "namespace": "default", "status": "Running", "node": "n", "ip": "1.1.1.1",
            "start_time": null, "labels": {}, "annotations": {}, "containers": [], "conditions": []}), 5);
        let env = boot(cx, api, None);
        env.store.update(cx, |s, cx| {
            s.select_pod(Some("p1".into()), cx);
            s.error = Some("boom".into());
        });
        ui!(cx, env, |w, c| {
            w.render_frame(c);
            w.render_frame(c);
            assert!(w.try_find("err-message").is_some());
            w.press("escape", c);
        });
        assert_eq!(st(cx, &env, |s| (s.drawer.clone(), s.error.clone())), (None, Some("boom".into())));
        ui!(cx, env, |w, c| {
            w.render_frame(c);
            w.press("escape", c);
        });
        assert_eq!(st(cx, &env, |s| s.error.clone()), Some("boom".into()));
    }
}
