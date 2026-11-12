//! Wrapped, clamped text with a genuinely padded inline mark.
use gpui_kit::*;
use std::{cell::RefCell, ops::Range, rc::Rc};

const PAD: f32 = 2.;

pub struct MarkText {
    text: SharedString,
    mark: Option<Range<usize>>,
    color: Hsla,
    mark_color: Hsla,
    mark_bg: Hsla,
}

pub fn mark_text(
    text: SharedString,
    mark: Option<Range<usize>>,
    color: Hsla,
    mark_color: Hsla,
    mark_bg: Hsla,
) -> MarkText {
    let text = text.replace(['\n', '\r', '\t'], " ").into();
    MarkText {
        text,
        mark,
        color,
        mark_color,
        mark_bg,
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Row {
    range: Range<usize>,
    left_pad: bool,
    right_pad: bool,
    ellipsis: bool,
    // x coordinates of the three segments, relative to the row.
    before_x: f32,
    mark_x: f32,
    after_x: f32,
    width: f32,
}

fn virtual_to_original(ix: usize, mark: Option<&Range<usize>>) -> usize {
    if let Some(mark) = mark {
        ix - usize::from(ix > mark.start) - usize::from(ix > mark.end + 1)
    } else {
        ix
    }
}

fn segment_ranges(row: &Row, mark: Option<&Range<usize>>) -> [Range<usize>; 3] {
    let start = row.range.start;
    let end = row.range.end;
    let (m0, m1) = mark
        .map(|m| (m.start.clamp(start, end), m.end.clamp(start, end)))
        .unwrap_or((end, end));
    [start..m0, m0..m1, m1..end]
}

fn measure_row(
    row: &mut Row,
    text: &str,
    mark: Option<&Range<usize>>,
    measure: &impl Fn(&str) -> f32,
    ellipsis: &str,
) {
    let parts = segment_ranges(row, mark);
    row.before_x = 0.;
    row.mark_x = measure(&text[parts[0].clone()]) + if row.left_pad { PAD } else { 0. };
    row.after_x =
        row.mark_x + measure(&text[parts[1].clone()]) + if row.right_pad { PAD } else { 0. };
    row.width = row.after_x
        + measure(&text[parts[2].clone()])
        + if row.ellipsis { measure(ellipsis) } else { 0. };
}

/// `starts` are gpui's virtual byte boundaries, including one byte for each pad.
fn layout_rows(
    text: &str,
    mark: Option<Range<usize>>,
    starts: &[usize],
    measure: impl Fn(&str) -> f32,
    wrap_width: f32,
    ellipsis: &str,
) -> Vec<Row> {
    let virtual_len = text.len() + if mark.is_some() { 2 } else { 0 };
    let mut rows: Vec<Row> = starts
        .iter()
        .take(2)
        .enumerate()
        .map(|(i, &start)| {
            let end = starts.get(i + 1).copied().unwrap_or(virtual_len);
            let left_pad = mark
                .as_ref()
                .is_some_and(|m| start <= m.start && m.start < end);
            let right_pad = mark
                .as_ref()
                .is_some_and(|m| start <= m.end + 1 && m.end + 1 < end);
            let mut row = Row {
                range: virtual_to_original(start, mark.as_ref())
                    ..virtual_to_original(end, mark.as_ref()),
                left_pad,
                right_pad,
                ellipsis: false,
                before_x: 0.,
                mark_x: 0.,
                after_x: 0.,
                width: 0.,
            };
            measure_row(&mut row, text, mark.as_ref(), &measure, ellipsis);
            row
        })
        .collect();
    if starts.len() > 2 {
        let row = &mut rows[1];
        let mut best = row.clone();
        best.range.end = best.range.start;
        best.right_pad = false;
        best.ellipsis = true;
        best.left_pad = false;
        measure_row(&mut best, text, mark.as_ref(), &measure, ellipsis);
        // Widths are monotonic, so inspect only logarithmically many prefixes.
        let cuts: Vec<usize> = std::iter::once(row.range.start)
            .chain(
                text[row.range.clone()]
                    .char_indices()
                    .map(|(i, c)| row.range.start + i + c.len_utf8()),
            )
            .collect();
        let (mut low, mut high) = (0, cuts.len());
        while low < high {
            let mid = low + (high - low) / 2;
            let cut = cuts[mid];
            let mut candidate = row.clone();
            candidate.range.end = cut;
            candidate.right_pad &= mark.as_ref().is_some_and(|m| cut >= m.end);
            candidate.left_pad &= mark.as_ref().is_some_and(|m| cut > m.start);
            candidate.ellipsis = true;
            measure_row(&mut candidate, text, mark.as_ref(), &measure, ellipsis);
            if candidate.width <= wrap_width {
                best = candidate;
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        *row = best;
    }
    rows
}

struct PaintedRow {
    row: Row,
    pieces: [ShapedLine; 3],
    ellipsis: Option<ShapedLine>,
}
pub struct Layout {
    rows: Vec<PaintedRow>,
    line_height: Pixels,
    content_height: Pixels,
}
type SharedLayout = Rc<RefCell<Option<Layout>>>;

impl IntoElement for MarkText {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for MarkText {
    type RequestLayoutState = SharedLayout;
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        _: &mut App,
    ) -> (LayoutId, SharedLayout) {
        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());
        let line_height = window.pixel_snap(
            style
                .line_height
                .to_pixels(font_size.into(), window.rem_size()),
        );
        let font = style.font();
        let text = self.text.clone();
        let mark = self.mark.clone();
        let color = self.color;
        let mark_color = self.mark_color;
        let layout: SharedLayout = Rc::new(RefCell::new(None));
        let output = layout.clone();
        let id = window.request_measured_layout(
            Style::default(),
            move |known, available, window, cx| {
                let width = known.width.or(match available.width {
                    AvailableSpace::Definite(w) => Some(w),
                    _ => None,
                });
                let mut starts = vec![0];
                if let Some(w) = width {
                    let mut wrapper = cx.text_system().line_wrapper(font.clone(), font_size);
                    let fragments = if let Some(m) = &mark {
                        vec![
                            LineFragment::text(&text[..m.start]),
                            LineFragment::element(px(PAD), 1),
                            LineFragment::text(&text[m.clone()]),
                            LineFragment::element(px(PAD), 1),
                            LineFragment::text(&text[m.end..]),
                        ]
                    } else {
                        vec![LineFragment::text(&text)]
                    };
                    starts.extend(
                        wrapper
                            .wrap_line(&fragments, w, IndentAdjustment::NoIndent)
                            .take(2)
                            .map(|b| b.ix),
                    );
                }
                let shape = |s: &str, c: Hsla| {
                    let mut run = style.to_run(s.len());
                    run.color = c;
                    window
                        .text_system()
                        .shape_line(s.to_owned().into(), font_size, &[run], None)
                };
                let rows = layout_rows(
                    &text,
                    mark.clone(),
                    &starts,
                    |s| f32::from(shape(s, color).width()),
                    width.map(f32::from).unwrap_or(f32::INFINITY),
                    "…",
                );
                let content_width = rows.iter().map(|r| r.width).fold(0_f32, f32::max);
                let painted = rows
                    .into_iter()
                    .map(|row| {
                        let parts = segment_ranges(&row, mark.as_ref());
                        let pieces = [
                            shape(&text[parts[0].clone()], color),
                            shape(&text[parts[1].clone()], mark_color),
                            shape(&text[parts[2].clone()], color),
                        ];
                        let ellipsis = row.ellipsis.then(|| shape("…", color));
                        PaintedRow {
                            row,
                            pieces,
                            ellipsis,
                        }
                    })
                    .collect::<Vec<_>>();
                let height = line_height * painted.len() as f32;
                let font_id = cx.text_system().resolve_font(&font);
                let content_height = cx.text_system().ascent(font_id, font_size)
                    + cx.text_system().descent(font_id, font_size);
                output.replace(Some(Layout {
                    rows: painted,
                    line_height,
                    content_height,
                }));
                size(width.unwrap_or(px(content_width)), height)
            },
        );
        (id, layout)
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut SharedLayout,
        _: &mut Window,
        _: &mut App,
    ) {
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut SharedLayout,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let layout = state.borrow();
        let Some(layout) = layout.as_ref() else {
            return;
        };
        for (i, painted) in layout.rows.iter().enumerate() {
            let row = &painted.row;
            let y = bounds.origin.y + layout.line_height * i as f32;
            let origin = |x: f32| point(bounds.origin.x + px(x), y);
            let mark_width = painted.pieces[1].width();
            if mark_width > px(0.) {
                let x0 = bounds.origin.x + px(row.mark_x - if row.left_pad { PAD } else { 0. });
                let x1 = bounds.origin.x + px(row.after_x);
                let top = y + (layout.line_height - layout.content_height) / 2.;
                let radius = |on| if on { px(4.) } else { px(0.) };
                window.paint_quad(
                    fill(
                        Bounds::from_corners(
                            point(x0, top),
                            point(x1, top + layout.content_height),
                        ),
                        self.mark_bg,
                    )
                    .corner_radii(Corners {
                        top_left: radius(row.left_pad),
                        bottom_left: radius(row.left_pad),
                        top_right: radius(row.right_pad),
                        bottom_right: radius(row.right_pad),
                    }),
                );
            }
            for (piece, x) in painted
                .pieces
                .iter()
                .zip([row.before_x, row.mark_x, row.after_x])
            {
                let _ = piece.paint(
                    origin(x),
                    layout.line_height,
                    TextAlign::Left,
                    None,
                    window,
                    cx,
                );
            }
            if let Some(ellipsis) = &painted.ellipsis {
                let _ = ellipsis.paint(
                    origin(row.width - f32::from(ellipsis.width())),
                    layout.line_height,
                    TextAlign::Left,
                    None,
                    window,
                    cx,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Row, layout_rows, measure_row, virtual_to_original};
    use std::{cell::Cell, ops::Range};

    // The original implementation, retained only as a comparison oracle.
    fn linear_rows(
        text: &str,
        mark: Option<Range<usize>>,
        starts: &[usize],
        measure: impl Fn(&str) -> f32,
        wrap_width: f32,
    ) -> Vec<Row> {
        let virtual_len = text.len() + if mark.is_some() { 2 } else { 0 };
        let mut rows: Vec<Row> = starts
            .iter()
            .enumerate()
            .map(|(i, &start)| {
                let end = starts.get(i + 1).copied().unwrap_or(virtual_len);
                let mut row = Row {
                    range: virtual_to_original(start, mark.as_ref())
                        ..virtual_to_original(end, mark.as_ref()),
                    left_pad: mark
                        .as_ref()
                        .is_some_and(|m| start <= m.start && m.start < end),
                    right_pad: mark
                        .as_ref()
                        .is_some_and(|m| start <= m.end + 1 && m.end + 1 < end),
                    ellipsis: false,
                    before_x: 0.,
                    mark_x: 0.,
                    after_x: 0.,
                    width: 0.,
                };
                measure_row(&mut row, text, mark.as_ref(), &measure, "…");
                row
            })
            .collect();
        if rows.len() > 2 {
            rows.truncate(2);
            let row = &mut rows[1];
            let mut best = row.clone();
            best.range.end = best.range.start;
            best.right_pad = false;
            best.ellipsis = true;
            best.left_pad = false;
            measure_row(&mut best, text, mark.as_ref(), &measure, "…");
            for cut in std::iter::once(row.range.start).chain(
                text[row.range.clone()]
                    .char_indices()
                    .map(|(i, c)| row.range.start + i + c.len_utf8()),
            ) {
                let mut candidate = row.clone();
                candidate.range.end = cut;
                candidate.right_pad &= mark.as_ref().is_some_and(|m| cut >= m.end);
                candidate.left_pad &= mark.as_ref().is_some_and(|m| cut > m.start);
                candidate.ellipsis = true;
                measure_row(&mut candidate, text, mark.as_ref(), &measure, "…");
                if candidate.width <= wrap_width {
                    best = candidate;
                }
            }
            *row = best;
        }
        rows
    }

    fn width(s: &str) -> f32 {
        s.chars().count() as f32 * 8.4
    }
    fn close(a: f32, b: f32) {
        assert!((a - b).abs() < 0.001, "{a} != {b}");
    }

    #[test]
    fn text_after_mark_shifts_by_four_px() {
        let rows = layout_rows("abCDef", Some(2..4), &[0], width, 100., "…");
        let row = &rows[0];
        close(row.before_x, 0.);
        close(row.mark_x, 2. * 8.4 + 2.);
        close(row.after_x, 4. * 8.4 + 4.);
        let plain = layout_rows("abCDef", None, &[0], width, 100., "…");
        close(row.after_x - width("abcd"), 4.);
        close(plain[0].width + 4., row.width);
    }

    #[test]
    fn pads_count_toward_wrap() {
        // "ab cd" is 42px; adding the pads makes it exceed a 44px line.
        let rows = layout_rows("ab cd", Some(0..2), &[0, 5], width, 44., "…");
        assert!(width("ab cd") <= 44. && width("ab cd") + 4. > 44.);
        assert_eq!(virtual_to_original(5, Some(&(0..2))), 3);
        assert_eq!(rows[0].range, 0..3);
        assert_eq!(rows[1].range, 3..5);
    }

    #[test]
    fn mark_split_across_rows_pads_outer_ends_only() {
        let rows = layout_rows("abcdef", Some(1..5), &[0, 4], width, 100., "…");
        assert_eq!(rows[0].range, 0..3);
        assert!(rows[0].left_pad && !rows[0].right_pad);
        assert!(!rows[1].left_pad && rows[1].right_pad);
    }

    #[test]
    fn clamp_keeps_two_rows_and_ellipsis_fits() {
        let rows = layout_rows("abcdefghijklmnop", None, &[0, 5, 10], width, 34., "…");
        assert_eq!(rows.len(), 2);
        assert!(rows[1].ellipsis);
        assert!(rows[1].width <= 34.);
        assert_eq!(&"abcdefghijklmnop"[rows[1].range.clone()], "fgh");
    }

    #[test]
    fn clamp_cutting_mark_drops_right_pad() {
        let rows = layout_rows(
            "abcdefghijklmnop",
            Some(2..11),
            &[0, 6, 13],
            width,
            27.,
            "…",
        );
        assert_eq!(rows.len(), 2);
        assert!(rows[1].ellipsis);
        assert!(rows[1].range.end < 11);
        assert!(!rows[1].left_pad && !rows[1].right_pad);
        assert!(rows[1].width <= 27.);
    }
    #[test]
    fn bounded_measurements_match_linear_reference() {
        let text = "a".repeat(5000);
        let mark = Some(100..150);
        let starts: Vec<_> = (0..5002).step_by(84).collect();
        let old_count = Cell::new(0);
        let old = linear_rows(
            &text,
            mark.clone(),
            &starts,
            |s| {
                old_count.set(old_count.get() + 1);
                width(s)
            },
            710.,
        );
        let new_count = Cell::new(0);
        let new = layout_rows(
            &text,
            mark,
            &starts[..3],
            |s| {
                new_count.set(new_count.get() + 1);
                width(s)
            },
            710.,
            "…",
        );
        assert_eq!(new, old);
        assert!(new_count.get() <= 64, "{} measurements", new_count.get());
        println!(
            "measure calls: before {}, after {}",
            old_count.get(),
            new_count.get()
        );
    }

    #[test]
    fn third_start_bounds_second_row_and_binary_search_matches_linear() {
        let text = "abcdefghijklmno";
        let starts = [0, 4, 8, 12];
        for mark in [None, Some(4..7), Some(2..10)] {
            for width_limit in [0., 8.4, 17., 26., 35., 1000.] {
                let expected = linear_rows(text, mark.clone(), &starts, width, width_limit);
                let actual = layout_rows(text, mark.clone(), &starts[..3], width, width_limit, "…");
                assert_eq!(actual, expected, "mark={mark:?}, width={width_limit}");
                assert!(actual[1].range.end <= virtual_to_original(starts[2], mark.as_ref()));
            }
        }
    }
}
