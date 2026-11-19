//! `Modal.tsx`: deferred window-anchored overlay. Persistent modals ignore ESC and backdrop clicks.
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::button::{BtnSize, Button, Variant};
use super::theme::{Tw, pal};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModalSize {
    Sm,
    Md,
    Lg,
    Xl,
}

impl ModalSize {
    pub fn width(self) -> f32 {
        match self {
            Self::Sm => 400.,
            Self::Md => 500.,
            Self::Lg => 700.,
            Self::Xl => 900.,
        }
    }
}

pub type OnClose = Rc<dyn Fn(&mut Window, &mut App)>;

actions!(overlay, [CloseOverlay]);

/// Context for the Escape binding (see `ui::init`).
pub const OVERLAY_CONTEXT: &str = "Overlay";

struct OverlayFocus {
    handle: FocusHandle,
    grabbed: bool,
}

/// A focus handle kept per overlay `id`. While `active`, focus is moved into the overlay once, so the
/// Escape binding (context "Overlay") is on the key path even though nothing else is focused.
pub fn overlay_focus(id: &ElementId, active: bool, window: &mut Window, cx: &mut App) -> FocusHandle {
    let st = window.use_keyed_state((id.clone(), "focus"), cx, |_, cx| OverlayFocus { handle: cx.focus_handle(), grabbed: false });
    let (handle, grab) = st.update(cx, |s, _| {
        let grab = active && !s.grabbed;
        s.grabbed = active;
        (s.handle.clone(), grab)
    });
    if grab {
        window.focus(&handle, cx);
    }
    handle
}

#[allow(clippy::too_many_arguments)]
pub fn modal(
    id: impl Into<ElementId>,
    title: &str,
    size: ModalSize,
    persistent: bool,
    on_close: OnClose,
    body: AnyElement,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let id: ElementId = id.into();
    let p = pal(cx);
    let vp = window.viewport_size();
    let close_btn = on_close.clone();
    let on_backdrop = on_close.clone();
    let on_esc = on_close;
    let focus = overlay_focus(&id, !persistent, window, cx);
    deferred(
        anchored().position_mode(AnchoredPositionMode::Window).position(point(px(0.), px(0.))).child(
            div()
                .id(id.clone())
                .w(vp.width)
                .h(vp.height)
                .flex()
                .items_center()
                .justify_center()
                .key_context(OVERLAY_CONTEXT)
                .track_focus(&focus)
                .when(!persistent, |d| d.on_action(move |_: &CloseOverlay, w, cx| on_esc(w, cx)))
                .child(
                    div()
                        .id((id.clone(), "backdrop"))
                        .test_support()
                        .absolute()
                        .inset_0()
                        .bg(hsla(0., 0., 0., if p.dark { 0.7 } else { 0.5 }))
                        .occlude()
                        .when(!persistent, |d| {
                            d.on_mouse_down(MouseButton::Left, move |_, window, cx| on_backdrop(window, cx))
                        }),
                )
                .child(
                    div()
                        .id((id.clone(), "box"))
                        .relative()
                        .occlude()
                        .w(px(size.width()))
                        .max_h(vp.height * 0.85)
                        .flex()
                        .flex_col()
                        .bg(p.bg1)
                        .border_1()
                        .border_color(p.border)
                        .rounded(px(8.))
                        .shadow_lg()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .px(px(16.))
                                .py(px(12.))
                                .flex_none()
                                .border_b_1()
                                .border_color(p.border)
                                .child(div().t_sm().font_weight(FontWeight::SEMIBOLD).text_color(p.text1).child(title.to_string()))
                                .child(
                                    Button::new((id.clone(), "close"), Variant::Ghost)
                                        .size(BtnSize::Icon)
                                        .icon(IconName::X)
                                        .on_click(move |_, w, cx| close_btn(w, cx)),
                                ),
                        )
                        .child(div().id((id, "body")).flex_1().min_h_0().overflow_y_scroll().child(body)),
                ),
        ),
    )
    .with_priority(4)
    .into_any_element()
}
