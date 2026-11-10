pub mod badge;
pub mod button;
pub mod collapsible;
pub mod dropdown;
pub mod drawer;
pub mod icons;
pub mod input;
pub mod modal;
pub mod skeleton;
pub mod theme;
pub mod tracked_text;

#[cfg(test)]
mod tests;

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

/// Key bindings shared by the overlays (Escape closes a dropdown, modal or drawer).
pub fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("escape", modal::CloseOverlay, Some(modal::OVERLAY_CONTEXT))]);
}
