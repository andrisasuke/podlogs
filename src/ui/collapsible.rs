//! Collapsible section header: chevron (−90° when closed) + tracked uppercase xs label, hover → text2.
use gpui_kit::assets::IconName;
use gpui_kit::component::Icon;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::theme::{Tw, pal};
use super::tracked_text::tracked;

pub fn collapsible_header(
    id: impl Into<ElementId>,
    title: &str,
    open: bool,
    on_toggle: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let p = pal(cx);
    div()
        .id(id)
        .test_support()
        .flex()
        .items_center()
        .gap(px(8.))
        .cursor_pointer()
        .t_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(p.muted)
        .hover(move |s| s.text_color(p.text2))
        .child(Icon::new(IconName::ChevronDown).size_4().when(!open, |i| i.rotate(radians(-std::f32::consts::FRAC_PI_2))))
        .child(tracked(title.to_uppercase()))
        .on_click(move |_, w, cx| on_toggle(w, cx))
}
