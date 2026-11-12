//! `DeploymentDetails.tsx`: drawer body. Annotation and condition texts are selectable.
use gpui_kit::component::text::{TextView, TextViewStyle};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::store::Store;
use crate::format::format_timestamp;
use crate::status::health;
use crate::ui::badge::{dot, health_color, label_badge};
use crate::ui::collapsible::collapsible_header;
use crate::ui::section_label;
use crate::ui::theme::{Tw, hex, pal, tw};

/// Markdown-escapes plain text so `TextView` shows it verbatim (hard line breaks kept).
pub fn escape_md(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.split('\n') {
        let indent = line.len() - line.trim_start_matches(' ').len();
        out.push_str(&"\u{a0}".repeat(indent)); // leading spaces would start a code block
        for c in line[indent..].chars() {
            if c.is_ascii_punctuation() {
                out.push('\\');
            }
            out.push(c);
        }
        out.push_str("\\\n");
    }
    out.truncate(out.len() - 2);
    out
}

/// Mouse-selectable text (gpui-component `TextView` with `selectable(true)`).
pub fn selectable_text(id: impl Into<ElementId>, text: &str, color: Hsla) -> impl IntoElement {
    TextView::markdown(id, escape_md(text))
        .selectable(true)
        .text_color(color)
        .style(TextViewStyle::default().paragraph_gap(rems(0.)))
}

pub struct DeploymentDetailsView {
    store: Entity<Store>,
    annotations_open: bool,
    conditions_open: bool,
    _sub: Subscription,
}

impl DeploymentDetailsView {
    pub fn new(store: Entity<Store>, _: &mut Window, cx: &mut Context<Self>) -> Self {
        let sub = cx.observe(&store, |_, _, cx| cx.notify());
        Self { store, annotations_open: false, conditions_open: true, _sub: sub }
    }
}

fn info_row(label: &'static str, value: impl IntoElement, last: bool, cx: &App) -> Div {
    let p = pal(cx);
    div()
        .flex()
        .items_center()
        .justify_between()
        .py(px(8.))
        .when(!last, |d| d.border_b_1().border_color(p.border_subtle))
        .child(div().t_sm().text_color(p.text2).child(label))
        .child(value)
}

fn section(title: &'static str, body: impl IntoElement, cx: &App) -> Div {
    div().child(div().mb(px(12.)).child(section_label(title, cx))).child(body)
}

impl Render for DeploymentDetailsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = pal(cx);
        let Some(d) = self.store.read(cx).deployment_details.data().cloned() else {
            return div().into_any_element();
        };
        let h = health(d.replicas, d.available_replicas);
        let color = health_color(h);
        let num = |n: i32| div().t_sm().text_color(p.text2).child(n.to_string());
        let badges = |m: &std::collections::BTreeMap<String, String>, empty: &'static str| {
            div()
                .flex()
                .flex_wrap()
                .gap(px(8.))
                .children(m.iter().map(|(k, v)| label_badge(format!("{k}={v}"), cx)))
                .when(m.is_empty(), |d| d.child(div().t_sm().text_color(p.muted).child(empty)))
        };
        let this = cx.entity();
        let this2 = this.clone();

        let overview = section(
            "OVERVIEW",
            div()
                .child(info_row(
                    "Status",
                    div().flex().items_center().gap(px(8.)).child(dot(color, 8.)).child(
                        div().t_sm().font_weight(FontWeight::MEDIUM).text_color(color).child(h.label()),
                    ),
                    false,
                    cx,
                ))
                .child(info_row("Replicas", num(d.replicas), false, cx))
                .child(info_row("Available", div().t_sm().text_color(color).child(d.available_replicas.to_string()), false, cx))
                .child(info_row("Ready", num(d.ready_replicas), false, cx))
                .child(info_row("Updated", num(d.updated_replicas), false, cx))
                .child(info_row("Created", div().t_sm().child(format_timestamp(d.creation_timestamp.as_deref())), true, cx)),
            cx,
        );
        let strategy = section(
            "STRATEGY",
            div()
                .child(info_row("Type", div().t_sm().text_color(p.text2).child(d.strategy.clone()), false, cx))
                .child(info_row("Min Ready Seconds", num(d.min_ready_seconds), false, cx))
                .child(info_row("Revision History", num(d.revision_history_limit), true, cx)),
            cx,
        );

        let annotations = (!d.annotations.is_empty()).then(|| {
            let n = d.annotations.len();
            let last = n - 1;
            div()
                .child(
                    div().mb(px(12.)).child(collapsible_header(
                        "ann-toggle",
                        &format!("Annotations ({n})"),
                        self.annotations_open,
                        move |_, cx| this.update(cx, |v, cx| {
                            v.annotations_open = !v.annotations_open;
                            cx.notify();
                        }),
                        cx,
                    )),
                )
                .when(self.annotations_open, |el| {
                    el.child(div().flex().flex_col().gap(px(8.)).children(d.annotations.iter().enumerate().map(|(i, (k, v))| {
                        div()
                            .py(px(8.))
                            .when(i != last, |d| d.border_b_1().border_color(p.border_subtle))
                            .child(div().mb(px(4.)).t_xs().mono().text_color(p.muted).child(k.clone()))
                            .child(div().t_sm().child(selectable_text(("ann", i), v, p.text2)))
                    })))
                })
        });

        let conditions = (!d.conditions.is_empty()).then(|| {
            let n = d.conditions.len();
            div()
                .child(
                    div().mb(px(12.)).child(collapsible_header(
                        "cond-toggle",
                        &format!("Conditions ({n})"),
                        self.conditions_open,
                        move |_, cx| this2.update(cx, |v, cx| {
                            v.conditions_open = !v.conditions_open;
                            cx.notify();
                        }),
                        cx,
                    )),
                )
                .when(self.conditions_open, |el| {
                    el.child(div().flex().flex_col().gap(px(12.)).children(d.conditions.iter().enumerate().map(|(i, c)| {
                        div()
                            .py(px(8.))
                            .when(i != n - 1, |d| d.border_b_1().border_color(p.border_subtle))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .mb(px(4.))
                                    .child(div().t_sm().font_weight(FontWeight::MEDIUM).text_color(p.text1).child(c.condition_type.clone()))
                                    .child(
                                        div()
                                            .t_sm()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(if c.status == "True" { hex(tw::EMERALD_500) } else { p.muted })
                                            .child(c.status.clone()),
                                    ),
                            )
                            .children(c.reason.as_ref().map(|r| div().t_xs().text_color(p.muted).child(format!("Reason: {r}"))))
                            .children(c.message.as_ref().map(|m| {
                                div().mt(px(4.)).t_xs().child(selectable_text(("cond", i), m, p.text2))
                            }))
                    })))
                })
        });

        div()
            .p(px(16.))
            .flex()
            .flex_col()
            .gap(px(24.))
            .child(overview)
            .child(strategy)
            .child(section("SELECTOR", badges(&d.selector, "No selectors"), cx))
            .child(section("LABELS", badges(&d.labels, "No labels"), cx))
            .children(annotations)
            .children(conditions)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::escape_md;

    #[test]
    fn escapes_markdown_and_keeps_newlines() {
        assert_eq!(escape_md("a*b_c"), "a\\*b\\_c");
        assert_eq!(escape_md("x\ny"), "x\\\n\\\ny".replace("\\\n\\\n", "\\\n"));
        assert_eq!(escape_md("  {\"k\"}"), "\u{a0}\u{a0}\\{\\\"k\\\"\\}");
    }
}
