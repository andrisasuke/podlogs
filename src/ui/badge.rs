//! `Badge.tsx`: status, log level and label badges.
use gpui_kit::*;

use super::theme::{Tw, hex, hexa, pal, tw};
use crate::status::{BadgeVariant, pod_status_variant};

pub fn dot(color: Hsla, size_px: f32) -> Div {
    div().size(px(size_px)).flex_none().rounded_full().bg(color)
}

/// `(dot/base, text light, text dark)` for a status variant.
fn variant_colors(v: BadgeVariant) -> (u32, u32, u32) {
    match v {
        BadgeVariant::Success => (tw::EMERALD_500, tw::EMERALD_600, tw::EMERALD_400),
        BadgeVariant::Warning => (tw::AMBER_500, tw::AMBER_600, tw::AMBER_400),
        BadgeVariant::Error => (tw::RED_500, tw::RED_600, tw::RED_400),
        BadgeVariant::Terminating => (tw::PURPLE_500, tw::PURPLE_600, tw::PURPLE_400),
        BadgeVariant::Default => (tw::GRAY_500, tw::GRAY_500, tw::GRAY_400),
    }
}

pub fn status_badge(status: &str, ready: Option<&str>, cx: &App) -> Div {
    let (base, light, dark) = variant_colors(pod_status_variant(status, ready));
    div()
        .flex()
        .flex_none()
        .items_center()
        .gap(px(6.))
        .px(px(10.))
        .py(px(4.))
        .rounded_full()
        .t_xs()
        .font_weight(FontWeight::MEDIUM)
        .bg(hexa(base, 0.1))
        .text_color(hex(if pal(cx).dark { dark } else { light }))
        .child(dot(hex(base), 6.))
        .child(status.to_string())
}

pub fn level_badge(level: &str, cx: &App) -> Div {
    let bg = match level {
        "ERROR" if pal(cx).dark => tw::RED_600,
        "ERROR" => tw::RED_500,
        "WARN" => tw::AMBER_500,
        "INFO" => tw::BLUE_500,
        _ => tw::PURPLE_500, // DEBUG
    };
    div()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .min_w(px(52.))
        .px(px(8.))
        .py(px(2.))
        .rounded(px(4.))
        .t_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(white())
        .bg(hex(bg))
        .child(level.to_string())
}

pub fn label_badge(text: impl Into<SharedString>, cx: &App) -> Div {
    let p = pal(cx);
    div()
        .flex()
        .flex_none()
        .items_center()
        .px(px(8.))
        .py(px(4.))
        .rounded(px(4.))
        .t_xs()
        .mono()
        .bg(p.bg3)
        .text_color(p.text2)
        .child(text.into())
}
