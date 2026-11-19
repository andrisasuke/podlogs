//! `Sidebar.tsx`: expanded 260 px sidebar and the 60 px collapsed rail (reachable through the new toggle).
use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, input::{InputEvent, InputState}, tooltip::Tooltip};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::store::{Store, View};
use crate::models::DeploymentInfo;
use crate::status::health;
use crate::ui::badge::{dot, health_color};
use crate::ui::button::{BtnSize, Button, Variant};
use crate::ui::dropdown::{DropdownEvent, DropdownState};
use crate::ui::input::search_input;
use crate::ui::section_label;
use crate::ui::theme::{Tw, hex, hexa, pal, tw};

pub const SIDEBAR_WIDTH: f32 = 260.;
pub const SIDEBAR_COLLAPSED_WIDTH: f32 = 60.;

pub struct SidebarView {
    store: Entity<Store>,
    cluster: Entity<DropdownState>,
    namespace: Entity<DropdownState>,
    pub(crate) filter: Entity<InputState>,
    _subs: Vec<Subscription>,
}

impl SidebarView {
    pub fn new(store: Entity<Store>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let cluster = cx.new(|cx| DropdownState::new("Select cluster...", None, window, cx));
        let namespace = cx.new(|cx| DropdownState::new("Select namespace...", Some("Search namespaces..."), window, cx));
        let filter = cx.new(|cx| InputState::new(window, cx).placeholder("Filter deployments..."));
        let subs = vec![
            cx.observe(&store, |this, _, cx| this.sync(cx)),
            cx.subscribe(&cluster, |this, _, ev: &DropdownEvent, cx| {
                let DropdownEvent::Change(v) = ev;
                this.store.update(cx, |s, cx| s.set_context(v.clone(), cx));
            }),
            cx.subscribe(&namespace, |this, _, ev: &DropdownEvent, cx| {
                let DropdownEvent::Change(v) = ev;
                this.store.update(cx, |s, cx| s.set_namespace(v.clone(), cx));
            }),
            cx.subscribe(&filter, |_, _, ev: &InputEvent, cx| {
                if matches!(ev, InputEvent::Change) {
                    cx.notify()
                }
            }),
        ];
        let mut this = Self { store, cluster, namespace, filter, _subs: subs };
        this.sync(cx);
        this
    }

    /// Pushes the store's clusters/namespaces/selection into the two dropdowns.
    fn sync(&mut self, cx: &mut Context<Self>) {
        let s = self.store.read(cx);
        let opts = |names: Option<Vec<String>>| -> Vec<(String, String)> {
            names.unwrap_or_default().into_iter().map(|n| (n.clone(), n)).collect()
        };
        let clusters = opts(s.clusters.data().map(|c| c.iter().map(|c| c.name.clone()).collect()));
        let namespaces = opts(s.namespaces.data().map(|n| n.iter().map(|n| n.name.clone()).collect()));
        let (ctx, ns) = (s.context.clone(), s.namespace.clone());
        self.cluster.update(cx, |d, cx| {
            d.set_options(clusters, cx);
            d.set_value((!ctx.is_empty()).then(|| ctx.clone()), cx);
        });
        self.namespace.update(cx, |d, cx| {
            d.set_options(namespaces, cx);
            d.set_value((!ns.is_empty()).then_some(ns), cx);
            d.set_disabled(ctx.is_empty(), cx);
        });
        cx.notify();
    }

    fn theme_button(&self, full: bool, cx: &App) -> Button {
        let store = self.store.clone();
        let dark = self.store.read(cx).is_dark();
        let b = Button::new("theme-toggle", Variant::Ghost)
            .size(BtnSize::Icon)
            .icon(if dark { IconName::Sun } else { IconName::Moon })
            .on_click(move |_, _, cx| store.update(cx, |s, cx| s.toggle_theme(cx)));
        if full { b.full_width() } else { b }
    }

    fn toggle_button(&self, id: &'static str, icon: IconName, tip: &'static str, full: bool) -> Button {
        let store = self.store.clone();
        let b = Button::new(id, Variant::Ghost)
            .size(BtnSize::Icon)
            .icon(icon)
            .tooltip(tip)
            .on_click(move |_, _, cx| store.update(cx, |s, cx| s.toggle_sidebar(cx)));
        if full { b.full_width() } else { b }
    }

    fn nav_items() -> [(View, &'static str, &'static str, IconName); 2] {
        [(View::Deployments, "nav-deployments", "Deployments", IconName::Layers), (View::Search, "nav-search", "Log Search", IconName::Search)]
    }

    fn rail(&self, cx: &App) -> AnyElement {
        let p = pal(cx);
        let view = self.store.read(cx).view;
        div()
            .id("sidebar-rail")
            .test_support()
            .w(px(SIDEBAR_COLLAPSED_WIDTH))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .bg(p.bg2)
            .border_r_1()
            .border_color(p.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .h(px(56.))
                    .flex_none()
                    .border_b_1()
                    .border_color(p.border)
                    .child(img("assets/icon.png").size_8()),
            )
            .child(div().flex_1().py(px(16.)).children(Self::nav_items().into_iter().map(|(v, id, label, icon)| {
                let store = self.store.clone();
                let active = view == v;
                div()
                    .id(id)
                    .test_support()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .py(px(12.))
                    .cursor_pointer()
                    .text_color(if active { hex(tw::ACCENT) } else { p.text2 })
                    .when(active, |d| d.bg(hexa(tw::ACCENT, 0.1)))
                    .when(!active, |d| d.hover(|s| s.bg(p.bg3)))
                    .tooltip(move |window, cx| Tooltip::new(label).build(window, cx))
                    .child(Icon::new(icon).size_5())
                    .on_click(move |_, _, cx| store.update(cx, |s, cx| s.set_view(v, cx)))
            })))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .p(px(8.))
                    .flex_none()
                    .border_t_1()
                    .border_color(p.border)
                    .child(self.toggle_button("expand-sidebar", IconName::PanelLeftOpen, "Expand sidebar", true))
                    .child(self.theme_button(true, cx)),
            )
            .into_any_element()
    }

    fn expanded(&self, cx: &App) -> AnyElement {
        let p = pal(cx);
        let s = self.store.read(cx);
        let view = s.view;
        let selected = s.deployment.clone();
        let q = self.filter.read(cx).value().to_lowercase();
        let deployments: Vec<DeploymentInfo> = s
            .deployments
            .data()
            .map(|d| d.iter().filter(|d| q.is_empty() || d.name.to_lowercase().contains(&q)).cloned().collect())
            .unwrap_or_default();
        let block = |label: &'static str, dd: &Entity<DropdownState>| {
            div().child(div().mb(px(8.)).child(section_label(label, cx))).child(dd.clone())
        };

        div()
            .id("sidebar-expanded")
            .test_support()
            .w(px(SIDEBAR_WIDTH))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .bg(p.bg2)
            .border_r_1()
            .border_color(p.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(16.))
                    .h(px(56.))
                    .flex_none()
                    .border_b_1()
                    .border_color(p.border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(img("assets/icon.png").size_7().flex_none())
                            .child(div().font_weight(FontWeight::SEMIBOLD).text_color(p.text1).child("PodLogs")),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .child(self.toggle_button("collapse-sidebar", IconName::PanelLeftClose, "Collapse sidebar", false))
                            .child(self.theme_button(false, cx)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(16.))
                    .px(px(16.))
                    .py(px(16.))
                    .flex_none()
                    .border_b_1()
                    .border_color(p.border)
                    .child(block("CLUSTER", &self.cluster))
                    .child(block("NAMESPACE", &self.namespace)),
            )
            .child(
                div().flex_none().px(px(8.)).py(px(12.)).border_b_1().border_color(p.border).children(
                    Self::nav_items().into_iter().map(|(v, id, label, icon)| {
                        let store = self.store.clone();
                        let active = view == v;
                        div()
                            .id(id)
                            .test_support()
                            .w_full()
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .px(px(12.))
                            .py(px(8.))
                            .rounded(px(6.))
                            .t_sm()
                            .cursor_pointer()
                            .text_color(if active { hex(tw::ACCENT) } else { p.text2 })
                            .when(active, |d| d.bg(hexa(tw::ACCENT, 0.1)))
                            .when(!active, |d| d.hover(|s| s.bg(p.bg3)))
                            .child(Icon::new(icon).size_4())
                            .child(label)
                            .on_click(move |_, _, cx| store.update(cx, |s, cx| s.set_view(v, cx)))
                    }),
                ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(
                        div()
                            .flex_none()
                            .px(px(16.))
                            .pt(px(16.))
                            .pb(px(8.))
                            .child(div().mb(px(8.)).child(section_label("DEPLOYMENTS", cx)))
                            .child(search_input(&self.filter, cx)),
                    )
                    .child(
                        div()
                            .id("sidebar-deployments")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .px(px(8.))
                            .pb(px(8.))
                            .children(deployments.iter().map(|d| {
                                let store = self.store.clone();
                                let name = d.name.clone();
                                let active = selected.as_deref() == Some(d.name.as_str());
                                div()
                                    .id(SharedString::from(format!("dep-{}", d.name)))
                                    .test_support()
                                    .w_full()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .px(px(12.))
                                    .py(px(8.))
                                    .rounded(px(6.))
                                    .t_sm()
                                    .cursor_pointer()
                                    .text_color(if active { hex(tw::ACCENT) } else { p.text2 })
                                    .when(active, |d| d.bg(hexa(tw::ACCENT, 0.1)))
                                    .when(!active, |d| d.hover(|s| s.bg(p.bg3)))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(8.))
                                            .min_w_0()
                                            .child(dot(health_color(health(d.replicas, d.available_replicas)), 6.))
                                            .child(div().min_w_0().truncate().child(d.name.clone())),
                                    )
                                    .child(div().flex_none().ml(px(8.)).t_xs().text_color(p.muted).child(d.replicas.to_string()))
                                    .on_click(move |_, _, cx| {
                                        store.update(cx, |s, cx| s.toggle_deployment_and_show_pods(name.clone(), cx))
                                    })
                            }))
                            .when(deployments.is_empty(), |d| {
                                d.child(
                                    div()
                                        .id("sidebar-empty")
                                        .test_support()
                                        .px(px(12.))
                                        .py(px(16.))
                                        .t_sm()
                                        .text_color(p.muted)
                                        .child("No deployments found"),
                                )
                            }),
                    ),
            )
            .into_any_element()
    }
}

impl Render for SidebarView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.store.read(cx).sidebar_collapsed { self.rail(cx) } else { self.expanded(cx) }
    }
}
