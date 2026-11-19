//! `Drawer.tsx`: 480 px right drawer; 300 ms ease-out slide + backdrop fade, both ways.
//! The drawer stays rendered until the close animation ends (state kept per `id`).
use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::button::{BtnSize, Button, Variant};
use super::modal::{CloseOverlay, OVERLAY_CONTEXT, OnClose, overlay_focus};
use super::theme::{Tw, pal};

pub const DRAWER_WIDTH: f32 = 480.;
const MS: u64 = 300;

struct DrawerAnim {
    open: bool,
    visible: bool,
    epoch: usize,
}

/// Tailwind `ease-out` = cubic-bezier(0, 0, .2, 1); a cubic ease-out is within a few percent of it.
fn ease_out(t: f32) -> f32 {
    1. - (1. - t).powi(3)
}

pub fn drawer(
    id: impl Into<ElementId>,
    title: &str,
    open: bool,
    on_close: OnClose,
    body: AnyElement,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let id: ElementId = id.into();
    let p = pal(cx);
    let anim = window.use_keyed_state((id.clone(), "anim"), cx, |_, _| DrawerAnim { open: false, visible: false, epoch: 0 });
    let (visible, epoch) = anim.update(cx, |a, _| {
        if a.open != open {
            a.open = open;
            a.visible = true;
            a.epoch += 1;
        }
        (a.visible, a.epoch)
    });
    if !visible {
        return div().size_0().into_any_element();
    }
    if !open {
        // Hide for good once the close animation is done (unless reopened meanwhile).
        let anim = anim.clone();
        window
            .spawn(cx, async move |cx| {
                cx.background_executor().timer(Duration::from_millis(MS)).await;
                let _ = cx.update(|_, cx| {
                    anim.update(cx, |a, cx| {
                        if !a.open && a.epoch == epoch {
                            a.visible = false;
                            cx.notify();
                        }
                    })
                });
            })
            .detach();
    }

    let focus = overlay_focus(&id, open, window, cx);
    let vp = window.viewport_size();
    let progress = move |t: f32| if open { ease_out(t) } else { 1. - ease_out(t) };
    let on_backdrop = on_close.clone();
    let on_x = on_close.clone();
    let on_esc = on_close;

    deferred(
        anchored().position_mode(AnchoredPositionMode::Window).position(point(px(0.), px(0.))).child(
            div()
                .id(id.clone())
                .relative()
                .w(vp.width)
                .h(vp.height)
                .key_context(OVERLAY_CONTEXT)
                .track_focus(&focus)
                .when(open, |d| d.on_action(move |_: &CloseOverlay, w, cx| on_esc(w, cx)))
                .child(
                    div()
                        .id((id.clone(), "backdrop"))
                        .test_support()
                        .absolute()
                        .inset_0()
                        .bg(hsla(0., 0., 0., if p.dark { 0.4 } else { 0.2 }))
                        .when(open, |d| {
                            d.occlude().on_mouse_down(MouseButton::Left, move |_, w, cx| on_backdrop(w, cx))
                        })
                        .with_animation(
                            (id.clone(), SharedString::from(format!("fade{epoch}"))),
                            Animation::new(Duration::from_millis(MS)),
                            move |el, t| el.opacity(progress(t)),
                        ),
                )
                .child(
                    div()
                        .id((id.clone(), "panel"))
                        .test_support()
                        .absolute()
                        .top_0()
                        .h_full()
                        .w(px(DRAWER_WIDTH))
                        .occlude()
                        .flex()
                        .flex_col()
                        .bg(p.bg2)
                        .border_l_1()
                        .border_color(p.border)
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(8.))
                                .px(px(16.))
                                .py(px(12.))
                                .flex_none()
                                .border_b_1()
                                .border_color(p.border)
                                .child(
                                    div()
                                        .min_w_0()
                                        .truncate()
                                        .t_sm()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(p.text1)
                                        .child(title.to_string()),
                                )
                                .child(
                                    Button::new((id.clone(), "close"), Variant::Ghost)
                                        .size(BtnSize::Icon)
                                        .icon(IconName::X)
                                        .on_click(move |_, w, cx| on_x(w, cx)),
                                ),
                        )
                        .child(div().id((id.clone(), "body")).flex_1().min_h_0().overflow_y_scroll().child(body))
                        .with_animation(
                            (id, SharedString::from(format!("slide{epoch}"))),
                            Animation::new(Duration::from_millis(MS)),
                            move |el, t| el.right(px(-DRAWER_WIDTH * (1. - progress(t)))),
                        ),
                ),
        ),
    )
    .with_priority(3)
    .into_any_element()
}
