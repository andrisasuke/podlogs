//! Headless UI tests (gpui-kit `test-support`). Imports are explicit: a `gpui_kit::*` glob would
//! shadow `#[test]` with gpui's own `test` macro.
use std::{cell::{Cell, RefCell}, rc::Rc};

use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AnyWindowHandle, AppContext, Bounds, Context, Entity, IntoElement, ParentElement, Render, Styled, TestAppContext,
    Window, WindowBounds, WindowOptions, div, point, prelude::*, px, size,
};

use super::button::{Button, Variant};
use super::dropdown::{DropdownEvent, DropdownState};
use super::modal::{ModalSize, modal};
use super::theme::{palette, set_palette};
use gpui_kit::TestSupportExt;

fn open<V: Render>(cx: &mut TestAppContext, build: impl FnOnce(&mut Window, &mut gpui_kit::App) -> Entity<V>) -> (AnyWindowHandle, Entity<V>) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        super::init(cx);
        set_palette(palette(true), cx);
        let bounds = Bounds { origin: point(px(0.), px(0.)), size: size(px(800.), px(600.)) };
        let (w, v) = gpui_kit::open_window(
            WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), ..Default::default() },
            cx,
            build,
        )
        .expect("open test window");
        (w, v)
    })
}

macro_rules! ui {
    ($cx:expr, $handle:expr, |$w:ident, $c:ident| $body:expr) => {
        $cx.update_window($handle, |_, $w, $c| $body).unwrap()
    };
}

// ---------- dropdown ----------
struct DdHarness {
    dd: Entity<DropdownState>,
    _sub: gpui_kit::Subscription,
}

fn dd_harness(cx: &mut TestAppContext, searchable: bool) -> (AnyWindowHandle, Entity<DdHarness>, Entity<DropdownState>, Rc<RefCell<Vec<String>>>) {
    let events = Rc::new(RefCell::new(vec![]));
    let ev = events.clone();
    let (h, view) = open(cx, |window, cx| {
        let dd = cx.new(|cx| DropdownState::new("Select...", searchable.then_some("Search..."), window, cx));
        dd.update(cx, |d, cx| {
            d.set_options(
                ["a", "b", "prod-backend", "dev"].iter().map(|v| (v.to_string(), v.to_string())).collect(),
                cx,
            )
        });
        let sub = cx.subscribe(&dd, move |_, e: &DropdownEvent, _| {
            let DropdownEvent::Change(v) = e;
            ev.borrow_mut().push(v.clone());
        });
        cx.new(|_| DdHarness { dd, _sub: sub })
    });
    let dd = view.read_with(cx, |v, _| v.dd.clone());
    (h, view, dd, events)
}

impl Render for DdHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(div().w(px(240.)).child(self.dd.clone()))
            .child(div().id("outside").test_support().w(px(300.)).h(px(100.)))
    }
}

#[gpui_kit::test]
fn dropdown_opens_selects_and_emits_change(cx: &mut TestAppContext) {
    let (h, _view, dd, events) = dd_harness(cx, false);
    ui!(cx, h, |w, c| {
        w.render_frame(c);
        assert!(w.try_find("dd-popup").is_none());
        w.click("dd-trigger", c);
        assert!(w.try_find("dd-popup").is_some());
        w.click("dd-opt-b", c);
        assert!(w.try_find("dd-popup").is_none());
    });
    assert_eq!(&*events.borrow(), &["b".to_string()]);
    assert_eq!(dd.read_with(cx, |d, _| d.value().map(str::to_string)), Some("b".into()));
}

#[gpui_kit::test]
fn dropdown_click_outside_closes_and_clears_search(cx: &mut TestAppContext) {
    let (h, _view, dd, events) = dd_harness(cx, true);
    ui!(cx, h, |w, c| {
        w.render_frame(c);
        w.click("dd-trigger", c);
        w.input("pro", c);
    });
    assert_eq!(dd.read_with(cx, |d, cx| d.search_text(cx)), "pro");
    ui!(cx, h, |w, c| {
        assert!(w.try_find("dd-popup").is_some());
        // Far right of "outside", clear of the 240 px popup.
        w.click_at("outside", point(px(280.), px(50.)), c);
        assert!(w.try_find("dd-popup").is_none());
    });
    assert_eq!(dd.read_with(cx, |d, cx| d.search_text(cx)), "");
    assert!(events.borrow().is_empty());
}

#[gpui_kit::test]
fn dropdown_escape_closes(cx: &mut TestAppContext) {
    let (h, _view, _dd, _events) = dd_harness(cx, true);
    ui!(cx, h, |w, c| {
        w.render_frame(c);
        w.click("dd-trigger", c);
        assert!(w.try_find("dd-popup").is_some());
        w.press("escape", c);
        assert!(w.try_find("dd-popup").is_none());
    });
}

#[gpui_kit::test]
fn dropdown_search_filters_options_case_insensitive(cx: &mut TestAppContext) {
    let (h, _view, _dd, _events) = dd_harness(cx, true);
    ui!(cx, h, |w, c| {
        w.render_frame(c);
        w.click("dd-trigger", c);
        w.input("PROD", c);
        assert!(w.try_find("dd-opt-prod-backend").is_some());
        assert!(w.try_find("dd-opt-a").is_none());
        assert!(w.try_find("dd-empty").is_none());
        w.input("zzz", c);
        assert!(w.try_find("dd-opt-prod-backend").is_none());
        assert!(w.try_find("dd-empty").is_some());
    });
}

// ---------- button ----------
struct BtnHarness {
    on: Rc<Cell<u32>>,
    off: Rc<Cell<u32>>,
}

impl Render for BtnHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let (on, off) = (self.on.clone(), self.off.clone());
        div()
            .flex()
            .gap(px(8.))
            .child(Button::new("on", Variant::Primary).label("On").on_click(move |_, _, _| on.set(on.get() + 1)))
            .child(
                Button::new("off", Variant::Primary)
                    .label("Off")
                    .disabled(true)
                    .on_click(move |_, _, _| off.set(off.get() + 1)),
            )
    }
}

#[gpui_kit::test]
fn button_disabled_ignores_click(cx: &mut TestAppContext) {
    let (on, off) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
    let (h, _v) = open(cx, |_, cx| cx.new(|_| BtnHarness { on: on.clone(), off: off.clone() }));
    ui!(cx, h, |w, c| {
        w.render_frame(c);
        w.click("on", c);
        w.click("off", c);
    });
    assert_eq!((on.get(), off.get()), (1, 0));
}

// ---------- modal ----------
struct ModalHarness {
    persistent: bool,
    closed: Rc<Cell<u32>>,
}

impl Render for ModalHarness {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let closed = self.closed.clone();
        div().size_full().child(modal(
            "m",
            "Title",
            ModalSize::Md,
            self.persistent,
            Rc::new(move |_, _| closed.set(closed.get() + 1)),
            div().child("body").into_any_element(),
            window,
            cx,
        ))
    }
}

fn modal_click_backdrop(cx: &mut TestAppContext, persistent: bool) -> u32 {
    let closed = Rc::new(Cell::new(0));
    let (h, _v) = open(cx, |_, cx| cx.new(|_| ModalHarness { persistent, closed: closed.clone() }));
    ui!(cx, h, |w, c| {
        w.render_frame(c);
        // Top-left corner of the backdrop: the modal box sits in the middle.
        w.click_at(gpui_kit::ElementId::from((gpui_kit::ElementId::from("m"), "backdrop")), point(px(5.), px(5.)), c);
    });
    closed.get()
}

#[gpui_kit::test]
fn persistent_modal_ignores_backdrop_click(cx: &mut TestAppContext) {
    assert_eq!(modal_click_backdrop(cx, true), 0);
}

#[gpui_kit::test]
fn modal_backdrop_click_closes(cx: &mut TestAppContext) {
    assert_eq!(modal_click_backdrop(cx, false), 1);
}

#[gpui_kit::test]
fn modal_escape_closes_unless_persistent(cx: &mut TestAppContext) {
    for (persistent, want) in [(false, 1), (true, 0)] {
        let closed = Rc::new(Cell::new(0));
        let (h, _v) = open(cx, |_, cx| cx.new(|_| ModalHarness { persistent, closed: closed.clone() }));
        ui!(cx, h, |w, c| {
            w.render_frame(c);
            w.press("escape", c);
        });
        assert_eq!(closed.get(), want, "persistent={persistent}");
    }
}
