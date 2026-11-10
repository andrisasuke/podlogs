//! TEMPORARY (remove after the owner's checks): `PODLOGS_GALLERY=1 cargo run`.
//! Left: the Task 6 letter-spacing label preview (sidebar 260 px, bg-secondary, labels at x = 16 px,
//! vertical metrics from `c70bd05:src/components/layout/Sidebar.tsx`). Right: every shared widget.
//! The "Switch theme" button flips light/dark live; `PODLOGS_GALLERY_THEME=light` starts in light.
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    Icon,
    input::{InputState},
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::badge::{dot, label_badge, level_badge, status_badge};
use super::button::{BtnSize, Button, Variant};
use super::collapsible::collapsible_header;
use super::dropdown::DropdownState;
use super::input::{search_input, text_input};
use super::modal::{ModalSize, modal};
use super::drawer::drawer;
use super::section_label;
use super::skeleton::{log_skeleton, table_skeleton};
use super::theme::{Palette, Tw, UI_FONT, hex, pal, palette, set_palette, tw};

pub fn enabled() -> bool {
    std::env::var("PODLOGS_GALLERY").is_ok_and(|v| v == "1")
}

pub fn start_dark() -> bool {
    !std::env::var("PODLOGS_GALLERY_THEME").is_ok_and(|v| v == "light")
}

pub struct Gallery {
    dark: bool,
    dd_plain: Entity<DropdownState>,
    dd_search: Entity<DropdownState>,
    dd_disabled: Entity<DropdownState>,
    text: Entity<InputState>,
    search: Entity<InputState>,
    modal: Option<bool>, // Some(persistent)
    drawer_open: bool,
    section_open: bool,
}

fn options(vals: &[&str]) -> Vec<(String, String)> {
    vals.iter().map(|v| (v.to_string(), v.to_string())).collect()
}

impl Gallery {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mk = |ph: &str, sp: Option<&str>, opts: &[&str], val: Option<&str>, disabled: bool, window: &mut Window, cx: &mut Context<Self>| {
            let (ph, sp) = (ph.to_string(), sp.map(str::to_string));
            let dd = cx.new(|cx| DropdownState::new(&ph, sp.as_deref(), window, cx));
            dd.update(cx, |d, cx| {
                d.set_options(options(opts), cx);
                d.set_value(val.map(str::to_string), cx);
                d.set_disabled(disabled, cx);
            });
            dd
        };
        let ns = ["default", "kube-system", "prod-backend", "prod-frontend", "monitoring"];
        Self {
            dark: start_dark(),
            dd_plain: mk("Select cluster...", None, &["production-cluster-us-east", "staging-cluster", "dev-local"], Some("staging-cluster"), false, window, cx),
            dd_search: mk("Select namespace...", Some("Search namespaces..."), &ns, Some("prod-backend"), false, window, cx),
            dd_disabled: mk("Disabled...", None, &ns, None, true, window, cx),
            text: cx.new(|cx| InputState::new(window, cx).placeholder("Text input")),
            search: cx.new(|cx| InputState::new(window, cx).placeholder("Search pods...")),
            modal: None,
            drawer_open: false,
            section_open: true,
        }
    }
}

/// Task 6 gate: the three sidebar section labels at their React positions. The dropdowns, nav buttons
/// and input are empty spacers so only the labels are drawn.
fn label_preview_strip(cx: &App) -> Div {
    let p = pal(cx);
    let label_block = |text: &'static str, below: f32| {
        div().flex().flex_col().child(section_label(text, cx)).child(div().h(px(8. + below)))
    };
    div()
        .w(px(260.))
        .h_full()
        .flex_none()
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
        .child(div().px(px(16.)).pt(px(16.)).child(label_block("DEPLOYMENTS", 36.)))
}

fn section(title: &'static str, p: &Palette, body: impl IntoElement) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(12.))
        .child(div().t_xs().font_weight(FontWeight::SEMIBOLD).text_color(p.muted).child(title))
        .child(div().flex().flex_wrap().items_center().gap(px(12.)).child(body))
}

impl Render for Gallery {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = pal(cx);
        let this = cx.entity();

        let buttons = div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .children([Variant::Primary, Variant::Secondary, Variant::Ghost].into_iter().enumerate().map(|(vi, v)| {
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(Button::new(("b-sm", vi), v).size(BtnSize::Sm).label("Small"))
                    .child(Button::new(("b-md", vi), v).label("Medium"))
                    .child(Button::new(("b-lg", vi), v).size(BtnSize::Lg).label("Large"))
                    .child(Button::new(("b-ic", vi), v).size(BtnSize::Icon).icon(IconName::Settings).tooltip("Settings"))
                    .child(Button::new(("b-ico", vi), v).size(BtnSize::Sm).icon(IconName::RefreshCw).label("Refresh"))
                    .child(Button::new(("b-search", vi), v).icon(IconName::Search).icon_gap(8.).label("Search"))
                    .child(Button::new(("b-dis", vi), v).size(BtnSize::Sm).label("Disabled").disabled(true))
            }));

        let statuses = [
            ("Running", Some("1/1")),
            ("Running", Some("0/1")),
            ("Pending", None),
            ("CrashLoopBackOff", None),
            ("Terminating", None),
            ("Unknown", None),
        ];
        let badges = div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(div().flex().items_center().gap(px(12.)).children(statuses.map(|(s, r)| status_badge(s, r, cx))))
            .child(div().flex().items_center().gap(px(12.)).children(["ERROR", "WARN", "INFO", "DEBUG"].map(|l| level_badge(l, cx))))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(label_badge("app=api-gateway", cx))
                    .child(label_badge("tier=backend", cx))
                    .child(dot(hex(tw::EMERALD_500), 8.))
                    .child(dot(hex(tw::AMBER_500), 8.))
                    .child(dot(hex(tw::RED_500), 8.)),
            );

        let dropdowns = div()
            .flex()
            .gap(px(16.))
            .items_start()
            .child(div().w(px(240.)).child(self.dd_plain.clone()))
            .child(div().w(px(240.)).child(self.dd_search.clone()))
            .child(div().w(px(240.)).child(self.dd_disabled.clone()));

        let inputs = div()
            .flex()
            .gap(px(16.))
            .child(div().w(px(240.)).child(search_input(&self.search, cx)))
            .child(div().w(px(240.)).child(text_input(&self.text, cx)));

        let icons = [
            ("Layers", IconName::Layers),
            ("Search", IconName::Search),
            ("Moon", IconName::Moon),
            ("Sun", IconName::Sun),
            ("Settings", IconName::Settings),
            ("ChevronDown", IconName::ChevronDown),
            ("Check", IconName::Check),
            ("ArrowLeft", IconName::ArrowLeft),
            ("Download", IconName::Download),
            ("Copy", IconName::Copy),
            ("X", IconName::X),
            ("TriangleAlert", IconName::TriangleAlert),
            ("RefreshCw", IconName::RefreshCw),
            ("Package", IconName::Package),
            ("PanelLeftClose", IconName::PanelLeftClose),
            ("PanelLeftOpen", IconName::PanelLeftOpen),
        ];
        let icon_row = div().flex().flex_wrap().gap(px(16.)).children(icons.map(|(n, i)| {
            div()
                .w(px(96.))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(4.))
                .t_xs()
                .text_color(p.text2)
                .child(Icon::new(i).size_5())
                .child(n)
        }));

        let toggle = this.clone();
        let t_collapse = this.clone();
        let t_modal = this.clone();
        let t_modal_p = this.clone();
        let t_drawer = this.clone();

        let content = div()
            .id("gallery-scroll")
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_y_scroll()
            .p(px(24.))
            .flex()
            .flex_col()
            .gap(px(28.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(16.))
                    .child(div().t_lg().font_weight(FontWeight::SEMIBOLD).child("Widget gallery (temporary)"))
                    .child(
                        Button::new("theme-toggle", Variant::Secondary)
                            .size(BtnSize::Sm)
                            .icon(if self.dark { IconName::Sun } else { IconName::Moon })
                            .label(if self.dark { "Switch to light" } else { "Switch to dark" })
                            .on_click(move |_, _, cx| {
                                toggle.update(cx, |g, cx| {
                                    g.dark = !g.dark;
                                    set_palette(palette(g.dark), cx);
                                    cx.notify();
                                })
                            }),
                    ),
            )
            .child(section("BUTTONS", &p, buttons))
            .child(section("BADGES & DOTS", &p, badges))
            .child(section("INPUTS", &p, inputs))
            .child(section("DROPDOWNS (plain, searchable, disabled)", &p, dropdowns))
            .child(section(
                "SKELETONS",
                &p,
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .child(table_skeleton(2, cx))
                    .child(log_skeleton(3, cx)),
            ))
            .child(section(
                "COLLAPSIBLE",
                &p,
                div().flex().flex_col().gap(px(8.)).child(collapsible_header(
                    "gallery-collapsible",
                    "Conditions",
                    self.section_open,
                    move |_, cx| t_collapse.update(cx, |g, cx| {
                        g.section_open = !g.section_open;
                        cx.notify();
                    }),
                    cx,
                )).when(self.section_open, |d| d.child(div().t_sm().text_color(p.text2).child("Collapsible body is visible"))),
            ))
            .child(section(
                "OVERLAYS",
                &p,
                div()
                    .flex()
                    .gap(px(12.))
                    .child(Button::new("open-modal", Variant::Secondary).size(BtnSize::Sm).label("Open modal").on_click(move |_, _, cx| {
                        t_modal.update(cx, |g, cx| {
                            g.modal = Some(false);
                            cx.notify();
                        })
                    }))
                    .child(Button::new("open-modal-p", Variant::Secondary).size(BtnSize::Sm).label("Open persistent modal").on_click(move |_, _, cx| {
                        t_modal_p.update(cx, |g, cx| {
                            g.modal = Some(true);
                            cx.notify();
                        })
                    }))
                    .child(Button::new("open-drawer", Variant::Secondary).size(BtnSize::Sm).label("Open drawer").on_click(move |_, _, cx| {
                        t_drawer.update(cx, |g, cx| {
                            g.drawer_open = true;
                            cx.notify();
                        })
                    })),
            ))
            .child(section("ICONS", &p, icon_row));

        let close_modal = {
            let this = this.clone();
            Rc::new(move |_: &mut Window, cx: &mut App| {
                this.update(cx, |g, cx| {
                    g.modal = None;
                    cx.notify();
                })
            })
        };
        let close_drawer = {
            let this = this.clone();
            Rc::new(move |_: &mut Window, cx: &mut App| {
                this.update(cx, |g, cx| {
                    g.drawer_open = false;
                    cx.notify();
                })
            })
        };

        let modal_el = self.modal.map(|persistent| {
            let on_close = close_modal.clone();
            modal(
                "gallery-modal",
                if persistent { "Connection Error (persistent)" } else { "Log Details" },
                ModalSize::Md,
                persistent,
                close_modal.clone(),
                div()
                    .p(px(16.))
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .t_sm()
                    .child(if persistent { "Only the button closes this one: ESC and backdrop clicks are ignored." } else { "ESC, the X button and a backdrop click close this." })
                    .child(Button::new("modal-ok", Variant::Primary).size(BtnSize::Sm).label("Close").on_click(move |_, w, cx| on_close(w, cx)))
                    .into_any_element(),
                window,
                cx,
            )
        });
        let drawer_el = drawer(
            "gallery-drawer",
            "Settings",
            self.drawer_open,
            close_drawer,
            div().p(px(16.)).t_sm().text_color(p.text2).child("Drawer body: 480 px wide, slides in over 300 ms.").into_any_element(),
            window,
            cx,
        );

        div()
            .size_full()
            .flex()
            .font_family(UI_FONT)
            .text_size(px(14.))
            .line_height(relative(1.5))
            .text_color(p.text1)
            .bg(p.bg1)
            .child(label_preview_strip(cx))
            .child(content)
            .child(drawer_el)
            .children(modal_el)
    }
}
