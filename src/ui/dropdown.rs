//! `Dropdown.tsx`: trigger + deferred popup + click-outside backdrop + optional search.
use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, input::{InputEvent, InputState}};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::input::search_input;
use super::modal::{CloseOverlay, OVERLAY_CONTEXT};
use super::theme::{Tw, hex, pal, tw};

pub enum DropdownEvent {
    Change(String),
}

pub struct DropdownState {
    placeholder: SharedString,
    options: Vec<(String, String)>, // (value, label)
    value: Option<String>,
    disabled: bool,
    open: bool,
    focus: FocusHandle,
    search: Option<Entity<InputState>>,
    _sub: Option<Subscription>,
}

impl EventEmitter<DropdownEvent> for DropdownState {}

impl DropdownState {
    pub fn new(placeholder: &str, search_placeholder: Option<&str>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = search_placeholder.map(|ph| {
            let ph = ph.to_string();
            cx.new(|cx| InputState::new(window, cx).placeholder(ph))
        });
        let sub = search.as_ref().map(|s| {
            cx.subscribe(s, |_, _, ev: &InputEvent, cx| {
                if matches!(ev, InputEvent::Change) {
                    cx.notify()
                }
            })
        });
        Self {
            placeholder: placeholder.to_string().into(),
            options: vec![],
            value: None,
            disabled: false,
            open: false,
            focus: cx.focus_handle(),
            search,
            _sub: sub,
        }
    }

    pub fn set_options(&mut self, options: Vec<(String, String)>, cx: &mut Context<Self>) {
        self.options = options;
        cx.notify();
    }
    pub fn set_value(&mut self, value: Option<String>, cx: &mut Context<Self>) {
        self.value = value;
        cx.notify();
    }
    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        self.disabled = disabled;
        if disabled {
            self.open = false;
        }
        cx.notify();
    }
    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = false;
        if let Some(s) = &self.search {
            s.update(cx, |s, cx| s.set_value("", window, cx));
        }
        cx.notify();
    }

    pub(crate) fn search_text(&self, cx: &App) -> String {
        self.search.as_ref().map(|s| s.read(cx).value().to_string()).unwrap_or_default()
    }

    fn query(&self, cx: &App) -> String {
        self.search.as_ref().map(|s| s.read(cx).value().to_lowercase()).unwrap_or_default()
    }
}

impl Render for DropdownState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = pal(cx);
        let open = self.open;
        let disabled = self.disabled;
        let selected = self.options.iter().find(|(v, _)| Some(v) == self.value.as_ref());
        let q = self.query(cx);
        let filtered: Vec<(String, String)> =
            self.options.iter().filter(|(_, l)| q.is_empty() || l.to_lowercase().contains(&q)).cloned().collect();
        let vp = window.viewport_size();

        let trigger = div()
            .id("dd-trigger")
            .test_support()
            .w_full()
            .h(px(36.))
            .px(px(12.))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .rounded(px(6.))
            .t_sm()
            .bg(p.bg3)
            .border_1()
            .border_color(p.border)
            .text_color(p.text1)
            .map(|d| if disabled { d.opacity(0.5) } else { d.cursor_pointer().hover(|s| s.bg(p.bg2)) })
            .child(
                div()
                    .truncate()
                    .when(selected.is_none(), |d| d.text_color(p.muted))
                    .child(selected.map(|(_, l)| l.clone()).unwrap_or_else(|| self.placeholder.to_string())),
            )
            .child(
                Icon::new(IconName::ChevronDown)
                    .size_4()
                    .flex_none()
                    .text_color(p.muted)
                    .when(open, |i| i.rotate(radians(std::f32::consts::PI))),
            )
            .when(!disabled, |d| {
                d.on_click(cx.listener(|this, _, window, cx| {
                    if this.open {
                        this.close(window, cx);
                        return;
                    }
                    this.open = true;
                    match &this.search {
                        Some(s) => s.update(cx, |s, cx| s.focus(window, cx)),
                        None => window.focus(&this.focus, cx),
                    }
                    cx.notify();
                }))
            });

        div()
            .relative()
            .w_full()
            .key_context(OVERLAY_CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &CloseOverlay, window, cx| {
                if this.open {
                    this.close(window, cx);
                }
            }))
            .child(trigger)
            .when(open, |el| {
                el
                    // click-outside backdrop (React: document mousedown listener)
                    .child(
                        deferred(
                            anchored().position_mode(AnchoredPositionMode::Window).position(point(px(0.), px(0.))).child(
                                div().id("dd-backdrop").test_support().w(vp.width).h(vp.height).occlude().on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, _, window, cx| this.close(window, cx)),
                                ),
                            ),
                        )
                        .with_priority(1),
                    )
                    .child(
                        deferred(
                            div()
                                .id("dd-popup")
                                .test_support()
                                .absolute()
                                .top(px(40.)) // h-9 trigger + mt-1
                                .left_0()
                                .right_0()
                                .occlude()
                                .rounded(px(6.))
                                .shadow_md()
                                .bg(p.bg2)
                                .border_1()
                                .border_color(p.border)
                                .max_h(px(300.))
                                .overflow_hidden()
                                .flex()
                                .flex_col()
                                .when_some(self.search.as_ref(), |d, s| {
                                    d.child(div().p(px(8.)).border_b_1().border_color(p.border).child(search_input(s, cx).h(px(32.))))
                                })
                                .child(
                                    div()
                                        .id("dd-list")
                                        .overflow_y_scroll()
                                        .py(px(4.))
                                        .children(filtered.iter().map(|(value, label)| {
                                            let sel = Some(value) == self.value.as_ref();
                                            let v = value.clone();
                                            div()
                                                .id(SharedString::from(format!("dd-opt-{value}")))
                                                .test_support()
                                                .w_full()
                                                .px(px(12.))
                                                .py(px(8.))
                                                .flex()
                                                .items_center()
                                                .gap(px(8.))
                                                .t_sm()
                                                .text_color(if sel { hex(tw::ACCENT) } else { p.text1 })
                                                .cursor_pointer()
                                                .hover(|s| s.bg(p.bg3))
                                                .child(if sel {
                                                    Icon::new(IconName::Check).size_4().into_any_element()
                                                } else {
                                                    div().w(px(16.)).flex_none().into_any_element()
                                                })
                                                .child(label.clone())
                                                .on_click(cx.listener(move |this, _, window, cx| {
                                                    this.value = Some(v.clone());
                                                    this.close(window, cx);
                                                    cx.emit(DropdownEvent::Change(v.clone()));
                                                }))
                                        }))
                                        .when(filtered.is_empty(), |d| {
                                            d.child(
                                                div()
                                                    .id("dd-empty")
                                                    .test_support()
                                                    .px(px(12.))
                                                    .py(px(8.))
                                                    .t_sm()
                                                    .text_color(p.muted)
                                                    .child("No results found"),
                                            )
                                        }),
                                ),
                        )
                        .with_priority(2),
                    )
            })
    }
}
