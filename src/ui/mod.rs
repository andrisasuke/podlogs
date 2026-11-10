pub mod theme;
pub mod tracked_text;

use gpui_kit::*;

use theme::{Tw, pal};
use tracked_text::tracked;

/// `text-xs font-medium text-text-muted uppercase tracking-wider`. Callers pass the uppercase string.
pub fn section_label(text: impl Into<SharedString>, cx: &App) -> impl IntoElement {
    div()
        .t_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(pal(cx).muted)
        .child(tracked(text))
}
