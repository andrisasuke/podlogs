//! Virtualized pod logs, cached filtering, native export and a reusable detail modal.
use std::{rc::Rc, time::Duration};

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    VirtualListScrollHandle,
    input::{InputEvent, InputState},
    v_virtual_list,
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::log_detail_modal::{LogDetailModal, message_color};
use super::store::{Store, View};
use crate::format::{TimeRange, format_timestamp, thousands};
use crate::models::LogEntry;
use crate::ui::badge::{dot, level_badge};
use crate::ui::button::{BtnSize, Button, Variant};
use crate::ui::dropdown::{DropdownEvent, DropdownState};
use crate::ui::input::search_input;
use crate::ui::section_label;
use crate::ui::skeleton::log_skeleton;
use crate::ui::theme::{Tw, hex, hexa, pal, tw};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Mode {
    #[default]
    Json,
    Raw,
}

pub fn row_height(mode: Mode, level: Option<&str>) -> f32 {
    match (mode, level) {
        (Mode::Raw, _) => 24.,
        (Mode::Json, Some("ERROR")) => 52.,
        _ => 36.,
    }
}

pub fn export_txt(rows: &[&LogEntry]) -> String {
    rows.iter().map(|entry| entry.raw.as_str()).collect::<Vec<_>>().join("\n")
}

#[derive(Default)]
pub struct FilterCache {
    key: Option<(String, usize, usize)>,
    indices: Rc<Vec<usize>>,
    revision: u64,
}

impl FilterCache {
    pub fn rows(&mut self, logs: &[LogEntry], filter: &str) -> &[usize] {
        let ptr = logs.as_ptr() as usize;
        if self.key.as_ref().is_none_or(|(f, len, p)| f != filter || *len != logs.len() || *p != ptr) {
            let lower = filter.to_lowercase();
            self.indices = Rc::new(
                logs.iter()
                    .enumerate()
                    .filter_map(|(i, log)| {
                        (lower.is_empty() || log.message.to_lowercase().contains(&lower) || log.raw.to_lowercase().contains(&lower))
                            .then_some(i)
                    })
                    .collect(),
            );
            self.key = Some((filter.to_owned(), logs.len(), ptr));
            self.revision += 1;
        }
        &self.indices
    }
}

pub fn near_bottom(offset: f32, max_offset: f32) -> bool {
    (max_offset + offset).max(0.) < 100.
}

pub struct LogViewerView {
    store: Entity<Store>,
    container: Entity<DropdownState>,
    time: Entity<DropdownState>,
    pub(crate) filter: Entity<InputState>,
    pub(crate) detail: Entity<LogDetailModal>,
    mode: Mode,
    scope: Option<(String, String, String)>,
    scroll: VirtualListScrollHandle,
    auto_scroll: bool,
    last_count: usize,
    cache: FilterCache,
    sizes: Rc<Vec<Size<Pixels>>>,
    layout_key: Option<(u64, Mode)>,
    _subs: Vec<Subscription>,
}

impl LogViewerView {
    pub fn new(store: Entity<Store>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let container = cx.new(|cx| DropdownState::new("Select...", None, window, cx));
        let time = cx.new(|cx| DropdownState::new("Select...", None, window, cx));
        time.update(cx, |d, cx| {
            d.set_options(TimeRange::ALL.into_iter().map(|t| (t.key().to_owned(), t.label().to_owned())).collect(), cx)
        });
        let filter = cx.new(|cx| InputState::new(window, cx).placeholder("Filter logs..."));
        let detail = cx.new(|_| LogDetailModal::new());
        let subs = vec![
            cx.observe_in(&store, window, |this, _, window, cx| this.sync(window, cx)),
            cx.subscribe(&container, |this, _, event: &DropdownEvent, cx| {
                let DropdownEvent::Change(value) = event;
                this.store.update(cx, |s, cx| s.set_log_container(value.clone(), cx));
            }),
            cx.subscribe(&time, |this, _, event: &DropdownEvent, cx| {
                let DropdownEvent::Change(value) = event;
                if let Some(range) = TimeRange::ALL.into_iter().find(|t| t.key() == value) {
                    this.store.update(cx, |s, cx| s.set_log_time_range(range, cx));
                }
            }),
            cx.subscribe(&filter, |_, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            }),
        ];
        let mut this = Self {
            store,
            container,
            time,
            filter,
            detail,
            mode: Mode::Json,
            scope: None,
            scroll: VirtualListScrollHandle::new(),
            auto_scroll: true,
            last_count: 0,
            cache: FilterCache::default(),
            sizes: Rc::default(),
            layout_key: None,
            _subs: subs,
        };
        this.sync(window, cx);
        this
    }

    fn sync(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let s = self.store.read(cx);
        let scope = (s.view == View::Logs)
            .then(|| s.log_viewer_pod.as_ref().map(|pod| (s.context.clone(), s.namespace.clone(), pod.clone())))
            .flatten();
        let options =
            s.pod_details.data().map(|p| p.containers.iter().map(|c| (c.name.clone(), c.name.clone())).collect()).unwrap_or_default();
        let selected = s.log_container.clone();
        let time = s.log_time_range.key().to_owned();
        if scope != self.scope {
            self.scope = scope;
            self.mode = Mode::Json;
            self.auto_scroll = true;
            self.last_count = 0;
            self.cache = FilterCache::default();
            self.layout_key = None;
            self.sizes = Rc::default();
            self.scroll = VirtualListScrollHandle::new();
            self.detail.update(cx, |d, cx| d.close(cx));
            self.filter.update(cx, |f, cx| f.set_value("", window, cx));
        }
        self.container.update(cx, |d, cx| {
            d.set_options(options, cx);
            d.set_value(selected, cx);
        });
        self.time.update(cx, |d, cx| d.set_value(Some(time), cx));
        cx.notify();
    }

    fn export(&mut self, cx: &mut Context<Self>) {
        let s = self.store.read(cx);
        let logs = s.pod_logs.data().map(Vec::as_slice).unwrap_or_default();
        let filter = self.filter.read(cx).value();
        let indices = self.cache.rows(logs, filter.as_ref());
        let rows: Vec<_> = indices.iter().filter_map(|&i| logs.get(i)).collect();
        let content = export_txt(&rows);
        let filename = format!("{}-{}-logs.txt", s.log_viewer_pod.as_deref().unwrap_or(""), s.log_container.as_deref().unwrap_or(""));
        let directory = dirs::home_dir().unwrap_or_else(std::env::temp_dir);
        let dialog = cx.prompt_for_new_path(&directory, Some(&filename));
        cx.spawn(async move |_, cx| {
            let result = match dialog.await {
                Ok(Ok(Some(path))) => {
                    cx.background_executor().spawn(async move { std::fs::write(path, content).map_err(|e| e.to_string()) }).await
                }
                Ok(Ok(None)) => return, // A cancelled save dialog never writes a file.
                Ok(Err(error)) => Err(error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            if let Err(error) = result {
                eprintln!("Export error: {error}");
            }
        })
        .detach();
        cx.notify();
    }

    fn row(&self, entry: &LogEntry, index: usize, cx: &Context<Self>) -> AnyElement {
        let p = pal(cx);
        let mut hover = p.bg3;
        hover.a = 0.5;
        let height = row_height(self.mode, entry.level.as_deref());
        let chosen = entry.clone(); // Only visible rows are cloned, never the full log buffer.
        let base = div()
            .id(("log-row", index))
            .test_support()
            .w_full()
            .h(px(height))
            .overflow_hidden()
            .px(px(24.))
            .mono()
            .t_sm()
            .cursor_pointer()
            .hover(move |s| s.bg(hover))
            .on_click(cx.listener(move |this, _, _, cx| this.detail.update(cx, |d, cx| d.open(chosen.clone(), cx))));
        if self.mode == Mode::Raw {
            return base
                .py(px(4.))
                .text_color(message_color(entry.level.as_deref(), p.dark, p.text2))
                .child(div().truncate().child(entry.raw.clone()))
                .into_any_element();
        }
        let is_error = entry.level.as_deref() == Some("ERROR");
        base.flex()
            .py(px(6.))
            .when(is_error, |d| d.items_start().bg(hexa(tw::RED_500, 0.05)))
            .when(!is_error, |d| d.items_center())
            .when(entry.level.as_deref() == Some("WARN"), |d| d.bg(hexa(tw::AMBER_500, 0.05)))
            .child(div().w(px(176.)).flex_none().text_color(p.muted).child(format_timestamp(entry.timestamp.as_deref())))
            .child(
                div()
                    .w(px(64.))
                    .flex_none()
                    .flex()
                    .when_some(entry.level.as_deref().filter(|l| !l.is_empty()), |d, level| d.child(level_badge(level, cx))),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_color(message_color(entry.level.as_deref(), p.dark, if entry.level.is_none() { p.text2 } else { p.text1 }))
                    .when(is_error, |d| d.line_clamp(2))
                    .when(!is_error, |d| d.truncate())
                    .child(entry.message.clone()),
            )
            .into_any_element()
    }

    fn mode_button(&self, id: &'static str, label: &'static str, mode: Mode, cx: &Context<Self>) -> AnyElement {
        let p = pal(cx);
        let active = self.mode == mode;
        div()
            .id(id)
            .test_support()
            .px(px(12.))
            .py(px(6.))
            .t_sm()
            .font_weight(FontWeight::MEDIUM)
            .cursor_pointer()
            .text_color(if active { white() } else { p.text2 })
            .when(active, |d| d.bg(hex(tw::ACCENT)))
            .when(!active, |d| d.hover(|s| s.bg(p.bg3)))
            .child(label)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.mode = mode;
                cx.notify();
            }))
            .into_any_element()
    }
}

impl Render for LogViewerView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = pal(cx);
        let s = self.store.read(cx);
        let Some(pod) = s.log_viewer_pod.clone() else {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(p.muted)
                .child("Select a pod to view logs")
                .into_any_element();
        };
        let (loading, fetching) = (s.pod_logs.is_loading(), s.pod_logs.is_fetching());
        let logs = s.pod_logs.data().map(Vec::as_slice).unwrap_or_default();
        let filter = self.filter.read(cx).value();
        self.cache.rows(logs, filter.as_ref());
        let indices = self.cache.indices.clone();
        let count = indices.len();
        let key = (self.cache.revision, self.mode);
        if self.layout_key != Some(key) {
            self.sizes = Rc::new(indices.iter().map(|&i| size(px(1.), px(row_height(self.mode, logs[i].level.as_deref())))).collect());
            self.layout_key = Some(key);
        }
        // Geometry still describes the previous frame, so new data cannot turn OFF into ON.
        if self.scroll.bounds().size.height > px(0.) {
            self.auto_scroll = near_bottom(f32::from(self.scroll.offset().y), f32::from(self.scroll.max_offset().y));
        }
        if count != self.last_count && self.auto_scroll && count > 0 {
            self.scroll.scroll_to_item(count - 1, ScrollStrategy::Bottom);
        }
        self.last_count = count;
        let back = self.store.clone();
        let settings = self.store.clone();
        let json_button = self.mode_button("log-mode-json", "JSON", Mode::Json, cx);
        let raw_button = self.mode_button("log-mode-raw", "Raw", Mode::Raw, cx);
        let header = div()
            .px(px(24.))
            .py(px(16.))
            .flex_none()
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
                            .flex()
                            .items_center()
                            .gap(px(16.))
                            .child(
                                Button::new("log-back", Variant::Ghost)
                                    .size(BtnSize::Sm)
                                    .icon(IconName::ArrowLeft)
                                    .label("Back")
                                    .on_click(move |_, _, cx| back.update(cx, |s, cx| s.close_log_viewer(cx))),
                            )
                            .child(
                                div()
                                    .child(div().t_lg().font_weight(FontWeight::SEMIBOLD).text_color(p.text1).child("Logs"))
                                    .child(div().t_sm().mono().text_color(p.muted).child(pod)),
                            ),
                    )
                    .child(
                        Button::new("log-settings", Variant::Ghost)
                            .size(BtnSize::Icon)
                            .icon(IconName::Settings)
                            .tooltip("Settings")
                            .on_click(move |_, _, cx| settings.update(cx, |s, cx| s.open_settings(cx))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(16.))
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(div().t_sm().text_color(p.muted).child("Container:"))
                            .child(div().id("log-container-select").w(px(160.)).child(self.container.clone())),
                    )
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(div().t_sm().text_color(p.muted).child("Time:"))
                            .child(div().id("log-time-select").w(px(176.)).child(self.time.clone())),
                    )
                    .child(div().flex_1().min_w_0().child(search_input(&self.filter, cx)))
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .rounded(px(6.))
                            .border_1()
                            .border_color(p.border)
                            .overflow_hidden()
                            .child(json_button)
                            .child(raw_button),
                    ),
            );
        let content = if loading {
            div().flex_1().min_h_0().overflow_hidden().child(log_skeleton(15, cx)).into_any_element()
        } else {
            let list = v_virtual_list(cx.entity(), "log-list", self.sizes.clone(), move |this, range, _, cx| {
                let store = this.store.clone();
                let logs = store.read(cx).pod_logs.data().map(Vec::as_slice).unwrap_or_default();
                range
                    .filter_map(|i| indices.get(i).and_then(|&source| logs.get(source).map(|entry| (source, entry))))
                    .map(|(i, entry)| this.row(entry, i, cx))
                    .collect::<Vec<_>>()
            })
            .track_scroll(&self.scroll)
            .mono()
            .t_sm();
            div()
                .flex_1()
                .min_h_0()
                .flex()
                .flex_col()
                .when(self.mode == Mode::Json, |d| {
                    d.child(
                        div()
                            .id("log-json-header")
                            .test_support()
                            .flex_none()
                            .flex()
                            .items_center()
                            .px(px(24.))
                            .py(px(8.))
                            .border_b_1()
                            .border_color(p.border)
                            .child(div().w(px(176.)).flex_none().child(section_label("TIMESTAMP", cx)))
                            .child(div().w(px(64.)).flex_none().child(section_label("LEVEL", cx)))
                            .child(div().flex_1().child(section_label("MESSAGE", cx))),
                    )
                })
                .child(div().id("log-scroll-area").test_support().flex_1().min_h_0().child(list))
                .into_any_element()
        };
        let sync = div()
            .id(if fetching { "log-syncing" } else { "log-live" })
            .test_support()
            .flex()
            .items_center()
            .gap(px(6.))
            .child(dot(hex(if fetching { tw::AMBER_500 } else { tw::EMERALD_500 }), 8.).map(|d| {
                if fetching {
                    d.with_animation("log-pulse", Animation::new(Duration::from_secs(2)).repeat(), |el, t| {
                        el.opacity(0.75 + 0.25 * (t * std::f32::consts::TAU).cos())
                    })
                    .into_any_element()
                } else {
                    d.into_any_element()
                }
            }))
            .child(if fetching { "Syncing..." } else { "Live" });
        div()
            .id("log-viewer")
            .test_support()
            .size_full()
            .flex()
            .flex_col()
            .child(header)
            .child(content)
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(24.))
                    .py(px(8.))
                    .border_t_1()
                    .border_color(p.border)
                    .t_sm()
                    .text_color(p.muted)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .child(sync)
                            .child(div().id("log-line-count").test_support().child(format!("{} lines", thousands(count))))
                            .child(
                                div()
                                    .id("log-auto-scroll")
                                    .test_support()
                                    .child(format!("Auto-scroll: {}", if self.auto_scroll { "ON" } else { "OFF" })),
                            ),
                    )
                    .child(
                        Button::new("log-export", Variant::Secondary)
                            .size(BtnSize::Sm)
                            .icon(IconName::Download)
                            .label("Export")
                            .on_click(cx.listener(|this, _, _, cx| this.export(cx))),
                    ),
            )
            .child(self.detail.clone())
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{AppContext, TestAppContext};
    use serde_json::json;

    use super::{FilterCache, Mode, export_txt, near_bottom, row_height};
    use crate::api::fake::Endpoint;
    use crate::app::testing::{Env, boot, many, pod, temp_state, with_pods};
    use crate::models::LogEntry;

    pub fn entry(message: &str, level: Option<&str>) -> LogEntry {
        LogEntry {
            timestamp: Some("2026-11-11T12:34:56Z".into()),
            level: level.map(str::to_owned),
            message: message.into(),
            raw: format!("2026-11-11T12:34:56Z {message}"),
            is_json: false,
            pod_name: "api-123".into(),
            container_name: "app".into(),
        }
    }

    fn boot_logs(cx: &mut TestAppContext, entries: &[LogEntry]) -> Env {
        let api = with_pods(vec![pod("api-123", "Running")]);
        let containers: Vec<_> = ["app", "sidecar"]
            .into_iter()
            .map(|name| {
                json!({
                    "name": name, "image": "api:v1", "ready": true, "restart_count": 0, "state": "Running", "env_vars": [], "ports": [],
                    "resources": {"cpu_request": null, "cpu_limit": null, "memory_request": null, "memory_limit": null}
                })
            })
            .collect();
        many(
            &api,
            Endpoint::PodDetails,
            json!({
                "name": "api-123", "namespace": "default", "status": "Running", "node": "worker", "ip": "10.0.0.1",
                "start_time": null, "labels": {}, "annotations": {}, "conditions": [],
                "containers": containers
            }),
            20,
        );
        many(&api, Endpoint::PodLogs, serde_json::to_value(entries).unwrap(), 50);
        let env = boot(cx, api, None);
        env.store.update(cx, |s, cx| s.open_log_viewer("api-123".into(), cx));
        cx.run_until_parked();
        env
    }

    #[test]
    fn filter_cache_recomputes_only_on_input_change() {
        let mut logs = vec![entry("first", Some("INFO")), entry("HELLO world", Some("WARN"))];
        let mut cache = FilterCache::default();
        for _ in 0..3 {
            assert_eq!(cache.rows(&logs, "hello"), &[1]);
        }
        assert_eq!(cache.revision, 1);
        assert_eq!(cache.rows(&logs, "FIRST"), &[0]);
        assert_eq!(cache.revision, 2);
        let replacement = vec![entry("changed", None), entry("other", None)];
        assert!(cache.rows(&replacement, "FIRST").is_empty());
        assert_eq!(cache.revision, 3);
        logs.push(entry("first again", None));
        assert_eq!(cache.rows(&logs, "FIRST"), &[0, 2]);
        assert_eq!(cache.revision, 4);
        assert_eq!(cache.rows(&logs, ""), &[0, 1, 2]);
        assert_eq!(cache.rows(&logs, "12:34:56Z"), &[0, 1, 2]); // raw content also matches
    }

    #[test]
    fn row_heights_ignore_content_length() {
        for log in [entry(&"x".repeat(10_000), Some("INFO")), entry("first\nsecond\nthird", Some("WARN"))] {
            assert_eq!(row_height(Mode::Json, log.level.as_deref()), 36.);
        }
        assert_eq!(row_height(Mode::Json, Some("ERROR")), 52.);
        assert_eq!(row_height(Mode::Raw, Some("ERROR")), 24.);
        assert_eq!(row_height(Mode::Raw, None), 24.);
    }

    #[test]
    fn export_txt_joins_raw_lines() {
        let a = entry("first", None);
        let b = entry("second", Some("ERROR"));
        assert_eq!(export_txt(&[&a, &b]), format!("{}\n{}", a.raw, b.raw));
        assert_eq!(export_txt(&[]), "");
    }

    #[test]
    fn auto_scroll_uses_strict_hundred_pixel_threshold() {
        assert!(near_bottom(0., 0.));
        assert!(near_bottom(-901., 1000.));
        assert!(!near_bottom(-900., 1000.));
        assert!(!near_bottom(-100., 1000.));
    }

    #[gpui_kit::test]
    fn clicking_row_opens_detail_modal_and_escape_closes(cx: &mut TestAppContext) {
        let env = boot_logs(cx, &[entry("hello", Some("INFO"))]);
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            window.click(("log-row", 0usize), cx);
            assert!(window.try_find("log-detail-modal").is_some());
            window.press("escape", cx);
            assert!(window.try_find("log-detail-modal").is_none());
        })
        .unwrap();
    }

    #[gpui_kit::test]
    fn json_raw_toggle_switches_header(cx: &mut TestAppContext) {
        let env = boot_logs(cx, &[entry("hello", Some("ERROR"))]);
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("log-json-header").is_some());
            window.click("log-mode-raw", cx);
            assert!(window.try_find("log-json-header").is_none());
            assert_eq!(window.find(("log-row", 0usize)).bounds().size.height, gpui_kit::px(24.));
            window.click("log-mode-json", cx);
            assert!(window.try_find("log-json-header").is_some());
            assert_eq!(window.find(("log-row", 0usize)).bounds().size.height, gpui_kit::px(52.));
        })
        .unwrap();
    }

    #[gpui_kit::test]
    fn container_and_time_controls_refetch_logs(cx: &mut TestAppContext) {
        let env = boot_logs(cx, &[entry("hello", Some("INFO"))]);
        let before = env.api.calls(Endpoint::PodLogs);
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            window.within("log-container-select").click("dd-trigger", cx);
            window.click("dd-opt-sidecar", cx);
        })
        .unwrap();
        cx.run_until_parked();
        assert_eq!(env.store.read_with(cx, |s, _| s.log_container.clone()), Some("sidecar".into()));
        assert_eq!(env.api.calls(Endpoint::PodLogs), before + 1);
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            window.within("log-time-select").click("dd-trigger", cx);
            window.click("dd-opt-1h", cx);
        })
        .unwrap();
        cx.run_until_parked();
        assert_eq!(env.store.read_with(cx, |s, _| s.log_time_range), crate::format::TimeRange::H1);
        assert_eq!(env.api.calls(Endpoint::PodLogs), before + 2);
    }

    #[gpui_kit::test]
    fn export_saves_only_filtered_raw_logs_and_cancel_writes_nothing(cx: &mut TestAppContext) {
        let first = entry("first", None);
        let second = entry("second", Some("INFO"));
        let env = boot_logs(cx, &[first, second.clone()]);
        let path = temp_state("export-txt").with_file_name("logs.txt");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let filter = env.app.read_with(cx, |a, cx| a.log_viewer.read(cx).filter.clone());
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            filter.update(cx, |f, cx| f.focus(window, cx));
            window.input("SECOND", cx);
            window.click("log-export", cx);
        })
        .unwrap();
        assert!(cx.did_prompt_for_new_path());
        cx.simulate_new_path_selection(|_| None);
        cx.run_until_parked();
        assert!(!path.exists());
        cx.update_window(env.handle, |_, window, cx| window.click("log-export", cx)).unwrap();
        cx.simulate_new_path_selection(|_| Some(path.clone()));
        cx.run_until_parked();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), second.raw);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[gpui_kit::test]
    fn scope_change_closes_details_and_back_returns_to_pods(cx: &mut TestAppContext) {
        let env = boot_logs(cx, &[entry("hello", None)]);
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            window.click(("log-row", 0usize), cx);
        })
        .unwrap();
        env.store.update(cx, |s, cx| s.set_namespace("other".into(), cx));
        cx.run_until_parked();
        let detail = env.app.read_with(cx, |a, cx| a.log_viewer.read(cx).detail.clone());
        assert!(detail.read_with(cx, |d, _| d.entry.is_none()));
        env.store.update(cx, |s, cx| {
            s.dismiss_error(cx);
            s.open_log_viewer("api-123".into(), cx);
        });
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            window.click("log-back", cx);
        })
        .unwrap();
        assert_eq!(env.store.read_with(cx, |s, _| s.view), crate::app::store::View::Pods);
    }
}
