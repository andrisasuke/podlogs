//! `Skeleton.tsx`: Tailwind `animate-pulse` blocks (2 s, opacity 1 → 0.5 → 1).
use std::time::Duration;

use gpui_kit::*;

use super::theme::{Tw, pal};

fn block(id: impl Into<ElementId>, h: f32, w: Option<f32>, radius: f32, cx: &App) -> AnyElement {
    let el = div().h(px(h)).flex_none().rounded(px(radius)).bg(pal(cx).bg3);
    let el = match w {
        Some(w) => el.w(px(w)),
        None => el.flex_1(),
    };
    el.with_animation(id.into(), Animation::new(Duration::from_secs(2)).repeat(), |el, t| {
        el.opacity(0.75 + 0.25 * (t * std::f32::consts::TAU).cos())
    })
    .into_any_element()
}

pub fn table_skeleton(rows: usize, cx: &App) -> Div {
    let cols: [(f32, f32, f32); 6] = [(16., 192., 4.), (24., 80., 12.), (16., 48., 4.), (16., 48., 4.), (16., 64., 4.), (16., 96., 4.)];
    div().flex().flex_col().gap(px(12.)).p(px(16.)).children((0..rows).map(|r| {
        let mut row = div().flex().items_center().gap(px(16.));
        for (c, (h, w, rad)) in cols.iter().enumerate() {
            row = row.child(block(("sk", r * 20 + c), *h, Some(*w), if *rad == 12. { 9999. } else { *rad }, cx));
        }
        row.child(div().flex_1())
            .child(block(("sk", r * 20 + 10), 32., Some(64.), 6., cx))
            .child(block(("sk", r * 20 + 11), 32., Some(64.), 6., cx))
    }))
}

pub fn log_skeleton(rows: usize, cx: &App) -> Div {
    div().flex().flex_col().gap(px(8.)).p(px(16.)).mono().t_sm().children((0..rows).map(|r| {
        div()
            .flex()
            .items_center()
            .gap(px(12.))
            .child(block(("lsk", r * 3), 16., Some(144.), 4., cx))
            .child(block(("lsk", r * 3 + 1), 20., Some(56.), 4., cx))
            .child(block(("lsk", r * 3 + 2), 16., None, 4., cx))
    }))
}
