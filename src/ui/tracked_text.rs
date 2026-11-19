//! Single-line text with CSS `letter-spacing` (Tailwind `tracking-wider` = 0.05em).
//! gpui has no letter-spacing, so we shape the line ourselves and paint each glyph
//! shifted by `spacing * chars_before`, on the same baseline `ShapedLine::paint` uses.
use gpui_kit::*;

/// CSS adds the spacing after every character, including the last.
pub fn tracked_width(shaped: f32, chars: usize, spacing: f32) -> f32 {
    shaped + spacing * chars as f32
}

/// Extra x offset for the glyph that starts at `byte_index`.
pub fn glyph_offset(text: &str, byte_index: usize, spacing: f32) -> f32 {
    // ponytail: O(n) per glyph; labels are a few characters long.
    text.get(..byte_index).map_or(0, |s| s.chars().count()) as f32 * spacing
}

pub struct TrackedText {
    text: SharedString,
    spacing_em: f32,
}

/// Uses the inherited text style (font, size, weight, color, line height).
pub fn tracked(text: impl Into<SharedString>) -> TrackedText {
    TrackedText { text: text.into(), spacing_em: 0.05 }
}

impl TrackedText {
    pub fn spacing_em(mut self, em: f32) -> Self {
        self.spacing_em = em;
        self
    }
}

pub struct Layout {
    line: ShapedLine,
    line_height: Pixels,
    spacing: f32,
    color: Hsla,
}

impl IntoElement for TrackedText {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for TrackedText {
    type RequestLayoutState = Layout;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Layout) {
        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());
        let line_height = window.pixel_snap(style.line_height.to_pixels(font_size.into(), window.rem_size()));
        let run = style.to_run(self.text.len());
        let line = window.text_system().shape_line(self.text.clone(), font_size, &[run], None);
        let spacing = f32::from(font_size) * self.spacing_em;
        let width = tracked_width(f32::from(line.width), self.text.chars().count(), spacing);

        let mut layout_style = Style::default();
        layout_style.size.width = px(width).into();
        layout_style.size.height = line_height.into();
        layout_style.flex_shrink = 0.;
        let id = window.request_layout(layout_style, [], cx);
        (id, Layout { line, line_height, spacing, color: style.color })
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _layout: &mut Layout,
        _window: &mut Window,
        _cx: &mut App,
    ) {
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        l: &mut Layout,
        _: &mut (),
        window: &mut Window,
        _cx: &mut App,
    ) {
        // Same baseline as gpui's `paint_line` (text_system/line.rs).
        let padding_top = (l.line_height - l.line.ascent - l.line.descent) / 2.;
        let baseline_y = bounds.origin.y + padding_top + l.line.ascent;
        let font_size = l.line.font_size;
        for run in &l.line.runs {
            for glyph in &run.glyphs {
                let x = bounds.origin.x + glyph.position.x + px(glyph_offset(&self.text, glyph.index, l.spacing));
                let origin = point(x, baseline_y + glyph.position.y);
                let _ = if glyph.is_emoji {
                    window.paint_emoji(origin, run.font_id, glyph.id, font_size)
                } else {
                    window.paint_glyph(origin, run.font_id, glyph.id, font_size, l.color)
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{glyph_offset, tracked_width}; // not `*`: gpui's `test` macro would shadow #[test]

    #[test]
    fn spacing_math() {
        assert!((tracked_width(100.0, 8, 0.6) - 104.8).abs() < 1e-4);
        assert_eq!(glyph_offset("CLUSTER", 0, 0.6), 0.0);
        assert!((glyph_offset("CLUSTER", 3, 0.6) - 1.8).abs() < 1e-4);
        assert!((glyph_offset("ÉA", 2, 0.6) - 0.6).abs() < 1e-4); // byte index 2 = 1 char before
    }
}
