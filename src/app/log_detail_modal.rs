//! Selectable log metadata and content, with native clipboard actions.
use std::{rc::Rc, time::Duration};

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    Icon,
    text::{TextView, TextViewStyle},
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use crate::format::format_timestamp;
use crate::models::LogEntry;
use crate::ui::badge::level_badge;
use crate::ui::modal::{ModalSize, OnClose, modal};
use crate::ui::selectable_text;
use crate::ui::theme::{Tw, hex, pal, tw};

pub struct LogDetailModal {
    pub(crate) entry: Option<LogEntry>,
    copied: Option<u8>,
    epoch: usize,
    copy_generation: u64,
    json: String,
}

pub fn formatted_json(entry: &LogEntry) -> String {
    entry
        .raw
        .find('{')
        .and_then(|start| serde_json::from_str::<serde_json::Value>(&entry.raw[start..]).ok())
        .and_then(|value| serde_json::to_string_pretty(&value).ok())
        .unwrap_or_else(|| entry.raw.clone())
}

pub fn message_color(level: Option<&str>, dark: bool, fallback: Hsla) -> Hsla {
    match level {
        Some("ERROR") => hex(if dark { tw::RED_400 } else { tw::RED_500 }),
        Some("WARN") => hex(if dark { tw::AMBER_400 } else { tw::AMBER_600 }),
        _ => fallback,
    }
}

impl LogDetailModal {
    pub fn new() -> Self {
        Self { entry: None, copied: None, epoch: 0, copy_generation: 0, json: String::new() }
    }

    pub fn open(&mut self, entry: LogEntry, cx: &mut Context<Self>) {
        self.json = if entry.is_json { formatted_json(&entry) } else { String::new() };
        self.entry = Some(entry);
        self.copied = None;
        self.epoch += 1;
        self.copy_generation += 1;
        cx.notify();
    }

    pub fn close(&mut self, cx: &mut Context<Self>) {
        self.entry = None;
        self.copied = None;
        self.json.clear();
        self.copy_generation += 1;
        cx.notify();
    }

    fn copy(&mut self, part: u8, cx: &mut Context<Self>) {
        let Some(entry) = &self.entry else { return };
        let text = match part {
            0 => &entry.message,
            1 => &entry.raw,
            _ => &self.json,
        };
        cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
        self.copied = Some(part);
        self.copy_generation += 1;
        let generation = self.copy_generation;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(2)).await;
            let _ = this.update(cx, |this, cx| {
                if this.copy_generation == generation {
                    this.copied = None;
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    fn content_section(&self, part: u8, title: &'static str, text: &str, color: Hsla, cx: &mut Context<Self>) -> AnyElement {
        let p = pal(cx);
        let copied = self.copied == Some(part);
        let small = part != 0;
        let height = if part == 1 { 150. } else { 200. };
        let copy = div()
            .id(("log-copy", part as usize))
            .test_support()
            .h(px(32.))
            .px(px(12.))
            .flex()
            .items_center()
            .justify_center()
            .gap(px(4.))
            .rounded(px(6.))
            .t_sm()
            .font_weight(FontWeight::MEDIUM)
            .text_color(p.text2)
            .cursor_pointer()
            .hover(|s| s.bg(p.bg3))
            .child(
                Icon::new(if copied { IconName::Check } else { IconName::Copy })
                    .size_4()
                    .when(copied, |i| i.text_color(hex(tw::EMERALD_500))),
            )
            .child(if copied { "Copied" } else { "Copy" })
            .on_click(cx.listener(move |this, _, _, cx| this.copy(part, cx)));
        div()
            .child(
                div().flex().items_center().justify_between().mb(px(8.)).child(div().t_sm().text_color(p.muted).child(title)).child(copy),
            )
            .child(
                div()
                    .id(("log-content", part as usize))
                    .test_support()
                    .p(px(12.))
                    .rounded(px(6.))
                    .bg(if small { p.bg2 } else { p.bg3 })
                    .when(small, |d| d.border_1().border_color(p.border))
                    .mono()
                    .text_color(color)
                    .max_h(px(height))
                    .overflow_y_scroll()
                    .when(small, |d| d.t_xs())
                    .when(!small, |d| d.t_sm())
                    .child(literal_block(format!("log-text-{part}"), text, small)),
            )
            .into_any_element()
    }
}

/// A styled preformatted text block preserves indentation, line breaks and literal markup.
fn literal_block(id: String, text: &str, small: bool) -> TextView {
    let escaped = text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    let code = StyleRefinement::default()
        .p_0()
        .border_0()
        .bg(hsla(0., 0., 0., 0.))
        .whitespace_normal()
        .text_size(px(if small { 12. } else { 14. }))
        .line_height(px(if small { 16. } else { 20. }));
    TextView::html(SharedString::from(id), format!("<pre>{escaped}</pre>"))
        .selectable(true)
        .style(TextViewStyle::default().paragraph_gap(rems(0.)).code_block(code))
}

fn metadata(id: &'static str, label: &'static str, text: &str, cx: &App) -> Div {
    let p = pal(cx);
    div()
        .flex_1()
        .min_w_0()
        .t_sm()
        .child(div().text_color(p.muted).child(label))
        .child(div().mt(px(4.)).mono().text_color(p.text1).child(selectable_text(id, text)))
}

impl Render for LogDetailModal {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(entry) = self.entry.clone() else { return div().into_any_element() };
        let p = pal(cx);
        let details = div()
            .flex()
            .flex_col()
            .gap(px(16.))
            .child(
                div()
                    .flex()
                    .gap(px(16.))
                    .child(metadata("log-timestamp", "Timestamp", &format_timestamp(entry.timestamp.as_deref()), cx))
                    .child(div().flex_1().min_w_0().t_sm().child(div().text_color(p.muted).child("Level")).child(
                        div().mt(px(4.)).flex().when_some(entry.level.as_deref().filter(|level| !level.is_empty()), |d, level| {
                            d.child(level_badge(level, cx))
                        }),
                    )),
            )
            .child(div().flex().gap(px(16.)).child(metadata("log-pod", "Pod", &entry.pod_name, cx)).child(metadata(
                "log-container",
                "Container",
                &entry.container_name,
                cx,
            )));
        let mut body = div()
            .id("log-detail-modal")
            .test_support()
            .p(px(16.))
            .flex()
            .flex_col()
            .gap(px(16.))
            .child(details)
            .child(self.content_section(0, "Message", &entry.message, message_color(entry.level.as_deref(), p.dark, p.text1), cx))
            .child(self.content_section(1, "Raw Log", &entry.raw, p.text2, cx));
        if entry.is_json {
            body = body.child(self.content_section(2, "JSON (Formatted)", &self.json, p.text2, cx));
        }
        let entity = cx.entity();
        let close: OnClose = Rc::new(move |_, cx| entity.update(cx, |this, cx| this.close(cx)));
        modal(("log-detail", self.epoch), "Log Details", ModalSize::Lg, false, close, body.into_any_element(), window, cx)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{AppContext, TestAppContext};

    use super::{LogDetailModal, formatted_json};
    use crate::models::LogEntry;

    fn entry() -> LogEntry {
        LogEntry {
            timestamp: None,
            level: Some("INFO".into()),
            message: "hello <literal>".into(),
            raw: "timestamp {\"message\":\"hello <literal>\"}".into(),
            is_json: true,
            pod_name: "api".into(),
            container_name: "app".into(),
        }
    }

    #[test]
    fn pretty_json_uses_first_brace_and_falls_back_to_raw() {
        let mut log = entry();
        assert_eq!(formatted_json(&log), "{\n  \"message\": \"hello <literal>\"\n}");
        log.raw = "{broken".into();
        assert_eq!(formatted_json(&log), log.raw);
        log.raw = "not json".into();
        assert_eq!(formatted_json(&log), log.raw);
    }

    #[gpui_kit::test]
    fn copy_writes_exact_text_and_resets_after_two_seconds(cx: &mut TestAppContext) {
        let (handle, detail) = cx.update(|cx| {
            gpui_kit::init(cx);
            crate::ui::init(cx);
            crate::ui::theme::set_palette(crate::ui::theme::palette(true), cx);
            gpui_kit::open_window(gpui_kit::WindowOptions::default(), cx, |_, cx| {
                cx.new(|cx| {
                    let mut modal = LogDetailModal::new();
                    modal.open(entry(), cx);
                    modal
                })
            })
            .unwrap()
        });
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            window.click(("log-copy", 0usize), cx);
        })
        .unwrap();
        assert_eq!(cx.read_from_clipboard().unwrap().text().as_deref(), Some("hello <literal>"));
        assert_eq!(detail.read_with(cx, |d, _| d.copied), Some(0));
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_secs(2));
        cx.run_until_parked();
        assert_eq!(detail.read_with(cx, |d, _| d.copied), None);
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            window.click(("log-copy", 1usize), cx);
        })
        .unwrap();
        assert_eq!(cx.read_from_clipboard().unwrap().text(), Some(entry().raw));
    }
}
