//! Pod overview, selectable values, container cards and collapsed disclosures.
use std::collections::{HashMap, HashSet};

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, tooltip::Tooltip};
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
    expanded_env: HashSet<String>,
    expanded_env_groups: HashSet<(String, String)>,
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
            reset_detail_state(
                &mut this.scope,
                scope,
                &mut this.open_env,
                &mut this.expanded_env,
                &mut this.expanded_env_groups,
                &mut this.conditions_open,
            );
            cx.notify();
        });
        Self {
            store,
            scope,
            open_env: HashSet::new(),
            expanded_env: HashSet::new(),
            expanded_env_groups: HashSet::new(),
            conditions_open: false,
            _sub: sub,
        }
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
        if !container.env_vars.is_empty() || !container.env_issues.is_empty() {
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
                let groups = group_env_vars(&container.env_vars);
                let container_name = container.name.clone();
                let entity = cx.entity();
                div()
                    .id(SharedString::from(format!("pod-env-{}", container_name)))
                    .test_support()
                    .mt(px(8.))
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .t_sm()
                    .children(groups.iter().map(|group| {
                        let source = group.source.to_owned();
                        let key = (container_name.clone(), source.clone());
                        let expanded = self.expanded_env_groups.contains(&key);
                        let total = group.rows.len();
                        let visible = if expanded { total } else { total.min(ENV_PREVIEW_LIMIT) };
                        let collapsed_hidden = total.saturating_sub(ENV_PREVIEW_LIMIT);
                        let header = div()
                            .id(SharedString::from(env_group_id(&container_name, &source)))
                            .test_support()
                            .mt(px(4.))
                            .t_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(format!("{} ({})", group.source, total));
                        let action = (total > ENV_PREVIEW_LIMIT).then(|| {
                            let entity = entity.clone();
                            let key = key.clone();
                            div()
                                .id(SharedString::from(env_group_action_id(&container_name, &source)))
                                .test_support()
                                .cursor_pointer()
                                .t_xs()
                                .text_color(hex(if p.dark { tw::CYAN_400 } else { tw::CYAN_500 }))
                                .child(if expanded { "Show less".to_owned() } else { format!("Show more ({collapsed_hidden} more)") })
                                .on_click(move |_, _, cx| {
                                    entity.update(cx, |this, cx| {
                                        if !this.expanded_env_groups.remove(&key) {
                                            this.expanded_env_groups.insert(key.clone());
                                        }
                                        cx.notify();
                                    })
                                })
                        });
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .child(header)
                            .children(group.rows[..visible].iter().map(|(i, env)| self.env_row(&container_name, *i, *env, cx)))
                            .children(action)
                    }))
                    .children(container.env_issues.iter().enumerate().map(|(i, issue)| {
                        let reason = match issue.reason {
                            crate::models::EnvResolutionReason::Forbidden => "access denied",
                            crate::models::EnvResolutionReason::NotFound => "not found",
                            crate::models::EnvResolutionReason::MissingKey => "key not found",
                            crate::models::EnvResolutionReason::Timeout => "lookup timed out",
                            crate::models::EnvResolutionReason::Other => "lookup failed",
                        };
                        let key = issue.key.as_deref().map(|k| format!(":{k}")).unwrap_or_default();
                        div().id(SharedString::from(format!("pod-env-issue-{}-{i}", container.name))).test_support().t_xs().text_color(p.muted).child(format!("{}{}: {}{}", issue.source, key, if issue.optional { "optional source unavailable · " } else { "" }, reason))
                    }))
                    .when(container.env_vars.iter().any(|env| env.source == "ConfigMap") || !container.env_issues.is_empty(), |d| {
                        d.child(div().id(SharedString::from(format!("pod-env-current-config-{}", container.name))).test_support().mt(px(4.)).t_xs().text_color(p.muted).child("ConfigMap values reflect the current configuration and may differ from a running container."))
                    })
            });
            card = card.child(div().mt(px(16.)).child(toggle).children(env_rows));
        }
        card.into_any_element()
    }

    fn env_row(&self, container_name: &str, i: usize, env: &EnvVar, cx: &mut Context<Self>) -> AnyElement {
        let p = pal(cx);
        let row_key = format!("{container_name}:{i}");
        let expanded = self.expanded_env.contains(&row_key);
        let value = displayed_env_value(env);
        let unavailable = env.resolution == crate::models::EnvResolution::Unavailable;
        let expandable = env.source == "ConfigMap" && env.resolution == crate::models::EnvResolution::ConfigMapResolved;
        let env_name = env.name.clone();
        let copy_value = value.to_owned();
        let copy = div()
            .id(SharedString::from(format!("pod-env-copy-{container_name}-{i}")))
            .test_support()
            .flex_none()
            .cursor_pointer()
            .text_color(p.muted)
            .child(Icon::new(IconName::Copy).size_4())
            .on_click(move |_, _, cx| cx.write_to_clipboard(ClipboardItem::new_string(copy_value.clone())));
        let detail = (expandable && expanded).then(|| div()
            .id(SharedString::from(format!("pod-env-detail-{container_name}-{i}")))
            .test_support()
            .min_w_0()
            .max_h(px(180.))
            .w_full()
            .overflow_x_scroll()
            .overflow_y_scroll()
            .mt(px(6.))
            .p(px(8.))
            .bg(p.bg3)
            .rounded(px(4.))
            .mono()
            .text_color(p.text2)
            .child(if value.is_empty() { div().child("(empty)").into_any_element() } else { selectable_text(format!("env-detail-{container_name}-{i}"), value).into_any_element() }));
        let toggle_detail = (expandable && !unavailable).then(|| {
            let key = row_key.clone();
            let entity = cx.entity();
            div().id(SharedString::from(format!("pod-env-expand-{container_name}-{i}"))).test_support()
                .cursor_pointer().text_color(hex(if p.dark { tw::CYAN_400 } else { tw::CYAN_500 }))
                .child(if expanded { "Hide full value" } else { "Show full value" })
                .on_click(move |_, _, cx| entity.update(cx, |this, cx| {
                    if !this.expanded_env.remove(&key) { this.expanded_env.insert(key.clone()); }
                    cx.notify();
                }))
        });
        div()
            .id(SharedString::from(format!("pod-env-row-{container_name}-{i}")))
            .test_support()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(4.))
            .py(px(4.))
            .px(px(8.))
            .bg(p.bg2)
            .rounded(px(4.))
            .child(div().flex().min_w_0().items_center().gap(px(8.))
            .child(div().id(SharedString::from(format!("pod-env-name-{container_name}-{i}"))).test_support().w(px(120.)).min_w_0().flex_none().truncate().tooltip(move |w, cx| Tooltip::new(env_name.clone()).build(w, cx)).mono().text_color(hex(if p.dark { tw::CYAN_400 } else { tw::CYAN_500 })).child(env.name.clone()))
            .child(
                div()
                    .id(SharedString::from(format!("pod-env-value-{container_name}-{i}")))
                    .test_support()
                    .min_w_0()
                    .flex_1()
                    .overflow_hidden()
                    .mono()
                    .text_color(if unavailable { p.muted } else { p.text2 })
                    .child(if unavailable { div().child("Unavailable").into_any_element() } else if value.is_empty() { div().child("(empty)").into_any_element() } else { selectable_text(format!("env-value-{container_name}-{i}"), value).max_lines(1).into_any_element() }),
            )
            .children(toggle_detail)
            .child(div().id(SharedString::from(format!("pod-env-source-{container_name}-{i}"))).test_support().flex_none().t_xs().text_color(p.muted).child(env.source.clone()))
            .children((!unavailable).then_some(copy)))
            .children(env.reference.as_ref().map(|reference| div().id(SharedString::from(format!("pod-env-reference-{container_name}-{i}"))).test_support().ml(px(4.)).t_xs().mono().text_color(p.muted).child(reference.clone())))
            .children(detail)
            .into_any_element()
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

/// Number of rows previewed per environment source group before Show more.
const ENV_PREVIEW_LIMIT: usize = 5;

/// Canonical category order; anything else is grouped by its original source label.
const KNOWN_ENV_SOURCES: [&str; 5] = ["Direct", "FieldRef", "ResourceRef", "ConfigMap", "Secret"];

struct EnvGroup<'a> {
    source: &'a str,
    rows: Vec<(usize, &'a EnvVar)>,
}

/// Group env vars by source in canonical order, preserving the original index of each row.
/// Unknown sources are appended after known categories, sorted by label.
fn group_env_vars<'a>(env_vars: &'a [EnvVar]) -> Vec<EnvGroup<'a>> {
    let mut by_source: HashMap<&'a str, Vec<(usize, &'a EnvVar)>> = HashMap::new();
    for (i, env) in env_vars.iter().enumerate() {
        by_source.entry(env.source.as_str()).or_default().push((i, env));
    }
    let mut groups = Vec::new();
    for known in KNOWN_ENV_SOURCES {
        if let Some(rows) = by_source.get(known) {
            groups.push(EnvGroup { source: known, rows: rows.clone() });
        }
    }
    let mut unknown: Vec<&'a str> = by_source.keys().copied().filter(|k| !KNOWN_ENV_SOURCES.contains(k)).collect();
    unknown.sort_unstable();
    for source in unknown {
        if let Some(rows) = by_source.get(source) {
            groups.push(EnvGroup { source, rows: rows.clone() });
        }
    }
    groups
}

/// Deterministic, collision-free element id for a group header from a container/source pair.
fn env_group_id(container: &str, source: &str) -> String {
    format!("pod-env-group-{}-{}-{}-{}", container.len(), container, source.len(), source)
}

/// Deterministic, collision-free element id for a group's Show more / Show less action.
fn env_group_action_id(container: &str, source: &str) -> String {
    format!("pod-env-group-toggle-{}-{}-{}-{}", container.len(), container, source.len(), source)
}

fn pod_scope(store: &Store) -> Option<(String, String, String)> {
    match &store.drawer {
        Some(DrawerKind::Pod(name)) => Some((store.context.clone(), store.namespace.clone(), name.clone())),
        _ => None,
    }
}

fn reset_detail_state(
    current_scope: &mut Option<(String, String, String)>,
    next_scope: Option<(String, String, String)>,
    open_env: &mut HashSet<String>,
    expanded_env: &mut HashSet<String>,
    expanded_env_groups: &mut HashSet<(String, String)>,
    conditions_open: &mut bool,
) {
    if *current_scope != next_scope {
        *current_scope = next_scope;
        open_env.clear();
        expanded_env.clear();
        expanded_env_groups.clear();
        *conditions_open = false;
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
    use gpui_kit::{AppContext, ScrollDelta, TestAppContext, point, px};
    use serde_json::{json, Value};
    use std::collections::HashSet;
    use std::sync::Arc;
    use std::time::Duration;

    use crate::api::fake::{Endpoint, FakeApi};
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
        let mut env = crate::models::EnvVar { name: "TOKEN".into(), value: "private".into(), source: "Secret".into(), reference: None, resolution: crate::models::EnvResolution::Declared };
        assert_eq!(super::displayed_env_value(&env), "<secret>");
        env.source = "ConfigMap".into();
        assert_eq!(super::displayed_env_value(&env), "private");
    }

    #[test]
    fn resets_environment_detail_state_when_pod_scope_changes() {
        let mut scope = Some(("ctx".into(), "ns".into(), "first".into()));
        let mut open = HashSet::from(["app".to_string()]);
        let mut expanded = HashSet::from(["app:0".to_string()]);
        let mut groups = HashSet::from([("app".to_string(), "Direct".to_string())]);
        let mut conditions_open = true;
        super::reset_detail_state(
            &mut scope,
            Some(("ctx".into(), "ns".into(), "second".into())),
            &mut open,
            &mut expanded,
            &mut groups,
            &mut conditions_open,
        );
        assert_eq!(scope.as_ref().unwrap().2, "second");
        assert!(open.is_empty());
        assert!(expanded.is_empty());
        assert!(groups.is_empty());
        assert!(!conditions_open);
    }

    #[test]
    fn preserves_environment_detail_state_when_scope_unchanged() {
        let mut scope = Some(("ctx".into(), "ns".into(), "same".into()));
        let mut open = HashSet::from(["app".to_string()]);
        let mut expanded = HashSet::from(["app:0".to_string()]);
        let mut groups = HashSet::from([("app".to_string(), "Direct".to_string())]);
        let mut conditions_open = true;
        super::reset_detail_state(
            &mut scope,
            Some(("ctx".into(), "ns".into(), "same".into())),
            &mut open,
            &mut expanded,
            &mut groups,
            &mut conditions_open,
        );
        assert!(open.contains("app"));
        assert!(expanded.contains("app:0"));
        assert!(groups.contains(&("app".to_string(), "Direct".to_string())));
        assert!(conditions_open);
    }

    #[test]
    fn group_env_vars_orders_known_then_unknown_and_preserves_indices() {
        let mk = |name: &str, source: &str| crate::models::EnvVar {
            name: name.into(),
            value: String::new(),
            source: source.into(),
            reference: None,
            resolution: crate::models::EnvResolution::Declared,
        };
        let vars = vec![
            mk("z", "CustomZ"),       // 0 unknown
            mk("c", "ConfigMap"),     // 1
            mk("s1", "Secret"),       // 2
            mk("d", "Direct"),        // 3
            mk("r", "ResourceRef"),   // 4
            mk("f", "FieldRef"),      // 5
            mk("s2", "Secret"),       // 6 (duplicate category)
            mk("a", "CustomA"),       // 7 unknown
            mk("d", "Direct"),        // 8 (duplicate name, same as index 3)
        ];
        let groups = super::group_env_vars(&vars);
        let sources: Vec<&str> = groups.iter().map(|g| g.source).collect();
        assert_eq!(sources, ["Direct", "FieldRef", "ResourceRef", "ConfigMap", "Secret", "CustomA", "CustomZ"]);
        let indices = |g: &super::EnvGroup<'_>| g.rows.iter().map(|(i, _)| *i).collect::<Vec<_>>();
        assert_eq!(indices(&groups[0]), vec![3, 8]);
        let secret = groups.iter().find(|g| g.source == "Secret").unwrap();
        assert_eq!(indices(secret), vec![2, 6]);
        let custom_a = groups.iter().find(|g| g.source == "CustomA").unwrap();
        assert_eq!(indices(custom_a), vec![7]);
        let custom_z = groups.iter().find(|g| g.source == "CustomZ").unwrap();
        assert_eq!(indices(custom_z), vec![0]);
        // Duplicate variable names stay distinct through their original indices.
        assert_eq!(groups[0].rows[0].1.name, "d");
        assert_eq!(groups[0].rows[1].1.name, "d");
    }

    #[gpui_kit::test]
    fn configmap_long_values_expand_and_copy_and_issue_only_container_is_visible(cx: &mut TestAppContext) {
        let long = format!("line one\n{}", "long-value-".repeat(24));
        let api = with_pods(vec![pod("api-123", "Running")]);
        many(&api, Endpoint::PodDetails, json!({
            "name":"api-123","namespace":"default","status":"Running","node":"worker-1","ip":"10.0.0.1",
            "start_time":null,"labels":{},"annotations":{},"conditions":[],"containers":[
                {"name":"app","image":"example/api","ready":true,"restart_count":0,"state":"Running",
                 "env_vars":[{"name":"CONFIG","value":long,"source":"ConfigMap","reference":"app-config:application.yaml","resolution":"config_map_resolved"},{"name":"EMPTY","value":"","source":"ConfigMap","reference":"app-config:empty","resolution":"config_map_resolved"},{"name":"MISSING","value":"error text must not be shown","source":"ConfigMap","reference":"app-config:missing","resolution":"unavailable"}],"env_issues":[],"ports":[],
                 "resources":{"cpu_request":null,"cpu_limit":null,"memory_request":null,"memory_limit":null}},
                {"name":"broken","image":"example/broken","ready":false,"restart_count":0,"state":"Running",
                 "env_vars":[],"env_issues":[{"source":"missing","key":"A","optional":false,"reason":"not_found"},{"source":"missing","key":"B","optional":false,"reason":"not_found"}],"ports":[],
                 "resources":{"cpu_request":null,"cpu_limit":null,"memory_request":null,"memory_limit":null}}
            ]
        }), 5);
        let env = boot(cx, api, None);
        env.store.update(cx, |s, cx| s.set_view(View::Pods, cx));
        cx.update_window(env.handle, |_, window, cx| { window.render_frame(cx); window.click("pod-info-api-123", cx); }).unwrap();
        cx.run_until_parked();
        settle_animation(cx);
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("pod-env-toggle-broken").is_some());
            window.click("pod-env-toggle-broken", cx);
            window.click("pod-env-toggle-app", cx);
            window.click("pod-env-expand-app-0", cx);
        }).unwrap();
        cx.run_until_parked();
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("pod-env-reference-app-0").is_some());
            assert!(window.try_find("pod-env-current-config-app").is_some());
            assert!(window.try_find("pod-env-detail-app-0").is_some());
            assert!(window.try_find("pod-env-value-app-1").is_some());
            assert!(window.try_find("pod-env-value-app-2").is_some());
            assert!(window.try_find("pod-env-copy-app-2").is_none());
            assert!(window.try_find("pod-env-issue-broken-0").is_some());
            assert!(window.try_find("pod-env-issue-broken-1").is_some());
            window.click("pod-env-copy-app-0", cx);
        }).unwrap();
        assert_eq!(cx.read_from_clipboard().unwrap().text().as_deref(), Some(long.as_str()));
        cx.update_window(env.handle, |_, window, cx| window.click("pod-env-copy-app-1", cx)).unwrap();
        assert!(cx.read_from_clipboard().is_some());
    }

    #[gpui_kit::test]
    fn short_configmap_value_with_long_name_stays_inside_drawer_and_expands(cx: &mut TestAppContext) {
        let value = "short-value";
        let long_name = "CONFIGURATION_VARIABLE_NAME_THAT_IS_LONG_ENOUGH_TO_CONSUME_THE_DRAWER_WIDTH_AND_MUST_NOT_PUSH_CONTROLS_OUTSIDE_ABCDEFGHIJKLMNOPQRSTUVWXYZ_0123456789";
        let api = with_pods(vec![pod("api-123", "Running")]);
        many(&api, Endpoint::PodDetails, json!({
            "name":"api-123","namespace":"default","status":"Running","node":"worker-1","ip":"10.0.0.1",
            "start_time":null,"labels":{},"annotations":{},"conditions":[],"containers":[
                {"name":"app","image":"example/api","ready":true,"restart_count":0,"state":"Running",
                 "env_vars":[{"name":long_name,"value":value,"source":"ConfigMap","reference":"app-config:key","resolution":"config_map_resolved"}],"env_issues":[],"ports":[],
                 "resources":{"cpu_request":null,"cpu_limit":null,"memory_request":null,"memory_limit":null}}
            ]
        }), 5);
        let env = boot(cx, api, None);
        env.store.update(cx, |s, cx| s.set_view(View::Pods, cx));
        cx.update_window(env.handle, |_, window, cx| { window.render_frame(cx); window.click("pod-info-api-123", cx); }).unwrap();
        cx.run_until_parked();
        settle_animation(cx);
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            window.click("pod-env-toggle-app", cx);
            window.render_frame(cx);
            assert!(window.try_find("pod-env-expand-app-0").is_some(), "short ConfigMap values still need full-value access");
            window.click("pod-env-expand-app-0", cx);
            window.render_frame(cx);
            assert!(window.try_find("pod-env-detail-app-0").is_some());
            let panel = window.find((gpui_kit::ElementId::from("drawer-pod"), "panel")).bounds();
            let panel_right = panel.origin.x + panel.size.width;
            let row = window.find("pod-env-row-app-0").bounds();
            let name = window.find("pod-env-name-app-0").bounds();
            assert!(name.size.width <= gpui_kit::px(120.));
            assert!(row.origin.x + row.size.width <= panel_right);
            for id in ["pod-env-source-app-0", "pod-env-expand-app-0", "pod-env-copy-app-0"] {
                let bounds = window.find(id).bounds();
                assert!(bounds.origin.x + bounds.size.width <= panel_right, "{id} must remain within the drawer");
            }
            window.click("pod-env-copy-app-0", cx);
        }).unwrap();
        assert_eq!(cx.read_from_clipboard().unwrap().text().as_deref(), Some(value));
    }

    #[gpui_kit::test]
    fn grouped_env_show_more_show_less_and_group_independence(cx: &mut TestAppContext) {
        let env_var = |name: &str, source: &str| json!({"name": name, "value": format!("v-{name}"), "source": source});
        let app_vars: Vec<_> = (0..6)
            .map(|n| env_var(&format!("DIR{n}"), "Direct"))
            .chain((0..6).map(|n| env_var(&format!("SEC{n}"), "Secret")))
            .collect();
        let api = with_pods(vec![pod("api-123", "Running")]);
        many(&api, Endpoint::PodDetails, json!({
            "name":"api-123","namespace":"default","status":"Running","node":"worker-1","ip":"10.0.0.1",
            "start_time":null,"labels":{},"annotations":{},"conditions":[],
            "containers": [{"name":"app","image":"example/img","ready":true,"restart_count":0,"state":"Running",
                "env_vars": app_vars, "env_issues": [], "ports": [],
                "resources":{"cpu_request":null,"cpu_limit":null,"memory_request":null,"memory_limit":null}}]
        }), 5);
        let env = boot(cx, api, None);
        env.store.update(cx, |s, cx| s.set_view(View::Pods, cx));
        cx.update_window(env.handle, |_, window, cx| { window.render_frame(cx); window.click("pod-info-api-123", cx); }).unwrap();
        cx.run_until_parked();
        settle_animation(cx);
        let direct_action = super::env_group_action_id("app", "Direct");
        let secret_action = super::env_group_action_id("app", "Secret");
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            window.click("pod-env-toggle-app", cx);
            window.render_frame(cx);
            assert!(window.try_find(super::env_group_id("app", "Direct")).is_some(), "group header renders");
            // Both six-row groups preview five; sixth row (original indices 5 and 11) hidden.
            assert!(window.try_find("pod-env-row-app-4").is_some());
            assert!(window.try_find("pod-env-row-app-5").is_none());
            assert!(window.try_find("pod-env-row-app-10").is_some());
            assert!(window.try_find("pod-env-row-app-11").is_none());
            assert!(window.try_find(direct_action.clone()).is_some());
            assert!(window.try_find(secret_action.clone()).is_some());
            window.click(direct_action.clone(), cx);
            window.render_frame(cx);
            // Show more reveals all six Direct rows at once, leaving Secret collapsed.
            assert!(window.try_find("pod-env-row-app-5").is_some());
            assert!(window.try_find("pod-env-row-app-11").is_none());
            // Show less collapses Direct back to five rows.
            window.click(direct_action.clone(), cx);
            window.render_frame(cx);
            assert!(window.try_find("pod-env-row-app-5").is_none());
        }).unwrap();
    }

    #[gpui_kit::test]
    fn grouped_env_expansion_independent_across_containers(cx: &mut TestAppContext) {
        let env_var = |name: &str| json!({"name": name, "value": format!("v-{name}"), "source": "Direct"});
        let app_vars: Vec<_> = (0..6).map(|n| env_var(&format!("A{n}"))).collect();
        let side_vars: Vec<_> = (0..6).map(|n| env_var(&format!("S{n}"))).collect();
        let api = with_pods(vec![pod("api-123", "Running")]);
        many(&api, Endpoint::PodDetails, json!({
            "name":"api-123","namespace":"default","status":"Running","node":"worker-1","ip":"10.0.0.1",
            "start_time":null,"labels":{},"annotations":{},"conditions":[],
            "containers": [
                {"name":"app","image":"example/img","ready":true,"restart_count":0,"state":"Running",
                 "env_vars": app_vars, "env_issues": [], "ports": [],
                 "resources":{"cpu_request":null,"cpu_limit":null,"memory_request":null,"memory_limit":null}},
                {"name":"side","image":"example/img","ready":true,"restart_count":0,"state":"Running",
                 "env_vars": side_vars, "env_issues": [], "ports": [],
                 "resources":{"cpu_request":null,"cpu_limit":null,"memory_request":null,"memory_limit":null}}
            ]
        }), 5);
        let env = boot(cx, api, None);
        env.store.update(cx, |s, cx| s.set_view(View::Pods, cx));
        cx.update_window(env.handle, |_, window, cx| { window.render_frame(cx); window.click("pod-info-api-123", cx); }).unwrap();
        cx.run_until_parked();
        settle_animation(cx);
        let app_action = super::env_group_action_id("app", "Direct");
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            window.click("pod-env-toggle-side", cx);
            window.render_frame(cx);
            window.click("pod-env-toggle-app", cx);
            window.render_frame(cx);
            assert!(window.try_find("pod-env-row-side-4").is_some());
            assert!(window.try_find("pod-env-row-side-5").is_none());
            window.click(app_action.clone(), cx);
            window.render_frame(cx);
            assert!(window.try_find("pod-env-row-app-5").is_some());
            // Expanding app's Direct group must not expand side's Direct group.
            assert!(window.try_find("pod-env-row-side-5").is_none());
        }).unwrap();
    }

    #[gpui_kit::test]
    fn grouped_rows_keep_copy_and_full_value_on_original_value(cx: &mut TestAppContext) {
        let long = format!("config\n{}", "cfg-value-".repeat(20));
        let api = with_pods(vec![pod("api-123", "Running")]);
        many(&api, Endpoint::PodDetails, json!({
            "name":"api-123","namespace":"default","status":"Running","node":"worker-1","ip":"10.0.0.1",
            "start_time":null,"labels":{},"annotations":{},"conditions":[],"containers":[
                {"name":"app","image":"example/api","ready":true,"restart_count":0,"state":"Running",
                 "env_vars":[
                    {"name":"TOKEN","value":"hidden-secret","source":"Secret"},
                    {"name":"DIRECT","value":"direct-value","source":"Direct"},
                    {"name":"CFG","value":long,"source":"ConfigMap","reference":"cfg:k","resolution":"config_map_resolved"}
                 ],"env_issues":[],"ports":[],
                 "resources":{"cpu_request":null,"cpu_limit":null,"memory_request":null,"memory_limit":null}}
            ]
        }), 5);
        let env = boot(cx, api, None);
        env.store.update(cx, |s, cx| s.set_view(View::Pods, cx));
        cx.update_window(env.handle, |_, window, cx| { window.render_frame(cx); window.click("pod-info-api-123", cx); }).unwrap();
        cx.run_until_parked();
        settle_animation(cx);
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            window.click("pod-env-toggle-app", cx);
            window.render_frame(cx);
            // Reordered by category: Direct (1), ConfigMap (2), Secret (0) all still render.
            assert!(window.try_find("pod-env-row-app-0").is_some());
            assert!(window.try_find("pod-env-row-app-1").is_some());
            assert!(window.try_find("pod-env-row-app-2").is_some());
            // Full-value access exists only for the ConfigMap row (original index 2).
            assert!(window.try_find("pod-env-expand-app-2").is_some());
            assert!(window.try_find("pod-env-expand-app-1").is_none());
            assert!(window.try_find("pod-env-expand-app-0").is_none());
            window.click("pod-env-copy-app-0", cx);
        }).unwrap();
        // Secret at original index 0 is still masked even though it renders last.
        assert_eq!(cx.read_from_clipboard().unwrap().text().as_deref(), Some("<secret>"));
        cx.update_window(env.handle, |_, window, cx| window.click("pod-env-copy-app-1", cx)).unwrap();
        assert_eq!(cx.read_from_clipboard().unwrap().text().as_deref(), Some("direct-value"));
        cx.update_window(env.handle, |_, window, cx| {
            window.click("pod-env-expand-app-2", cx);
            window.render_frame(cx);
            assert!(window.try_find("pod-env-detail-app-2").is_some());
        }).unwrap();
    }

    #[gpui_kit::test]
    fn five_row_group_shows_all_without_action_and_empty_categories_absent(cx: &mut TestAppContext) {
        let env_var = |name: &str| json!({"name": name, "value": format!("v-{name}"), "source": "Direct"});
        let api = with_pods(vec![pod("api-123", "Running")]);
        many(&api, Endpoint::PodDetails, json!({
            "name":"api-123","namespace":"default","status":"Running","node":"worker-1","ip":"10.0.0.1",
            "start_time":null,"labels":{},"annotations":{},"conditions":[],
            "containers":[
                {"name":"app","image":"example/img","ready":true,"restart_count":0,"state":"Running",
                 "env_vars": (0..5).map(|n| env_var(&format!("DIR{n}"))).collect::<Vec<_>>(),
                 "env_issues": [], "ports": [],
                 "resources":{"cpu_request":null,"cpu_limit":null,"memory_request":null,"memory_limit":null}},
                {"name":"broken","image":"example/broken","ready":false,"restart_count":0,"state":"Running",
                 "env_vars":[],"env_issues":[{"source":"missing","key":"A","optional":false,"reason":"not_found"}],"ports":[],
                 "resources":{"cpu_request":null,"cpu_limit":null,"memory_request":null,"memory_limit":null}},
                {"name":"bare","image":"example/bare","ready":true,"restart_count":0,"state":"Running",
                 "env_vars":[],"env_issues":[],"ports":[],
                 "resources":{"cpu_request":null,"cpu_limit":null,"memory_request":null,"memory_limit":null}}
            ]
        }), 5);
        let env = boot(cx, api, None);
        env.store.update(cx, |s, cx| s.set_view(View::Pods, cx));
        cx.update_window(env.handle, |_, window, cx| { window.render_frame(cx); window.click("pod-info-api-123", cx); }).unwrap();
        cx.run_until_parked();
        settle_animation(cx);
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            // A container with no variables and no issues has no disclosure at all.
            assert!(window.try_find("pod-env-toggle-bare").is_none());
            // An issue-only container keeps its disclosure and issues but renders no group.
            window.click("pod-env-toggle-broken", cx);
            window.render_frame(cx);
            assert!(window.try_find("pod-env-issue-broken-0").is_some());
            assert!(window.try_find(super::env_group_id("broken", "Direct")).is_none());
            // Exactly five rows: all five visible, no Show more action, and empty categories absent.
            window.click("pod-env-toggle-app", cx);
            window.render_frame(cx);
            assert!(window.try_find(super::env_group_id("app", "Direct")).is_some());
            for i in 0..5 {
                assert!(window.try_find(format!("pod-env-row-app-{i}")).is_some(), "row {i} must be visible");
            }
            assert!(window.try_find("pod-env-row-app-5").is_none());
            assert!(window.try_find(super::env_group_action_id("app", "Direct")).is_none(), "five rows must not offer Show more");
            for source in ["FieldRef", "ResourceRef", "ConfigMap", "Secret"] {
                assert!(window.try_find(super::env_group_id("app", source)).is_none(), "empty {source} category must be absent");
            }
        }).unwrap();
    }

    #[gpui_kit::test]
    fn group_expansion_preserved_on_refresh_and_reset_on_scope_change(cx: &mut TestAppContext) {
        let direct_vars = |prefix: &str| -> Vec<Value> {
            (0..6).map(|n| json!({"name": format!("{prefix}{n}"), "value": format!("v-{prefix}{n}"), "source": "Direct"})).collect()
        };
        let pod_details = |pod_name: &str, vars: Vec<Value>| json!({
            "name": pod_name, "namespace": "default", "status": "Running", "node": "worker-1", "ip": "10.0.0.1",
            "start_time": null, "labels": {}, "annotations": {}, "conditions": [],
            "containers": [{"name": "app", "image": "example/img", "ready": true, "restart_count": 0, "state": "Running",
                "env_vars": vars, "env_issues": [], "ports": [],
                "resources": {"cpu_request": null, "cpu_limit": null, "memory_request": null, "memory_limit": null}}]
        });
        let api = Arc::new(FakeApi::default());
        many(&api, Endpoint::Clusters, json!([{"name":"c1","server":"s","is_current":true},{"name":"c2","server":"s","is_current":false}]), 5);
        many(&api, Endpoint::Namespaces, json!([{"name":"default","status":"Active"},{"name":"prod","status":"Active"}]), 50);
        many(&api, Endpoint::Deployments, json!([]), 50);
        many(&api, Endpoint::Pods, json!([pod("api-123", "Running"), pod("web-456", "Running")]), 100);
        // Both pods share the container name "app" so a leaked `(container, source)` key
        // would survive a scope change and make a missing reset observable.
        many(&api, Endpoint::PodDetails, pod_details("api-123", direct_vars("A")), 2);
        many(&api, Endpoint::PodDetails, pod_details("web-456", direct_vars("W")), 3);

        let env = boot(cx, api, None);
        env.store.update(cx, |s, cx| s.set_view(View::Pods, cx));
        cx.update_window(env.handle, |_, window, cx| { window.render_frame(cx); window.click("pod-info-api-123", cx); }).unwrap();
        cx.run_until_parked();
        settle_animation(cx);
        let action = super::env_group_action_id("app", "Direct");
        let details_calls = env.api.calls(Endpoint::PodDetails);

        // Expand the six-row Direct group.
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            window.click("pod-env-toggle-app", cx);
            window.render_frame(cx);
            assert!(window.try_find("pod-env-row-app-5").is_none());
            window.click(action.clone(), cx);
            window.render_frame(cx);
            assert!(window.try_find("pod-env-row-app-5").is_some());
        }).unwrap();

        // A same-scope refresh/notify (the periodic tick) must preserve the expansion and
        // must only notify, not refetch PodDetails (polling is disabled for pod details).
        cx.executor().advance_clock(Duration::from_secs(1));
        cx.run_until_parked();
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("pod-env-row-app-5").is_some(), "same-scope refresh must keep the group expanded");
        }).unwrap();
        assert_eq!(env.api.calls(Endpoint::PodDetails), details_calls, "the periodic tick notifies without refetching PodDetails");

        // Closing and reopening the environment disclosure keeps the group expanded.
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            window.click("pod-env-toggle-app", cx);
            window.render_frame(cx);
            assert!(window.try_find("pod-env-row-app-5").is_none());
            window.click("pod-env-toggle-app", cx);
            window.render_frame(cx);
            assert!(window.try_find("pod-env-row-app-5").is_some(), "reopening the disclosure must preserve expansion");
        }).unwrap();

        // Closing and reopening the drawer resets the group to five rows.
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            window.press("escape", cx);
        }).unwrap();
        cx.run_until_parked();
        settle_animation(cx);
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            window.click("pod-info-api-123", cx);
        }).unwrap();
        cx.run_until_parked();
        settle_animation(cx);
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            window.click("pod-env-toggle-app", cx);
            window.render_frame(cx);
            assert!(window.try_find("pod-env-row-app-5").is_none(), "reopening the drawer must reset the group to five rows");
            assert!(window.try_find(action.clone()).is_some());
        }).unwrap();

        // Switching to another pod (same container/source key) resets expansion.
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            window.click(action.clone(), cx);
            window.render_frame(cx);
            assert!(window.try_find("pod-env-row-app-5").is_some());
        }).unwrap();
        env.store.update(cx, |s, cx| s.select_pod(Some("web-456".into()), cx));
        cx.run_until_parked();
        settle_animation(cx);
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            window.click("pod-env-toggle-app", cx);
            window.render_frame(cx);
            assert!(window.try_find("pod-env-row-app-5").is_none(), "switching pods must reset expansion");
        }).unwrap();

        // Switching namespace closes the drawer and, after returning, resets expansion.
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            window.click(action.clone(), cx);
            window.render_frame(cx);
            assert!(window.try_find("pod-env-row-app-5").is_some());
        }).unwrap();
        env.store.update(cx, |s, cx| s.set_namespace("prod".into(), cx));
        env.store.read_with(cx, |s, _| {
            assert_eq!(s.drawer, None);
            assert_eq!(s.view, View::Deployments);
        });
        cx.run_until_parked();
        settle_animation(cx);
        env.store.update(cx, |s, cx| s.set_namespace("default".into(), cx));
        env.store.update(cx, |s, cx| s.set_view(View::Pods, cx));
        env.store.update(cx, |s, cx| s.select_pod(Some("web-456".into()), cx));
        cx.run_until_parked();
        settle_animation(cx);
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            window.click("pod-env-toggle-app", cx);
            window.render_frame(cx);
            assert!(window.try_find("pod-env-row-app-5").is_none(), "switching namespace must reset expansion");
        }).unwrap();

        // Switching context likewise closes the drawer and resets expansion after returning.
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            window.click(action.clone(), cx);
            window.render_frame(cx);
            assert!(window.try_find("pod-env-row-app-5").is_some());
        }).unwrap();
        env.store.update(cx, |s, cx| s.set_context("c2".into(), cx));
        env.store.read_with(cx, |s, _| {
            assert_eq!(s.drawer, None);
            assert_eq!(s.view, View::Deployments);
        });
        cx.run_until_parked();
        settle_animation(cx);
        env.store.update(cx, |s, cx| s.set_context("c1".into(), cx));
        env.store.update(cx, |s, cx| s.set_view(View::Pods, cx));
        env.store.update(cx, |s, cx| s.select_pod(Some("web-456".into()), cx));
        cx.run_until_parked();
        settle_animation(cx);
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            window.click("pod-env-toggle-app", cx);
            window.render_frame(cx);
            assert!(window.try_find("pod-env-row-app-5").is_none(), "switching context must reset expansion");
        }).unwrap();
    }

    #[gpui_kit::test]
    fn configmap_group_show_less_keeps_sixth_row_detail_and_copy(cx: &mut TestAppContext) {
        let long = format!("line one\n{}", "cfg-value-".repeat(20));
        let cfg_var = |n: usize| {
            let value = if n == 5 { long.clone() } else { format!("short-{n}") };
            json!({"name": format!("CFG{n}"), "value": value, "source": "ConfigMap", "resolution": "config_map_resolved"})
        };
        let app_vars: Vec<_> = (0..6).map(cfg_var).collect();
        let api = with_pods(vec![pod("api-123", "Running")]);
        many(&api, Endpoint::PodDetails, json!({
            "name":"api-123","namespace":"default","status":"Running","node":"worker-1","ip":"10.0.0.1",
            "start_time":null,"labels":{},"annotations":{},"conditions":[],
            "containers": [{"name":"app","image":"example/img","ready":true,"restart_count":0,"state":"Running",
                "env_vars": app_vars, "env_issues": [], "ports": [],
                "resources":{"cpu_request":null,"cpu_limit":null,"memory_request":null,"memory_limit":null}}]
        }), 5);
        let env = boot(cx, api, None);
        env.store.update(cx, |s, cx| s.set_view(View::Pods, cx));
        cx.update_window(env.handle, |_, window, cx| { window.render_frame(cx); window.click("pod-info-api-123", cx); }).unwrap();
        cx.run_until_parked();
        settle_animation(cx);
        let action = super::env_group_action_id("app", "ConfigMap");
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            window.click("pod-env-toggle-app", cx);
            window.render_frame(cx);
            // Collapsed: five of six rows visible; the sixth (original index 5) is hidden.
            assert!(window.try_find("pod-env-row-app-4").is_some());
            assert!(window.try_find("pod-env-row-app-5").is_none());
            assert!(window.try_find(super::env_group_id("app", "ConfigMap")).is_some());
            // The six-row group and its action sit near the drawer fold; scroll the drawer
            // body (the wheel event bubbles to its scroll container) before interacting.
            window.scroll("pod-details", ScrollDelta::Pixels(point(px(0.), px(-1000.))), cx);
            window.render_frame(cx);
            // Show more reveals the sixth row, then open its full-value detail.
            window.click(action.clone(), cx);
            window.render_frame(cx);
            assert!(window.try_find("pod-env-row-app-5").is_some());
            window.scroll("pod-details", ScrollDelta::Pixels(point(px(0.), px(-1000.))), cx);
            window.render_frame(cx);
            window.click("pod-env-expand-app-5", cx);
            window.render_frame(cx);
            assert!(window.try_find("pod-env-detail-app-5").is_some());
        }).unwrap();
        // Show less hides the sixth row; Show more restores it with its detail still open.
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            window.scroll("pod-details", ScrollDelta::Pixels(point(px(0.), px(-1000.))), cx);
            window.render_frame(cx);
            window.click(action.clone(), cx);
            window.render_frame(cx);
            assert!(window.try_find("pod-env-row-app-5").is_none(), "Show less must hide the sixth row");
            window.scroll("pod-details", ScrollDelta::Pixels(point(px(0.), px(-1000.))), cx);
            window.render_frame(cx);
            window.click(action.clone(), cx);
            window.render_frame(cx);
            assert!(window.try_find("pod-env-row-app-5").is_some(), "Show more must restore the sixth row");
            assert!(window.try_find("pod-env-detail-app-5").is_some(), "the sixth row's full value stays open across Show less/more");
            window.scroll("pod-details", ScrollDelta::Pixels(point(px(0.), px(-1000.))), cx);
            window.render_frame(cx);
            window.click("pod-env-copy-app-5", cx);
        }).unwrap();
        assert_eq!(cx.read_from_clipboard().unwrap().text().as_deref(), Some(long.as_str()));
    }
}
