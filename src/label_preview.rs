//! TEMPORARY (remove in Task 7): letter-spacing gate for Task 6.
//! `PODLOGS_LABEL_PREVIEW=1 cargo run` draws the three sidebar section labels at their
//! React positions (sidebar 260 px, bg-secondary, labels at x = 16 px).
//! `PODLOGS_LABEL_PREVIEW_THEME=light` switches to the light palette.
use gpui_kit::*;

use crate::ui::section_label;
use crate::ui::theme::{UI_FONT, pal};

pub fn enabled() -> bool {
    std::env::var("PODLOGS_LABEL_PREVIEW").is_ok_and(|v| v == "1")
}

pub fn dark() -> bool {
    !std::env::var("PODLOGS_LABEL_PREVIEW_THEME").is_ok_and(|v| v == "light")
}

/// Vertical metrics copied from `c70bd05:src/components/layout/Sidebar.tsx`; the dropdowns,
/// nav buttons and input are empty spacers so only the labels are drawn.
pub fn render(cx: &App) -> Div {
    let p = pal(cx);
    let label_block = |text: &'static str, below: f32| {
        div().flex().flex_col().child(section_label(text, cx)).child(div().h(px(8. + below)))
    };
    let sidebar = div()
        .w(px(260.))
        .h_full()
        .flex()
        .flex_col()
        .bg(p.bg2)
        .border_r_1()
        .border_color(p.border)
        // header h-14 border-b
        .child(div().h(px(56.)).border_b_1().border_color(p.border))
        // px-4 py-4 space-y-4 border-b: label + mb-2 + dropdown (h 36), twice
        .child(
            div()
                .px(px(16.))
                .py(px(16.))
                .flex()
                .flex_col()
                .gap(px(16.))
                .border_b_1()
                .border_color(p.border)
                .child(label_block("CLUSTER", 36.))
                .child(label_block("NAMESPACE", 36.)),
        )
        // nav px-2 py-3 border-b: two 36 px buttons
        .child(div().h(px(12. + 36. + 36. + 12.)).border_b_1().border_color(p.border))
        // px-4 pt-4: deployments label
        .child(div().px(px(16.)).pt(px(16.)).child(label_block("DEPLOYMENTS", 36.)));
    div()
        .size_full()
        .flex()
        .font_family(UI_FONT)
        .text_size(px(14.))
        .line_height(relative(1.5))
        .text_color(p.text1)
        .bg(p.bg1)
        .child(sidebar)
}
