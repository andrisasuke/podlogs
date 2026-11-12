//! Pod overview, selectable values, container cards and collapsed disclosures.
use std::collections::HashSet;

use gpui_kit::assets::IconName;
use gpui_kit::component::Icon;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::store::{DrawerKind, Store};
use crate::format::format_timestamp;
use crate::models::{ContainerDetails, EnvVar, PodDetails};
use crate::ui::badge::status_badge;
use crate::ui::collapsible::collapsible_header;
use crate::ui::theme::{Tw, hex, pal, tw};
use crate::ui::{section_label, selectable_text};

pub struct PodDetailsView {
    store: Entity<Store>,
    scope: Option<(String, String, String)>,
    open_env: HashSet<String>,
    conditions_open: bool,
    _sub: Subscription,
}

/// Mask before constructing any element, selectable text or accessibility value.
pub fn displayed_env_value(env: &EnvVar) -> &str {
    if env.source == "Secret" { "<secret>" } else { &env.value }
}

impl PodDetailsView {
    pub fn new(store: Entity<Store>, _: &mut Window, cx: &mut Context<Self>) -> Self {
        let scope = pod_scope(store.read(cx));
        let sub = cx.observe(&store, |this, _, cx| {
            let scope = pod_scope(this.store.read(cx));
            if this.scope != scope {
                this.scope = scope;
                this.open_env.clear();
                this.conditions_open = false;
            }
            cx.notify();
        });
        Self { store, scope, open_env: HashSet::new(), conditions_open: false, _sub: sub }
    }

    fn container(&self, container: &ContainerDetails, cx: &mut Context<Self>) -> AnyElement {
        let p = pal(cx);
        let mut info = div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .t_sm()
            .child(container_row(
                "Image",
                div()
                    .min_w_0()
                    .flex_1()
                    .mono()
                    .text_color(p.text2)
                    .child(selectable_text(format!("container-image-{}", container.name), &container.image)),
                false,
                cx,
            ))
            .child(container_row("Restarts", div().text_color(p.text2).child(container.restart_count.to_string()), true, cx));
        let r = &container.resources;
        let present = |s: &Option<String>| s.as_ref().is_some_and(|s| !s.is_empty());
        let value = |s: &Option<String>| s.as_deref().filter(|s| !s.is_empty()).unwrap_or("-").to_owned();
        if present(&r.cpu_request) || present(&r.memory_request) {
            info = info.child(container_row(
                "Resources",
                div().min_w_0().flex_1().mono().text_color(p.text2).child(format!(
                    "CPU: {}/{} · Mem: {}/{}",
                    value(&r.cpu_request),
                    value(&r.cpu_limit),
                    value(&r.memory_request),
                    value(&r.memory_limit),
                )),
                true,
                cx,
            ));
        }
        if !container.ports.is_empty() {
            let ports =
                container.ports.iter().map(|port| format!("{}/{}", port.container_port, port.protocol)).collect::<Vec<_>>().join(", ");
            info = info.child(container_row("Ports", div().min_w_0().flex_1().mono().text_color(p.text2).child(ports), true, cx));
        }
        let mut card = div()
            .id(SharedString::from(format!("pod-container-{}", container.name)))
            .test_support()
            .bg(p.bg3)
            .rounded(px(8.))
            .p(px(16.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .mb(px(12.))
                    .child(
                        div()
                            .flex()
                            .min_w_0()
                            .items_center()
                            .gap(px(8.))
                            .child(Icon::new(IconName::Package).size_4().flex_none().text_color(p.muted))
                            .child(div().font_weight(FontWeight::MEDIUM).text_color(p.text1).child(container.name.clone())),
                    )
                    .child(status_badge(&container.state, None, cx)),
            )
            .child(info);
        if !container.env_vars.is_empty() {
            let open = self.open_env.contains(&container.name);
            let name = container.name.clone();
            let entity = cx.entity();
            let toggle = div()
                .id(SharedString::from(format!("pod-env-toggle-{name}")))
                .test_support()
                .flex()
                .items_center()
                .gap(px(4.))
                .t_sm()
                .text_color(p.muted)
                .cursor_pointer()
                .hover(|s| s.text_color(p.text2))
                .child(Icon::new(IconName::ChevronDown).size_4().when(!open, |i| i.rotate(radians(-std::f32::consts::FRAC_PI_2))))
                .child(format!("Environment Variables ({})", container.env_vars.len()))
                .on_click(move |_, _, cx| {
                    entity.update(cx, |this, cx| {
                        if !this.open_env.remove(&name) {
                            this.open_env.insert(name.clone());
                        }
                        cx.notify();
                    })
                });
            let env_rows = open.then(|| {
                div()
                    .id(SharedString::from(format!("pod-env-{}", container.name)))
                    .test_support()
                    .mt(px(8.))
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .t_sm()
                    .children(container.env_vars.iter().enumerate().map(|(i, env)| {
                        let value = displayed_env_value(env);
                        let copy_value = value.to_owned();
                        div()
                            .flex()
                            .min_w_0()
                            .items_center()
                            .gap(px(8.))
                            .py(px(4.))
                            .px(px(8.))
                            .bg(p.bg2)
                            .rounded(px(4.))
                            .child(div().mono().text_color(hex(if p.dark { tw::CYAN_400 } else { tw::CYAN_500 })).child(env.name.clone()))
                            .child(div().min_w_0().flex_1().overflow_hidden().mono().text_color(p.text2)
                                .child(selectable_text(format!("env-value-{}-{i}", container.name), value).max_lines(1)))
                            .child(div().flex_none().t_xs().text_color(p.muted).child(env.source.clone()))
                            .child(div().id(SharedString::from(format!("pod-env-copy-{}-{i}", container.name))).test_support()
                                .flex_none().cursor_pointer().text_color(p.muted)
                                .child(Icon::new(IconName::Copy).size_4())
                                .on_click(move |_, _, cx| cx.write_to_clipboard(ClipboardItem::new_string(copy_value.clone()))))
                    }))
            });
            card = card.child(div().mt(px(16.)).child(toggle).children(env_rows));
        }
        card.into_any_element()
    }

    fn sections(&self, details: &PodDetails, cx: &mut Context<Self>) -> AnyElement {
        let p = pal(cx);
        let overview = div()
            .child(info_row("Status", status_badge(&details.status, None, cx), false, cx))
            .child(info_row("Node", div().mono().t_sm().child(details.node.clone()), false, cx))
            .child(info_row(
                "IP",
                div().mono().t_sm().child(if details.ip.is_empty() { "-".into() } else { details.ip.clone() }),
                false,
                cx,
            ))
            .child(info_row("Started", div().t_sm().child(format_timestamp(details.start_time.as_deref())), true, cx));
        let mut entries: Vec<_> = details.labels.iter().collect();
        entries.sort_by_key(|(key, _)| *key);
        let labels = div()
            .flex()
            .flex_wrap()
            .gap(px(8.))
            .children(entries.iter().enumerate().map(|(i, (key, value))| {
                div()
                    .flex_none()
                    .px(px(8.))
                    .py(px(4.))
                    .rounded(px(4.))
                    .t_xs()
                    .mono()
                    .bg(p.bg3)
                    .text_color(p.text2)
                    .child(selectable_text(format!("pod-label-{i}"), &format!("{key}={value}")))
            }))
            .when(entries.is_empty(), |d| d.child(div().t_sm().text_color(p.muted).child("No labels")));
        let containers =
            div().flex().flex_col().gap(px(12.)).children(details.containers.iter().map(|container| self.container(container, cx)));
        let mut body = div()
            .id("pod-details")
            .test_support()
            .p(px(16.))
            .flex()
            .flex_col()
            .gap(px(24.))
            .child(section("OVERVIEW", overview, cx))
            .child(section("LABELS", labels, cx))
            .child(section(&format!("CONTAINERS ({})", details.containers.len()), containers, cx));
        if !details.conditions.is_empty() {
            let entity = cx.entity();
            let header = collapsible_header(
                "pod-conditions-toggle",
                &format!("Conditions ({})", details.conditions.len()),
                self.conditions_open,
                move |_, cx| {
                    entity.update(cx, |this, cx| {
                        this.conditions_open = !this.conditions_open;
                        cx.notify();
                    })
                },
                cx,
            )
            .into_any_element();
            let conditions = self.conditions_open.then(|| {
                div().id("pod-conditions").test_support().flex().flex_col().gap(px(8.)).children(details.conditions.iter().enumerate().map(
                    |(i, condition)| {
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .py(px(8.))
                            .when(i + 1 < details.conditions.len(), |d| d.border_b_1().border_color(p.border_subtle))
                            .child(div().t_sm().text_color(p.text2).child(condition.condition_type.clone()))
                            .child(
                                div()
                                    .t_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(if condition.status == "True" { hex(tw::EMERALD_500) } else { p.muted })
                                    .child(condition.status.clone()),
                            )
                    },
                ))
            });
            body = body.child(div().child(div().mb(px(12.)).child(header)).children(conditions));
        }
        body.into_any_element()
    }
}

fn pod_scope(store: &Store) -> Option<(String, String, String)> {
    match &store.drawer {
        Some(DrawerKind::Pod(name)) => Some((store.context.clone(), store.namespace.clone(), name.clone())),
        _ => None,
    }
}

fn section(title: &str, body: impl IntoElement, cx: &App) -> Div {
    div().child(div().mb(px(12.)).child(section_label(title.to_owned(), cx))).child(body)
}

fn info_row(label: &'static str, body: impl IntoElement, last: bool, cx: &App) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.))
        .py(px(8.))
        .when(!last, |d| d.border_b_1().border_color(pal(cx).border_subtle))
        .child(div().t_sm().text_color(pal(cx).text2).child(label))
        .child(body)
}

fn container_row(label: &'static str, body: impl IntoElement, centered: bool, cx: &App) -> Div {
    div()
        .flex()
        .gap(px(8.))
        .when(centered, |d| d.items_center())
        .when(!centered, |d| d.items_start())
        .child(div().w(px(80.)).flex_none().text_color(pal(cx).muted).child(label))
        .child(body)
}

impl Render for PodDetailsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let details = self.store.read(cx).pod_details.data().cloned();
        match details {
            Some(details) => self.sections(&details, cx),
            None => div().into_any_element(),
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{AppContext, TestAppContext};
    use serde_json::json;

    use crate::api::fake::Endpoint;
    use crate::app::store::{DrawerKind, View};
    use crate::app::testing::{boot, many, pod, settle_animation, with_pods};

    #[gpui_kit::test]
    fn secret_env_values_masked(cx: &mut TestAppContext) {
        let api = with_pods(vec![pod("api-123", "Running")]);
        many(
            &api,
            Endpoint::PodDetails,
            json!({
                "name": "api-123", "namespace": "default", "status": "Running", "node": "worker-1", "ip": "10.0.0.1",
                "start_time": "2026-11-11T00:00:00Z", "labels": {"app": "api"}, "annotations": {},
                "containers": [{"name": "app", "image": "example/api:v1", "ready": true, "restart_count": 2, "state": "Running",
                    "env_vars": [{"name": "TOKEN", "value": "sensitive-value-must-stay-hidden", "source": "Secret"},
                        {"name": "REGION", "value": "jakarta", "source": "Direct"}],
                    "ports": [{"name": "http", "container_port": 8080, "protocol": "TCP"}],
                    "resources": {"cpu_request": "100m", "cpu_limit": "1", "memory_request": "128Mi", "memory_limit": "512Mi"}}],
                "conditions": [{"condition_type": "Ready", "status": "True", "reason": null, "message": null, "last_transition_time": null}]
            }),
            5,
        );
        let env = boot(cx, api.clone(), None);
        env.store.update(cx, |s, cx| s.set_view(View::Pods, cx));
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            window.click("pod-info-api-123", cx);
        })
        .unwrap();
        cx.run_until_parked();
        env.store.read_with(cx, |s, _| {
            assert_eq!(s.view, View::Pods);
            assert_eq!(s.drawer, Some(DrawerKind::Pod("api-123".into())));
        });
        assert_eq!(api.calls(Endpoint::PodDetails), 1);
        settle_animation(cx);
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("pod-details").is_some());
            assert!(window.try_find("pod-env-app").is_none());
            assert!(window.try_find("pod-conditions").is_none());
            window.click("pod-conditions-toggle", cx);
            assert!(window.try_find("pod-conditions").is_some());
            window.click("pod-conditions-toggle", cx);
            window.click("pod-env-toggle-app", cx);
            assert!(window.try_find("pod-env-app").is_some());
        })
        .unwrap();
        // Rich text parsing runs asynchronously; allow it to finish before inspecting painted text.
        cx.run_until_parked();
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            let text = gpui_kit::base::test_support::texts(window).into_iter().map(|t| t.text().to_owned()).collect::<Vec<_>>().join("\n");
            assert!(text.contains("<secret>"));
            assert!(text.contains("jakarta"));
            assert!(!text.contains("sensitive-value-must-stay-hidden"));
            window.click("pod-env-copy-app-0", cx);
            window.press("escape", cx);
        })
        .unwrap();
        assert_eq!(cx.read_from_clipboard().unwrap().text().as_deref(), Some("<secret>"));
        assert_eq!(env.store.read_with(cx, |s, _| s.drawer.clone()), None);
    }

    #[test]
    fn masks_secret_source_before_rendering() {
        let mut env = crate::models::EnvVar { name: "TOKEN".into(), value: "private".into(), source: "Secret".into() };
        assert_eq!(super::displayed_env_value(&env), "<secret>");
        env.source = "ConfigMap".into();
        assert_eq!(super::displayed_env_value(&env), "private");
    }

}
