//! `DeploymentList.tsx`: header + search + settings, table, skeleton, empty state.
use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, input::{InputEvent, InputState}};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::store::{Store, View};
use crate::status::health;
use crate::ui::badge::health_color;
use crate::ui::button::{BtnSize, Button, Variant};
use crate::ui::input::search_input;
use crate::ui::skeleton::table_skeleton;
use crate::ui::theme::{Tw, pal};
use crate::ui::tracked_text::tracked;

pub struct DeploymentsView {
    store: Entity<Store>,
    search: Entity<InputState>,
    _subs: Vec<Subscription>,
}

impl DeploymentsView {
    pub fn new(store: Entity<Store>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search deployments..."));
        let subs = vec![
            cx.observe(&store, |_, _, cx| cx.notify()),
            cx.subscribe(&search, |_, _, ev: &InputEvent, cx| {
                if matches!(ev, InputEvent::Change) {
                    cx.notify()
                }
            }),
        ];
        Self { store, search, _subs: subs }
    }
}

fn th(text: &'static str) -> Div {
    div().t_xs().font_weight(FontWeight::MEDIUM).child(tracked(text))
}

impl Render for DeploymentsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = pal(cx);
        let s = self.store.read(cx);
        let loading = s.deployments.is_loading();
        let q = self.search.read(cx).value().to_lowercase();
        let rows: Vec<_> = s
            .deployments
            .data()
            .map(|d| d.iter().filter(|d| d.name.to_lowercase().contains(&q)).cloned().collect())
            .unwrap_or_default();
        let store = self.store.clone();

        let header = div()
            .flex()
            .flex_none()
            .items_center()
            .justify_between()
            .px(px(24.))
            .py(px(16.))
            .border_b_1()
            .border_color(p.border)
            .child(div().t_lg().font_weight(FontWeight::SEMIBOLD).text_color(p.text1).child("Deployments"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(div().w(px(256.)).child(search_input(&self.search, cx)))
                    .child(
                        Button::new("open-settings", Variant::Ghost)
                            .size(BtnSize::Icon)
                            .icon(IconName::Settings)
                            .tooltip("Settings")
                            .on_click({
                                let store = store.clone();
                                move |_, _, cx| store.update(cx, |s, cx| s.open_settings(cx))
                            }),
                    ),
            );

        let table = div()
            .child(
                div()
                    .flex()
                    .items_center()
                    .text_color(p.muted)
                    .border_b_1()
                    .border_color(p.border)
                    .bg(p.bg1)
                    .child(th("NAME").flex_1().px(px(24.)).py(px(12.)))
                    .child(th("REPLICAS").w(px(128.)).px(px(16.)).py(px(12.)).flex().justify_center())
                    .child(th("AVAILABLE").w(px(128.)).px(px(16.)).py(px(12.)).flex().justify_center())
                    .child(th("READY").w(px(128.)).px(px(16.)).py(px(12.)).flex().justify_center())
                    .child(th("STATUS").w(px(128.)).px(px(16.)).py(px(12.)))
                    .child(th("ACTIONS").w(px(144.)).px(px(24.)).py(px(12.)).flex().justify_end()),
            )
            .children(rows.iter().map(|d| {
                let h = health(d.replicas, d.available_replicas);
                let color = health_color(h);
                let (row_store, info_store, name, info_name) = (store.clone(), store.clone(), d.name.clone(), d.name.clone());
                let cell = |w: f32| div().w(px(w)).px(px(16.)).py(px(12.)).t_sm();
                div()
                    .id(SharedString::from(format!("dep-row-{}", d.name)))
                    .test_support()
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(p.border_subtle)
                    .cursor_pointer()
                    .hover(|s| s.bg(p.bg3.opacity(0.5)))
                    .on_click(move |_, _, cx| {
                        let n = name.clone();
                        row_store.update(cx, |s, cx| {
                            s.set_deployment(Some(n), cx);
                            s.set_view(View::Pods, cx);
                        })
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .px(px(24.))
                            .py(px(12.))
                            .child(Icon::new(IconName::Layers).size_4().flex_none().text_color(p.muted))
                            .child(div().truncate().font_weight(FontWeight::MEDIUM).text_color(p.text1).child(d.name.clone())),
                    )
                    .child(cell(128.).flex().justify_center().text_color(p.text2).child(d.replicas.to_string()))
                    .child(cell(128.).flex().justify_center().text_color(color).child(d.available_replicas.to_string()))
                    .child(cell(128.).flex().justify_center().text_color(p.text2).child(d.ready_replicas.to_string()))
                    .child(
                        cell(128.)
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(crate::ui::badge::dot(color, 8.))
                            .child(
                                div()
                                    .id(SharedString::from(format!("dep-status-{}", d.name)))
                                    .test_support()
                                    .aria_label(h.label())
                                    .text_color(color)
                                    .child(h.label()),
                            ),
                    )
                    .child(
                        div().w(px(144.)).px(px(24.)).py(px(12.)).flex().justify_end().child(
                            Button::new(SharedString::from(format!("dep-info-{}", d.name)), Variant::Secondary)
                                .size(BtnSize::Sm)
                                .label("Info")
                                .on_click(move |_, _, cx| {
                                    cx.stop_propagation();
                                    let n = info_name.clone();
                                    info_store.update(cx, |s, cx| s.select_deployment_info(Some(n), cx))
                                }),
                        ),
                    )
            }));

        div()
            .size_full()
            .flex()
            .flex_col()
            .child(header)
            .child(
                div()
                    .id("deployments-body")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(if loading { table_skeleton(8, cx).into_any_element() } else { table.into_any_element() })
                    .when(!loading && rows.is_empty(), |d| {
                        d.child(
                            div()
                                .id("deployments-empty")
                                .test_support()
                                .flex()
                                .flex_col()
                                .items_center()
                                .justify_center()
                                .h(px(256.))
                                .text_color(p.muted)
                                .child(div().mb(px(16.)).opacity(0.5).child(Icon::new(IconName::Layers).size_12()))
                                .child(div().t_lg().child("No deployments found")),
                        )
                    }),
            )
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{AppContext, TestAppContext};

    use crate::app::store::{DrawerKind, View};
    use crate::app::testing::{boot, deployment, seeded};
    use crate::status::Health;
    use crate::ui::badge::health_color;
    use crate::ui::theme::{hex, tw};

    macro_rules! ui {
        ($cx:expr, $env:expr, |$w:ident, $c:ident| $body:expr) => {
            $cx.update_window($env.handle, |_, $w, $c| $body).unwrap()
        };
    }

    #[gpui_kit::test]
    fn deployment_row_click_navigates_to_pods(cx: &mut TestAppContext) {
        let env = boot(cx, seeded(vec![deployment("api", 1, 1, 1)]), None);
        ui!(cx, env, |w, c| {
            w.render_frame(c);
            w.click("dep-row-api", c);
        });
        assert_eq!(env.store.read_with(cx, |s, _| (s.deployment.clone(), s.view)), (Some("api".into()), View::Pods));
    }

    #[gpui_kit::test]
    fn deployment_info_opens_drawer_without_navigating(cx: &mut TestAppContext) {
        let env = boot(cx, seeded(vec![deployment("api", 1, 1, 1)]), None);
        ui!(cx, env, |w, c| {
            w.render_frame(c);
            w.click("dep-info-api", c);
        });
        let (d, dep, v) = env.store.read_with(cx, |s, _| (s.drawer.clone(), s.deployment.clone(), s.view));
        assert_eq!((d, dep, v), (Some(DrawerKind::Deployment("api".into())), None, View::Deployments));
    }

    #[gpui_kit::test]
    fn deployment_health_text_and_colors(cx: &mut TestAppContext) {
        let env = boot(cx, seeded(vec![deployment("a", 3, 3, 3), deployment("b", 3, 1, 1), deployment("c", 3, 0, 0)]), None);
        ui!(cx, env, |w, c| {
            w.render_frame(c);
            assert_eq!(w.find("dep-status-a").label(), Some("Healthy"));
            assert_eq!(w.find("dep-status-b").label(), Some("Degraded"));
            assert_eq!(w.find("dep-status-c").label(), Some("Unhealthy"));
        });
        assert!(health_color(Health::Healthy) == hex(tw::EMERALD_500));
        assert!(health_color(Health::Degraded) == hex(tw::AMBER_500));
        assert!(health_color(Health::Unhealthy) == hex(tw::RED_500));
    }
}
