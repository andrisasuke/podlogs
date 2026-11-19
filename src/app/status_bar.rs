//! `StatusBar.tsx`: 28 px bar with Live/Syncing on the left, pod count + refresh countdown on the right.
use std::time::Duration;

use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::store::{Store, View};
use crate::ui::badge::dot;
use crate::ui::theme::{Tw, hex, pal, tw};

pub const STATUS_BAR_HEIGHT: f32 = 28.;

pub fn status_bar(store: &Store, cx: &App) -> AnyElement {
    let p = pal(cx);
    let fetching = store.pods.is_fetching();
    let pods = store.pods.data().map_or(0, Vec::len);
    let sync = if fetching {
        div()
            .id("status-syncing")
            .test_support()
            .flex()
            .items_center()
            .gap(px(6.))
            .child(dot(hex(tw::AMBER_500), 8.).with_animation("status-pulse", Animation::new(Duration::from_secs(2)).repeat(), |el, t| {
                el.opacity(0.75 + 0.25 * (t * std::f32::consts::TAU).cos()) // Tailwind animate-pulse
            }))
            .child("Syncing...")
    } else {
        div()
            .id("status-live")
            .test_support()
            .flex()
            .items_center()
            .gap(px(6.))
            .child(dot(hex(tw::EMERALD_500), 8.))
            .child("Live")
    };
    div()
        .flex()
        .flex_none()
        .items_center()
        .justify_between()
        .h(px(STATUS_BAR_HEIGHT))
        .px(px(16.))
        .t_xs()
        .text_color(p.muted)
        .border_t_1()
        .border_color(p.border)
        .bg(p.bg2)
        .child(div().flex().items_center().gap(px(12.)).when(!store.context.is_empty(), |d| d.child(sync)))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(16.))
                .when(store.view == View::Pods, |d| {
                    d.child(
                        div()
                            .id("status-pods")
                            .test_support()
                            .child(format!("Showing {pods} pod{}", if pods == 1 { "" } else { "s" })),
                    )
                })
                .when(!fetching, |d| {
                    d.child(div().id("status-countdown").test_support().child(format!("Refresh in {}s", store.countdown_s)))
                }),
        )
        .into_any_element()
}
