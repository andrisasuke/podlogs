//! Log search screen and export formatting.
use crate::models::{LogEntry, LogSearchResponse, LogSearchResult};

pub fn entry_at(d: &LogSearchResponse, mut ix: usize) -> Option<(&LogSearchResult, &LogEntry)> {
    for result in &d.results {
        if ix < result.entries.len() {
            return Some((result, &result.entries[ix]));
        }
        ix -= result.entries.len();
    }
    None
}

pub fn row_count(d: &LogSearchResponse) -> usize {
    d.results.iter().map(|r| r.entries.len()).sum()
}

pub fn export_csv(results: &[LogSearchResult]) -> String {
    results
        .iter()
        .flat_map(|r| {
            r.entries.iter().map(move |e| {
                format!(
                    "{}\t{}\t{}\t{}",
                    e.timestamp.as_deref().unwrap_or(""),
                    r.pod_name,
                    e.level.as_deref().filter(|s| !s.is_empty()).unwrap_or("-"),
                    e.message
                )
            })
        })
        .collect::<Vec<_>>()
        .join("\n")
}

use super::log_detail_modal::{LogDetailModal, message_color};
use super::search_state::{Scope, SearchState, SearchStatus, Snapshot, view_model};
use super::store::Store;
use crate::format::{TimeRange, format_short_timestamp, highlight_match};
use crate::ui::badge::{dot, level_badge};
use crate::ui::button::{BtnSize, Button, Variant};
use crate::ui::dropdown::{DropdownEvent, DropdownState};
use crate::ui::input::text_input;
use crate::ui::mark_text::mark_text;
use crate::ui::section_label;
use crate::ui::theme::{MONO_FONT, Tw, hex, hexa, pal, tw};
use gpui_kit::assets::IconName;
use gpui_kit::component::{
    Icon,
    input::{InputEvent, InputState},
    tooltip::Tooltip,
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

pub struct LogSearchView {
    store: Entity<Store>,
    pub(crate) keyword: Entity<InputState>,
    deployment: Entity<DropdownState>,
    level: Entity<DropdownState>,
    time: Entity<DropdownState>,
    detail: Entity<LogDetailModal>,
    scope: Scope,
    state: SearchState,
    level_value: String,
    time_value: TimeRange,
    failures_open: bool,
    pod_width: f32,
    drag: Option<(f32, f32)>,
    list: ListState,
    _subs: Vec<Subscription>,
}
impl LogSearchView {
    pub fn new(store: Entity<Store>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let keyword = cx.new(|cx| InputState::new(window, cx).placeholder("Search keyword..."));
        let deployment = cx.new(|cx| {
            DropdownState::new(
                "Select deployment...",
                Some("Search deployments..."),
                window,
                cx,
            )
        });
        let level = cx.new(|cx| DropdownState::new("Any", None, window, cx));
        level.update(cx, |d, cx| {
            d.set_options(
                [
                    ("", "Any"),
                    ("ERROR", "ERROR"),
                    ("WARN", "WARN"),
                    ("INFO", "INFO"),
                    ("DEBUG", "DEBUG"),
                ]
                .into_iter()
                .map(|(a, b)| (a.into(), b.into()))
                .collect(),
                cx,
            );
            d.set_value(Some(String::new()), cx);
        });
        let time = cx.new(|cx| DropdownState::new("", None, window, cx));
        time.update(cx, |d, cx| {
            d.set_options(
                TimeRange::ALL
                    .into_iter()
                    .map(|t| (t.key().into(), t.label().into()))
                    .collect(),
                cx,
            );
            d.set_value(Some(TimeRange::H1.key().into()), cx);
        });
        let detail = cx.new(|_| LogDetailModal::new());
        let subs = vec![
            cx.observe_in(&store, window, |this, _, _, cx| this.sync(cx)),
            cx.subscribe(&deployment, |this, _, ev: &DropdownEvent, cx| {
                let DropdownEvent::Change(v) = ev;
                this.store
                    .update(cx, |s, cx| s.set_deployment(Some(v.clone()), cx));
            }),
            cx.subscribe(&level, |this, _, ev: &DropdownEvent, cx| {
                let DropdownEvent::Change(v) = ev;
                this.level_value = v.clone();
                cx.notify();
            }),
            cx.subscribe(&time, |this, _, ev: &DropdownEvent, cx| {
                let DropdownEvent::Change(v) = ev;
                if let Some(t) = TimeRange::ALL.into_iter().find(|t| t.key() == v) {
                    this.time_value = t;
                }
                cx.notify();
            }),
            cx.subscribe(&keyword, |this, _, ev: &InputEvent, cx| {
                if matches!(ev, InputEvent::PressEnter { .. }) {
                    this.submit(None, cx);
                } else if matches!(ev, InputEvent::Change) {
                    cx.notify();
                }
            }),
        ];
        let mut this = Self {
            store,
            keyword,
            deployment,
            level,
            time,
            detail,
            scope: Scope {
                context: String::new(),
                namespace: String::new(),
                deployment: None,
            },
            state: SearchState::default(),
            level_value: String::new(),
            time_value: TimeRange::H1,
            failures_open: false,
            pod_width: 208.,
            drag: None,
            list: ListState::new(0, ListAlignment::Top, px(400.)),
            _subs: subs,
        };
        this.sync(cx);
        this
    }
    fn sync(&mut self, cx: &mut Context<Self>) {
        let s = self.store.read(cx);
        let scope = Scope {
            context: s.context.clone(),
            namespace: s.namespace.clone(),
            deployment: s.deployment.clone(),
        };
        let options = s
            .deployments
            .data()
            .map(|d| d.iter().map(|d| (d.name.clone(), d.name.clone())).collect())
            .unwrap_or_default();
        if self.scope != scope {
            self.scope = scope.clone();
            self.state.reset_scope();
            self.detail.update(cx, |d, cx| d.close(cx));
        }
        self.deployment.update(cx, |d, cx| {
            d.set_options(options, cx);
            d.set_value(scope.deployment, cx);
        });
        cx.notify();
    }
    fn form(&self, cx: &App) -> Snapshot {
        Snapshot {
            context: self.scope.context.clone(),
            namespace: self.scope.namespace.clone(),
            deployment: self.scope.deployment.clone().unwrap_or_default(),
            keyword: self.keyword.read(cx).value().to_string(),
            log_level: self.level_value.clone(),
            time_range: self.time_value,
        }
    }
    fn submit(&mut self, snap: Option<Snapshot>, cx: &mut Context<Self>) {
        let snap = snap.unwrap_or_else(|| self.form(cx));
        let Some(id) = self.state.start(snap.clone(), &self.scope) else {
            return;
        };
        self.detail.update(cx, |d, cx| d.close(cx));
        let api = self.store.read(cx).api();
        let fut = api.search_logs(
            snap.context.clone(),
            snap.namespace.clone(),
            snap.deployment.clone(),
            (!snap.keyword.is_empty()).then_some(snap.keyword.clone()),
            (!snap.log_level.is_empty()).then_some(snap.log_level.clone()),
            Some(snap.time_range.seconds()),
        );
        cx.spawn(async move |this, cx| {
            let result = fut.await;
            let _ = this.update(cx, |this, cx| {
                this.state.finish(id, result, &this.scope);
                let n = match this.state.visible(&this.scope) {
                    SearchStatus::Success(_, d) => row_count(d),
                    _ => 0,
                };
                this.list.reset(n);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
    fn render_row(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let status = self.state.visible(&self.scope).clone();
        let SearchStatus::Success(snap, data) = status else {
            return div().into_any_element();
        };
        let Some((result, entry)) = entry_at(&data, index) else {
            return div().into_any_element();
        };
        let p = pal(cx);
        let mark_h = {
            let ts = window.text_system();
            let id = ts.resolve_font(&font(MONO_FONT));
            ts.ascent(id, px(14.)) + ts.descent(id, px(14.))
        };
        let mut entry = entry.clone();
        entry.pod_name = result.pod_name.clone();
        entry.container_name = result.container_name.clone();
        let pod = result.pod_name.clone();
        let container = result.container_name.clone();
        let tip = format!("{} / {}", pod, container);
        let tip2 = container.clone();
        let color = message_color(entry.level.as_deref(), p.dark, p.text2);
        let is_error = entry.level.as_deref() == Some("ERROR");
        let split = message_segments(&entry.message, &snap.keyword);
        let msg = if is_error {
            let range = split
                .as_ref()
                .map(|(before, matched, _)| before.len()..before.len() + matched.len());
            div()
                .min_w_0()
                .mono()
                .t_sm()
                .text_color(color)
                .child(mark_text(
                    entry.message.clone().into(),
                    range,
                    color,
                    p.text1,
                    hexa(tw::AMBER_500, 0.3),
                ))
        } else if let Some((before, matched, after)) = split {
            div()
                .flex()
                .items_center()
                .min_w_0()
                .overflow_hidden()
                .mono()
                .t_sm()
                .text_color(color)
                .child(div().flex_none().child(before))
                .child(
                    div()
                        .flex_none()
                        .px(px(2.))
                        // Tailwind `rounded` is 4 px; an inline mark's background spans
                        // only the font's content area, not the 20 px line box.
                        .rounded(px(4.))
                        .line_height(mark_h)
                        .bg(hexa(tw::AMBER_500, 0.3))
                        .text_color(p.text1)
                        .child(matched),
                )
                .child(div().flex_1().min_w_0().truncate().child(after))
        } else {
            div()
                .min_w_0()
                .mono()
                .t_sm()
                .text_color(color)
                .truncate()
                .child(entry.message.clone())
        };
        div()
            .id(("search-row", index))
            .test_support()
            .w_full()
            .min_w_0()
            .flex()
            .items_center()
            .when(is_error, |d| d.items_start().bg(hexa(tw::RED_500, 0.05)))
            .when(entry.level.as_deref() == Some("WARN"), |d| {
                d.bg(hexa(tw::AMBER_500, 0.05))
            })
            .border_b_1()
            .border_color(p.border)
            .hover(|s| s.bg(p.bg3))
            .cursor_pointer()
            .child(
                div()
                    .w(px(96.))
                    .flex_none()
                    .px(px(16.))
                    .py(px(8.))
                    .mono()
                    .t_sm()
                    .text_color(p.muted)
                    .child(format_short_timestamp(entry.timestamp.as_deref())),
            )
            .child(
                div()
                    .id(("search-pod-cell", index))
                    .test_support()
                    .w(px(self.pod_width))
                    .flex_none()
                    .px(px(16.))
                    .py(px(8.))
                    .min_w_0()
                    .child(
                        div()
                            .id(("search-pod", index))
                            .truncate()
                            .mono()
                            .t_sm()
                            .text_color(hex(if p.dark { tw::CYAN_400 } else { tw::CYAN_500 }))
                            .tooltip(move |w, cx| Tooltip::new(tip.clone()).build(w, cx))
                            .child(pod),
                    )
                    .child(
                        div()
                            .id(("search-container", index))
                            .truncate()
                            .t_xs()
                            .text_color(p.muted)
                            .tooltip(move |w, cx| Tooltip::new(tip2.clone()).build(w, cx))
                            .child(container),
                    ),
            )
            .child(
                div()
                    .w(px(80.))
                    .flex_none()
                    .px(px(16.))
                    .when_some(entry.level.as_deref(), |d, l| d.child(level_badge(l, cx))),
            )
            .child(
                div()
                    .id(("search-message-cell", index))
                    .test_support()
                    .flex_1()
                    .min_w_0()
                    .px(px(16.))
                    .child(msg),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.detail.update(cx, |d, cx| d.open(entry.clone(), cx))
            }))
            .into_any_element()
    }
    fn export(&mut self, cx: &mut Context<Self>) {
        let SearchStatus::Success(snap, data) = self.state.visible(&self.scope) else {
            return;
        };
        let content = export_csv(&data.results);
        let name = format!("{}-log-search.csv", snap.deployment);
        let dir = dirs::home_dir().unwrap_or_else(std::env::temp_dir);
        let dialog = cx.prompt_for_new_path(&dir, Some(&name));
        cx.spawn(async move |_, cx| {
            if let Ok(Ok(Some(path))) = dialog.await {
                let _ = cx
                    .background_executor()
                    .spawn(async move { std::fs::write(path, content) })
                    .await;
            }
        })
        .detach();
    }
}

/// Split only the first case-insensitive match, retaining original casing.
fn message_segments(text: &str, keyword: &str) -> Option<(String, String, String)> {
    highlight_match(text, keyword)
}

impl Render for LogSearchView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = pal(cx);
        let status = self.state.visible(&self.scope).clone();
        let vm = view_model(&status, &self.form(cx));
        let pending = matches!(status, SearchStatus::Pending(_));
        let disabled = self.scope.context.is_empty()
            || self.scope.namespace.is_empty()
            || self.scope.deployment.as_ref().is_none_or(String::is_empty)
            || pending;
        let field = |name: &str, child: AnyElement| {
            div()
                .flex_1()
                .min_w_0()
                .child(div().mb(px(8.)).child(section_label(name, cx)))
                .child(child)
        };
        let form = div()
            .px(px(24.))
            .py(px(16.))
            .border_b_1()
            .border_color(p.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .mb(px(16.))
                    .child(
                        div()
                            .t_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Log Search"),
                    )
                    .child(
                        Button::new("search-settings", Variant::Ghost)
                            .size(BtnSize::Icon)
                            .icon(IconName::Settings)
                            .tooltip("Settings")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.store.update(cx, |s, cx| s.open_settings(cx))
                            })),
                    ),
            )
            .child(
                div()
                    .id("search-controls-row")
                    .test_support()
                    .flex()
                    .items_end()
                    .gap(px(16.))
                    .child(field(
                        "Deployment",
                        div()
                            .w_full()
                            .min_w_0()
                            .id("search-deployment")
                            .child(self.deployment.clone())
                            .into_any_element(),
                    ))
                    .child(
                        field("Keyword", text_input(&self.keyword, cx).into_any_element())
                            .id("search-keyword-control")
                            .test_support(),
                    )
                    .child(
                        field("Log Level", self.level.clone().into_any_element())
                            .w(px(100.))
                            .flex_none(),
                    )
                    .child(
                        field("Time Range", self.time.clone().into_any_element())
                            .id("search-time-control")
                            .test_support()
                            .w(px(144.))
                            .flex_none(),
                    )
                    .child(
                        Button::new("search-submit", Variant::Primary)
                            .icon(IconName::Search)
                            .icon_gap(8.)
                            .label("Search")
                            .disabled(disabled)
                            .on_click(cx.listener(|this, _, _, cx| this.submit(None, cx))),
                    ),
            )
            .when(vm.filters_changed, |form| {
                form.child(
                    div()
                        .id("search-filter-warning")
                        .test_support()
                        .mt(px(8.))
                        .t_sm()
                        .text_color(hex(if p.dark { tw::AMBER_400 } else { tw::AMBER_600 }))
                        .child("Filters changed. Click Search to apply."),
                )
            });
        let mut content = div().flex_1().min_h_0().flex().flex_col();
        if let Some(line) = vm.applied_line {
            content = content.child(
                div()
                    .px(px(24.))
                    .pt(px(16.))
                    .t_xs()
                    .text_color(p.muted)
                    .child(line),
            );
        }
        let data = if let SearchStatus::Success(_, d) = &status {
            Some(d)
        } else {
            None
        };
        if vm.partial {
            let d = data.unwrap();
            content = content.child(
                div()
                    .mx(px(24.))
                    .mt(px(16.))
                    .p(px(12.))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(hexa(tw::AMBER_500, 0.4))
                    .text_color(hex(if p.dark { tw::AMBER_400 } else { tw::AMBER_600 }))
                    .t_sm()
                    .child(format!(
                        "Search incomplete: {} of {} containers searched successfully.",
                        d.successful_containers, d.total_containers
                    ))
                    .child(self.failure_list(d, cx))
                    .child(
                        Button::new("search-retry-partial", Variant::Secondary)
                            .size(BtnSize::Sm)
                            .label("Retry")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.submit(this.state.retry_snapshot(), cx)
                            })),
                    ),
            );
        }
        match &status {
            SearchStatus::Idle => {
                content = content.child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .text_color(p.muted)
                        .child(
                            div()
                                .id("search-initial-icon")
                                .test_support()
                                .mb(px(16.))
                                .opacity(0.3)
                                .child(Icon::new(IconName::Search).size(px(64.))),
                        )
                        .child(div().t_lg().child("Search across all pods in a deployment"))
                        .child(
                            div()
                                .t_sm()
                                .mt(px(8.))
                                .child("Select a deployment and click Search"),
                        ),
                )
            }
            SearchStatus::Pending(_) => {
                content = content.child(crate::ui::skeleton::log_skeleton(15, cx))
            }
            SearchStatus::Error(_, error) => {
                content = content.child(self.error_view(error, None, cx))
            }
            SearchStatus::Success(_, d) if vm.all_failed => {
                content = content.child(self.error_view(
                    "Could not read logs from any container.",
                    Some(d),
                    cx,
                ))
            }
            SearchStatus::Success(_, d) if d.total_containers == 0 => {
                content = content.child(
                    div()
                        .h(px(256.))
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .text_color(p.muted)
                        .child(div().t_lg().child("No pods or containers found"))
                        .child(
                            div()
                                .t_sm()
                                .child("This deployment has no regular containers to search."),
                        ),
                )
            }
            SearchStatus::Success(_, d) if d.results.is_empty() => {
                content = content.child(
                    div()
                        .h(px(256.))
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .text_color(p.muted)
                        .child(
                            div()
                                .id("search-empty-icon")
                                .test_support()
                                .mb(px(16.))
                                .opacity(0.5)
                                .child(Icon::new(IconName::Search).size(px(48.))),
                        )
                        .child(div().t_lg().child(if vm.partial {
                            "No matches in successfully searched containers"
                        } else {
                            "No results found"
                        }))
                        .child(div().t_sm().child(if vm.partial {
                            "The search is incomplete because some containers could not be read."
                        } else {
                            "Try adjusting your search filters"
                        })),
                )
            }
            SearchStatus::Success(_, _) => {
                content = content.child(
                    div()
                        .px(px(24.))
                        .py(px(16.))
                        .flex()
                        .justify_between()
                        .t_sm()
                        .child("Results")
                        .child(
                            div()
                                .flex()
                                .text_color(p.muted)
                                .child("Found ")
                                .child(
                                    div()
                                        .text_color(hex(tw::ACCENT))
                                        .child(vm.total_matches.to_string()),
                                )
                                .child(" matches across ")
                                .child(
                                    div()
                                        .text_color(hex(tw::ACCENT))
                                        .child(vm.unique_pods.to_string()),
                                )
                                .child(" pods"),
                        ),
                );
                let header = div()
                    .id("search-table-header")
                    .test_support()
                    .flex_none()
                    .flex()
                    .items_center()
                    .bg(p.bg2)
                    .border_b_1()
                    .border_color(p.border)
                    .t_xs()
                    .text_color(p.muted)
                    .child(
                        div()
                            .w(px(96.))
                            .flex_none()
                            .px(px(16.))
                            .py(px(12.))
                            .id("search-header-timestamp")
                            .test_support()
                            .child(section_label("TIMESTAMP", cx)),
                    )
                    .child(
                        div()
                            .relative()
                            .w(px(self.pod_width))
                            .flex_none()
                            .px(px(16.))
                            .py(px(12.))
                            .id("search-header-pod")
                            .test_support()
                            .child(section_label("POD", cx))
                            .child(
                                div()
                                    .id("search-resize")
                                    .test_support()
                                    .absolute()
                                    .right_0()
                                    .top_0()
                                    .bottom_0()
                                    .w(px(4.))
                                    .cursor_col_resize()
                                    .hover(|s| s.bg(hexa(tw::ACCENT, 0.5)))
                                    .tooltip(|w, cx| Tooltip::new("Drag to resize").build(w, cx))
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, e: &MouseDownEvent, _, cx| {
                                            this.drag =
                                                Some((f32::from(e.position.x), this.pod_width));
                                            cx.stop_propagation();
                                        }),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .id("search-header-level")
                            .test_support()
                            .w(px(80.))
                            .flex_none()
                            .px(px(16.))
                            .py(px(12.))
                            .child(section_label("LEVEL", cx)),
                    )
                    .child(
                        div()
                            .id("search-header-message")
                            .test_support()
                            .flex_1()
                            .min_w_0()
                            .px(px(16.))
                            .py(px(12.))
                            .child(section_label("MESSAGE", cx)),
                    );
                let rows = div()
                    .id("search-table-scroll")
                    .test_support()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .child(
                        list(
                            self.list.clone(),
                            cx.processor(|this, ix: usize, window, cx| {
                                this.render_row(ix, window, cx)
                            }),
                        )
                        .size_full()
                        .min_w_0(),
                    );
                content = content.child(
                    div()
                        .id("search-table")
                        .flex_1()
                        .min_h_0()
                        .mx(px(24.))
                        .mb(px(24.))
                        .border_1()
                        .border_color(p.border)
                        .rounded(px(8.))
                        .overflow_hidden()
                        .flex()
                        .flex_col()
                        .child(header)
                        .child(rows),
                );
            }
        }
        let mut footer = div();
        if let Some(d) = data.filter(|d| !d.results.is_empty()) {
            let mut seen = std::collections::HashSet::new();
            footer = div()
                .px(px(24.))
                .py(px(8.))
                .border_t_1()
                .border_color(p.border)
                .flex()
                .justify_between()
                .child(
                    div().flex().gap(px(8.)).children(
                        d.results
                            .iter()
                            .filter(|r| seen.insert(r.pod_name.clone()))
                            .map(|r| {
                                let pod = r.pod_name.clone();
                                let suffix = pod
                                    .chars()
                                    .rev()
                                    .take(6)
                                    .collect::<String>()
                                    .chars()
                                    .rev()
                                    .collect::<String>();
                                div()
                                    .id(SharedString::from(format!("chip-{pod}")))
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .px(px(8.))
                                    .py(px(4.))
                                    .rounded(px(4.))
                                    .bg(p.bg3)
                                    .t_xs()
                                    .text_color(p.muted)
                                    .tooltip(move |w, cx| Tooltip::new(pod.clone()).build(w, cx))
                                    .child(dot(hex(tw::EMERALD_500), 6.))
                                    .child(format!("...{suffix}"))
                            }),
                    ),
                )
                .child(
                    Button::new("search-export", Variant::Secondary)
                        .size(BtnSize::Sm)
                        .icon(IconName::Download)
                        .label("Export CSV")
                        .on_click(cx.listener(|this, _, _, cx| this.export(cx))),
                );
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, _, cx| {
                if let Some((start, width)) = this.drag {
                    this.pod_width = (width + f32::from(e.position.x) - start).clamp(100., 500.);
                    this.list.remeasure();
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.drag = None),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.drag = None),
            )
            .child(form)
            .child(content)
            .child(footer)
            .child(self.detail.clone())
    }
}
impl LogSearchView {
    fn failure_list(&self, d: &crate::models::LogSearchResponse, cx: &Context<Self>) -> Div {
        let p = pal(cx);
        div()
            .mt(px(8.))
            .child(
                div()
                    .id("search-failures")
                    .cursor_pointer()
                    .child(format!("Failed containers ({})", d.failures.len()))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.failures_open = !this.failures_open;
                        cx.notify();
                    })),
            )
            .when(self.failures_open, |d0| {
                d0.child(
                    div()
                        .id("search-failure-list")
                        .max_h(px(160.))
                        .overflow_y_scroll()
                        .text_color(p.text2)
                        .children(d.failures.iter().map(|f| {
                            div().child(format!(
                                "{} / {}: {}",
                                f.pod_name, f.container_name, f.message
                            ))
                        })),
                )
            })
    }
    fn error_view(
        &self,
        error: &str,
        data: Option<&crate::models::LogSearchResponse>,
        cx: &Context<Self>,
    ) -> Div {
        div()
            .px(px(24.))
            .py(px(32.))
            .flex()
            .flex_col()
            .items_center()
            .child(
                div()
                    .t_lg()
                    .text_color(hex(tw::RED_500))
                    .child("Search failed"),
            )
            .child(div().t_sm().mt(px(8.)).child(error.to_owned()))
            .when_some(data, |d, data| d.child(self.failure_list(data, cx)))
            .child(
                Button::new("search-retry", Variant::Secondary)
                    .label("Retry")
                    .on_click(
                        cx.listener(|this, _, _, cx| this.submit(this.state.retry_snapshot(), cx)),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::export_csv;
    use gpui_kit::AppContext;

    fn boot_search_at(
        cx: &mut gpui_kit::TestAppContext,
        width: f32,
    ) -> crate::app::testing::Env {
        use crate::{
            app::{root::PodLogsApp, store::{Store, View}, testing::{Env, persisted, seeded}},
            ui::theme::{palette, set_palette},
        };
        use gpui_kit::{Bounds, WindowBounds, WindowOptions, point, px, size};
        let api = seeded(vec![crate::app::testing::deployment("api", 1, 1, 1)]);
        let window_api = api.clone();
        let (handle, app) = cx.update(|cx| {
            gpui_kit::init(cx);
            crate::ui::init(cx);
            set_palette(palette(true), cx);
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: point(px(0.), px(0.)),
                        size: size(px(width), px(700.)),
                    })),
                    ..Default::default()
                },
                cx,
                move |window, cx| {
                    let store = cx.new(|cx| Store::new(window_api, persisted(), None, cx));
                    cx.new(|cx| PodLogsApp::new(store, window, cx))
                },
            )
            .expect("open test window")
        });
        cx.run_until_parked();
        let store = app.read_with(cx, |a, _| a.store.clone());
        store.update(cx, |s, cx| s.set_view(View::Search, cx));
        cx.run_until_parked();
        Env { api, store, app, handle }
    }
    #[test]
    fn entry_at_skips_whole_containers() {
        use super::{entry_at, row_count};
        use crate::models::{LogEntry, LogSearchResponse, LogSearchResult};
        let e = |m: &str| LogEntry {
            timestamp: None,
            level: None,
            message: m.into(),
            raw: String::new(),
            is_json: false,
            pod_name: String::new(),
            container_name: String::new(),
        };
        let r = |pod: &str, entries: Vec<LogEntry>| LogSearchResult {
            pod_name: pod.into(),
            container_name: "c".into(),
            total_matches: entries.len() as i32,
            entries,
        };
        let d = LogSearchResponse {
            results: vec![
                r("p1", vec![e("a"), e("b")]),
                r("p2", vec![]),
                r("p3", vec![e("c")]),
            ],
            ..Default::default()
        };
        let got: Vec<_> = (0..4)
            .map(|i| entry_at(&d, i).map(|(r, e)| (r.pod_name.as_str(), e.message.as_str())))
            .collect();
        assert_eq!(
            got,
            [
                Some(("p1", "a")),
                Some(("p1", "b")),
                Some(("p3", "c")),
                None
            ]
        );
        assert_eq!(row_count(&d), 3);
    }
    #[test]
    fn export_csv_format() {
        let results = vec![crate::models::LogSearchResult {
            pod_name: "pod".into(),
            container_name: "app".into(),
            total_matches: 1,
            entries: vec![crate::models::LogEntry {
                timestamp: Some("2026-01-01T00:00:00Z".into()),
                level: None,
                message: "hello".into(),
                raw: "raw".into(),
                is_json: false,
                pod_name: "pod".into(),
                container_name: "app".into(),
            }],
        }];
        assert_eq!(export_csv(&results), "2026-01-01T00:00:00Z\tpod\t-\thello");
    }
    #[gpui_kit::test]
    fn enter_in_keyword_submits(cx: &mut gpui_kit::TestAppContext) {
        use crate::app::{
            store::View,
            testing::{boot, deployment, seeded},
        };
        use gpui_kit::test::TestWindowExt;
        let env = boot(cx, seeded(vec![deployment("api", 1, 1, 1)]), None);
        env.store.update(cx, |s, cx| {
            s.set_deployment(Some("api".into()), cx);
            s.set_view(View::Search, cx);
        });
        let input = env
            .app
            .read_with(cx, |a, cx| a.log_search.read(cx).keyword.clone());
        cx.update_window(env.handle, |_, w, cx| {
            w.render_frame(cx);
            input.update(cx, |i, cx| i.focus(w, cx));
            w.input("hello", cx);
            w.press("enter", cx);
        })
        .unwrap();
        cx.run_until_parked();
        assert_eq!(env.api.calls(crate::api::fake::Endpoint::SearchLogs), 1);
    }
    #[gpui_kit::test]
    fn search_disabled_without_deployment(cx: &mut gpui_kit::TestAppContext) {
        use crate::app::{
            store::View,
            testing::{boot, seeded},
        };
        use gpui_kit::test::TestWindowExt;
        let env = boot(cx, seeded(vec![]), None);
        env.store.update(cx, |s, cx| s.set_view(View::Search, cx));
        cx.update_window(env.handle, |_, w, cx| {
            w.render_frame(cx);
            w.click("search-submit", cx);
        })
        .unwrap();
        assert_eq!(env.api.calls(crate::api::fake::Endpoint::SearchLogs), 0);
    }
    #[gpui_kit::test]
    fn compact_search_form_fits_and_aligns_at_minimum_and_wide_widths(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        use gpui_kit::{px, test::TestWindowExt};
        for width in [1000., 1400.] {
            let env = boot_search_at(cx, width);
            let time = env.app.read_with(cx, |a, cx| a.log_search.read(cx).time.clone());
            time.update(cx, |d, cx| d.set_value(Some("48h".into()), cx));
            cx.update_window(env.handle, |_, w, cx| {
                w.render_frame(cx);
                let row = w.find("search-controls-row").bounds();
                let time = w.find("search-time-control").bounds();
                let button = w.find("search-submit").bounds();
                let trigger = w.within("search-time-control").find("dd-trigger").bounds();
                let deployment = w.within("search-deployment").find("dd-trigger").bounds();
                let keyword = w.find("search-keyword-control").bounds();
                let dropdowns = w.within("search-controls-row").find_all("dd-trigger");
                assert!(button.origin.x >= time.origin.x + time.size.width);
                assert!(button.origin.x + button.size.width <= row.origin.x + row.size.width);
                assert!(keyword.origin.x >= row.origin.x);
                assert!(keyword.origin.x + keyword.size.width <= row.origin.x + row.size.width);
                assert_eq!(button.origin.y + button.size.height, row.origin.y + row.size.height);
                assert_eq!(trigger.origin.y + trigger.size.height, button.origin.y + button.size.height);
                assert!(deployment.size.width > px(0.));
                assert!(keyword.size.width > px(0.));
                assert_eq!(dropdowns.len(), 3);
                assert!(dropdowns.iter().all(|control| {
                    let bounds = control.bounds();
                    bounds.origin.x >= row.origin.x
                        && bounds.origin.x + bounds.size.width <= row.origin.x + row.size.width
                }));
                assert!(row.origin.x >= px(260.));
                assert!(row.origin.x + row.size.width <= px(width));
                assert_eq!(time.size.width, px(144.));
                assert!(trigger.size.width >= px(144.));
            })
            .unwrap();
        }
    }
    #[gpui_kit::test]
    fn search_filter_warning_only_appears_after_applied_filters_change(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        use crate::{
            api::fake::Endpoint,
            app::{store::View, testing::{boot, deployment, seeded}},
        };
        use gpui_kit::test::TestWindowExt;
        let env = boot(cx, seeded(vec![deployment("api", 1, 1, 1)]), None);
        env.store.update(cx, |s, cx| {
            s.set_deployment(Some("api".into()), cx);
            s.set_view(View::Search, cx);
        });
        env.api.reply(Endpoint::SearchLogs, Ok(serde_json::json!({
            "results": [], "failures": [], "total_containers": 0, "successful_containers": 0
        })));
        cx.update_window(env.handle, |_, w, cx| {
            w.render_frame(cx);
            assert!(w.try_find("search-filter-warning").is_none());
            w.click("search-submit", cx);
        })
        .unwrap();
        cx.run_until_parked();
        let input = env.app.read_with(cx, |a, cx| a.log_search.read(cx).keyword.clone());
        cx.update_window(env.handle, |_, w, cx| {
            input.update(cx, |i, cx| i.focus(w, cx));
            w.input("changed", cx);
            w.render_frame(cx);
            assert!(w.try_find("search-filter-warning").is_some());
        })
        .unwrap();
    }
    #[test]
    fn highlight_splits_message_into_before_mark_after() {
        assert_eq!(
            super::message_segments("Failed to connect", "CONNECT"),
            Some(("Failed to ".into(), "connect".into(), "".into()))
        );
        assert_eq!(
            super::message_segments("one ONE", "one"),
            Some(("".into(), "one".into(), " ONE".into()))
        );
        assert_eq!(super::message_segments("abc", ""), None);
    }
    #[gpui_kit::test]
    fn error_row_with_match_renders_painted_mark(cx: &mut gpui_kit::TestAppContext) {
        use crate::api::fake::Endpoint;
        use crate::app::{
            store::View,
            testing::{boot, deployment, seeded},
        };
        use gpui_kit::test::TestWindowExt;
        let api = seeded(vec![deployment("api", 1, 1, 1)]);
        let message = format!("{} connect refused {}", "x".repeat(300), "y".repeat(300));
        api.reply(Endpoint::SearchLogs, Ok(serde_json::json!({"results":[{"pod_name":"p","container_name":"c","total_matches":1,"entries":[{"timestamp":"2026-01-01T00:00:00Z","level":"ERROR","message":message,"raw":"raw","is_json":false,"pod_name":"p","container_name":"c"}]}],"failures":[],"total_containers":1,"successful_containers":1})));
        let env = boot(cx, api, None);
        env.store.update(cx, |s, cx| {
            s.set_deployment(Some("api".into()), cx);
            s.set_view(View::Search, cx);
        });
        let input = env
            .app
            .read_with(cx, |a, cx| a.log_search.read(cx).keyword.clone());
        cx.update_window(env.handle, |_, w, cx| {
            w.render_frame(cx);
            input.update(cx, |i, cx| i.focus(w, cx));
            w.input("CONNECT", cx);
            w.press("enter", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(env.handle, |_, w, cx| {
            w.render_frame(cx);
            assert!(w.try_find(("search-row", 0usize)).is_some());
        })
        .unwrap();
    }
    fn boot_search(cx: &mut gpui_kit::TestAppContext, rows: usize) -> crate::app::testing::Env {
        boot_search_entries(
            cx,
            (0..rows).map(|i| ("INFO", format!("line {i}"))).collect(),
        )
    }
    fn boot_search_entries(
        cx: &mut gpui_kit::TestAppContext,
        input: Vec<(&str, String)>,
    ) -> crate::app::testing::Env {
        use crate::api::fake::Endpoint;
        use crate::app::{
            store::View,
            testing::{boot, deployment, seeded},
        };
        use gpui_kit::test::TestWindowExt;
        let api = seeded(vec![deployment("api", 1, 1, 1)]);
        let rows = input.len();
        let entries: Vec<_> = input.into_iter().map(|(level, message)| serde_json::json!({"timestamp":"2026-01-01T00:00:00Z","level":level,"message":message,"raw":"raw","is_json":false,"pod_name":"p","container_name":"c"})).collect();
        let results = if rows == 0 {
            vec![]
        } else {
            vec![
                serde_json::json!({"pod_name":"p","container_name":"c","total_matches":rows,"entries":entries}),
            ]
        };
        api.reply(Endpoint::SearchLogs, Ok(serde_json::json!({"results":results,"failures":[],"total_containers":1,"successful_containers":1})));
        let env = boot(cx, api, None);
        env.store.update(cx, |s, cx| {
            s.set_deployment(Some("api".into()), cx);
            s.set_view(View::Search, cx);
        });
        cx.update_window(env.handle, |_, w, cx| {
            w.render_frame(cx);
            w.click("search-submit", cx);
        })
        .unwrap();
        cx.run_until_parked();
        env
    }
    #[gpui_kit::test]
    fn search_table_builds_only_visible_rows(cx: &mut gpui_kit::TestAppContext) {
        use gpui_kit::test::TestWindowExt;
        let env = boot_search(cx, 5000);
        cx.update_window(env.handle, |_, w, cx| {
            w.render_frame(cx);
            assert!(w.try_find(("search-row", 0usize)).is_some());
            assert!(
                w.try_find(("search-row", 200usize)).is_none(),
                "off-screen rows must not be built"
            );
        })
        .unwrap();
    }
    #[gpui_kit::test]
    fn search_table_scrolls_through_virtualized_rows(cx: &mut gpui_kit::TestAppContext) {
        use gpui_kit::{ScrollDelta, point, px, test::TestWindowExt};
        let env = boot_search(cx, 5000);
        cx.update_window(env.handle, |_, w, cx| {
            for _ in 0..30 {
                w.scroll(
                    "search-table-scroll",
                    ScrollDelta::Pixels(point(px(0.), px(-300.))),
                    cx,
                );
            }
            assert!(w.try_find(("search-row", 0usize)).is_none());
            assert!((150..400usize).any(|i| w.try_find(("search-row", i)).is_some()));
        })
        .unwrap();
    }
    #[gpui_kit::test]
    fn new_search_resets_rows_to_top(cx: &mut gpui_kit::TestAppContext) {
        use crate::api::fake::Endpoint;
        use gpui_kit::{ScrollDelta, point, px, test::TestWindowExt};
        let env = boot_search(cx, 5000);
        cx.update_window(env.handle, |_, w, cx| {
            for _ in 0..10 {
                w.scroll(
                    "search-table-scroll",
                    ScrollDelta::Pixels(point(px(0.), px(-300.))),
                    cx,
                );
            }
        })
        .unwrap();
        let entries: Vec<_> = (0..3).map(|i| serde_json::json!({"timestamp":null,"level":"INFO","message":format!("line {i}"),"raw":"raw","is_json":false,"pod_name":"p","container_name":"c"})).collect();
        env.api.reply(Endpoint::SearchLogs, Ok(serde_json::json!({"results":[{"pod_name":"p","container_name":"c","total_matches":3,"entries":entries}],"failures":[],"total_containers":1,"successful_containers":1})));
        cx.update_window(env.handle, |_, w, cx| {
            w.click("search-submit", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(env.handle, |_, w, cx| {
            w.render_frame(cx);
            assert!(w.try_find(("search-row", 0usize)).is_some());
            assert!(w.try_find(("search-row", 2usize)).is_some());
            assert!(w.try_find(("search-row", 3usize)).is_none());
        })
        .unwrap();
    }
    #[gpui_kit::test]
    fn pod_resize_remeasures_error_rows(cx: &mut gpui_kit::TestAppContext) {
        use gpui_kit::{point, px, test::TestWindowExt};
        let env = boot_search_entries(cx, vec![("ERROR", "e".repeat(40)), ("ERROR", "e".into())]);
        cx.update_window(env.handle, |_, w, cx| {
            w.render_frame(cx);
            let h0 = w.find(("search-row", 0usize)).bounds().size.height;
            let message_height = w.find(("search-message-cell", 0usize)).bounds().size.height;
            let h1 = w.find(("search-row", 1usize)).bounds().size.height;
            assert_eq!(h0, h1);
            let resize = w.find("search-resize").bounds();
            let start = point(
                resize.origin.x + resize.size.width / 2.,
                resize.origin.y + resize.size.height / 2.,
            );
            w.drag(start, point(start.x + px(292.), start.y), cx);
            w.render_frame(cx);
            let row0 = w.find(("search-row", 0usize)).bounds();
            let row1 = w.find(("search-row", 1usize)).bounds();
            assert!(w.find(("search-message-cell", 0usize)).bounds().size.height > message_height);
            assert!(row1.origin.y >= row0.origin.y + row0.size.height);
        })
        .unwrap();
    }
    #[gpui_kit::test]
    #[ignore]
    fn search_scroll_frame_time_probe(cx: &mut gpui_kit::TestAppContext) {
        use gpui_kit::{ScrollDelta, point, px, test::TestWindowExt};
        for n in [100, 1000, 5000] {
            let env = boot_search(cx, n);
            cx.update_window(env.handle, |_, w, cx| {
                w.render_frame(cx);
                let start = std::time::Instant::now();
                for _ in 0..10 {
                    w.scroll(
                        "search-table-scroll",
                        ScrollDelta::Pixels(point(px(0.), px(-20.))),
                        cx,
                    );
                    w.render_frame(cx);
                }
                eprintln!("rows={n}: {:?}/frame", start.elapsed() / 10);
            })
            .unwrap();
        }
    }
    #[gpui_kit::test]
    fn results_headers_use_tracked_labels(cx: &mut gpui_kit::TestAppContext) {
        use gpui_kit::test::TestWindowExt;
        let env = boot_search(cx, 1);
        cx.update_window(env.handle, |_, w, cx| {
            w.render_frame(cx);
            for id in [
                "search-header-timestamp",
                "search-header-pod",
                "search-header-level",
                "search-header-message",
            ] {
                assert!(w.try_find(id).is_some(), "missing {id}");
            }
        })
        .unwrap();
    }
    #[gpui_kit::test]
    fn empty_states_show_search_icon(cx: &mut gpui_kit::TestAppContext) {
        use crate::app::{
            store::View,
            testing::{boot, seeded},
        };
        use gpui_kit::test::TestWindowExt;
        let env = boot(cx, seeded(vec![]), None);
        env.store.update(cx, |s, cx| s.set_view(View::Search, cx));
        cx.update_window(env.handle, |_, w, cx| {
            w.render_frame(cx);
            assert!(w.try_find("search-initial-icon").is_some());
        })
        .unwrap();
        let env = boot_search(cx, 0);
        cx.update_window(env.handle, |_, w, cx| {
            w.render_frame(cx);
            assert!(w.try_find("search-empty-icon").is_some());
        })
        .unwrap();
    }
    #[gpui_kit::test]
    fn header_stays_visible_after_scrolling_results(cx: &mut gpui_kit::TestAppContext) {
        use gpui_kit::{ScrollDelta, point, px, test::TestWindowExt};
        let env = boot_search(cx, 100);
        cx.update_window(env.handle, |_, w, cx| {
            w.render_frame(cx);
            let resize = w.find("search-resize").bounds();
            let start = point(
                resize.origin.x + resize.size.width / 2.,
                resize.origin.y + resize.size.height / 2.,
            );
            w.drag(start, point(start.x + px(50.), start.y), cx);
            w.render_frame(cx);
            let head_pod = w.find("search-header-pod").bounds();
            let row_pod = w.find(("search-pod-cell", 0usize)).bounds();
            assert_eq!(head_pod.size.width, px(258.));
            assert_eq!(head_pod.size.width, row_pod.size.width);
            assert_eq!(head_pod.origin.x, row_pod.origin.x);
            let header = w.find("search-table-header").bounds();
            let before = w.find(("search-row", 0usize)).bounds();
            w.scroll(
                "search-table-scroll",
                ScrollDelta::Pixels(point(px(0.), px(-20.))),
                cx,
            );
            let after = w.find(("search-row", 0usize)).bounds();
            assert!(after.origin.y < before.origin.y, "rows did not scroll");
            assert_eq!(w.find("search-table-header").bounds(), header);
        })
        .unwrap();
    }
}
