//! Pods table, matching `PodList.tsx` at the parity baseline.
use gpui_kit::assets::IconName;
use gpui_kit::component::{
    Icon,
    input::{InputEvent, InputState},
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::store::{Store, View};
use crate::models::PodInfo;
use crate::ui::badge::status_badge;
use crate::ui::button::{BtnSize, Button, Variant};
use crate::ui::input::search_input;
use crate::ui::section_label;
use crate::ui::skeleton::table_skeleton;
use crate::ui::theme::{MONO_FONT, Tw, UI_FONT, hex, pal, palette, tw};

pub struct PodsView {
    store: Entity<Store>,
    pub(crate) search: Entity<InputState>,
    active: bool,
    _subs: Vec<Subscription>,
}

pub fn actions_disabled(status: &str) -> bool {
    matches!(status, "Terminating" | "ContainerCreating" | "Evicted")
}

pub fn restarts_color(restarts: i32, dark: bool) -> Hsla {
    if restarts > 0 { hex(if dark { tw::RED_400 } else { tw::RED_500 }) } else { palette(dark).text2 }
}

/// Explicit widths keep the sticky header aligned with the scrollable rows.
/// Measure real values so long statuses and IPv6 addresses cannot overlap Actions.
#[derive(Clone, Copy)]
struct Columns {
    status: f32,
    ready: f32,
    restarts: f32,
    age: f32,
    ip: f32,
    actions: f32,
}

impl Columns {
    fn measure(pods: &[PodInfo], window: &mut Window) -> Self {
        let width = |text: &str, family: &'static str, size: f32, weight: FontWeight| {
            let mut run = window.text_style().to_run(text.len());
            run.font = font(family);
            run.font.weight = weight;
            f32::from(window.text_system().shape_line(text.to_owned().into(), px(size), &[run], None).width).ceil()
        };
        let ui = |text: &str| width(text, UI_FONT, 14., FontWeight::NORMAL);
        let mut c = Self {
            status: 112.,
            ready: 80.,
            restarts: 96.,
            age: 80.,
            ip: 112.,
            // Button padding 48 + secondary border 2 + gap 8 + cell padding 48.
            actions: width("Logs", UI_FONT, 14., FontWeight::MEDIUM) + width("Info", UI_FONT, 14., FontWeight::MEDIUM) + 48. + 2. + 8. + 48.,
        };
        for pod in pods {
            c.status = c.status.max(width(&pod.status, UI_FONT, 12., FontWeight::MEDIUM) + 20. + 6. + 6. + 32.);
            c.ready = c.ready.max(ui(&pod.ready) + 32.);
            c.restarts = c.restarts.max(ui(&pod.restarts.to_string()) + 32.);
            c.age = c.age.max(ui(&pod.age) + 32.);
            c.ip = c.ip.max(width(if pod.ip.is_empty() { "-" } else { &pod.ip }, MONO_FONT, 14., FontWeight::NORMAL) + 32.);
        }
        c
    }

    fn min_width(self) -> f32 {
        200. + self.status + self.ready + self.restarts + self.age + self.ip + self.actions
    }
}

impl PodsView {
    pub fn new(store: Entity<Store>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search pods..."));
        let subs = vec![
            cx.observe_in(&store, window, |this, _, window, cx| {
                let active = this.store.read(cx).view == View::Pods;
                if active && !this.active {
                    this.search.update(cx, |s, cx| s.set_value("", window, cx));
                }
                this.active = active;
                cx.notify();
            }),
            cx.subscribe(&search, |_, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            }),
        ];
        let active = store.read(cx).view == View::Pods;
        Self { store, search, active, _subs: subs }
    }

    fn row(&self, pod: &PodInfo, c: Columns, cx: &App) -> AnyElement {
        let p = pal(cx);
        let mut hover = p.bg3;
        hover.a = 0.5;
        let disabled = actions_disabled(&pod.status);
        let logs_store = self.store.clone();
        let info_store = self.store.clone();
        let logs_name = pod.name.clone();
        let info_name = pod.name.clone();
        div()
            .id(SharedString::from(format!("pod-row-{}", pod.name)))
            .test_support()
            .flex()
            .items_center()
            .w_full()
            .border_b_1()
            .border_color(p.border_subtle)
            .hover(move |s| s.bg(hover))
            .child(div().flex_1().min_w(px(200.)).px(px(24.)).py(px(12.)).mono().t_sm().text_color(p.text1).child(pod.name.clone()))
            .child(cell(c.status).child(status_badge(&pod.status, Some(&pod.ready), cx)))
            .child(cell(c.ready).text_center().text_color(p.text2).child(pod.ready.clone()))
            .child(cell(c.restarts).text_center().text_color(restarts_color(pod.restarts, p.dark)).child(pod.restarts.to_string()))
            .child(cell(c.age).text_right().text_color(p.text2).child(pod.age.clone()))
            .child(cell(c.ip).text_right().mono().text_color(p.text2).child(if pod.ip.is_empty() { "-".into() } else { pod.ip.clone() }))
            .child(
                div()
                    .w(px(c.actions))
                    .flex_none()
                    .px(px(24.))
                    .py(px(12.))
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        Button::new(SharedString::from(format!("pod-logs-{}", pod.name)), Variant::Primary)
                            .size(BtnSize::Sm)
                            .label("Logs")
                            .disabled(disabled)
                            .on_click(move |_, _, cx| logs_store.update(cx, |s, cx| s.open_log_viewer(logs_name.clone(), cx))),
                    )
                    .child(
                        Button::new(SharedString::from(format!("pod-info-{}", pod.name)), Variant::Secondary)
                            .size(BtnSize::Sm)
                            .label("Info")
                            .disabled(disabled)
                            .on_click(move |_, _, cx| info_store.update(cx, |s, cx| s.select_pod(Some(info_name.clone()), cx))),
                    ),
            )
            .into_any_element()
    }
}

fn cell(width: f32) -> Div {
    div().w(px(width)).flex_none().px(px(16.)).py(px(12.)).t_sm()
}

impl Render for PodsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = pal(cx);
        let s = self.store.read(cx);
        let loading = s.pods.is_loading();
        let pods = s.pods.data().map(Vec::as_slice).unwrap_or_default();
        let c = Columns::measure(pods, window);
        let query = self.search.read(cx).value().to_lowercase();
        let filtered: Vec<_> = pods.iter().filter(|pod| pod.name.to_lowercase().contains(&query)).collect();
        let settings = self.store.clone();
        let header = div()
            .flex()
            .items_center()
            .w_full()
            .flex_none()
            .bg(p.bg1)
            .border_b_1()
            .border_color(p.border)
            .child(div().flex_1().min_w(px(200.)).px(px(24.)).py(px(12.)).child(section_label("NAME", cx)))
            .children(
                [
                    ("STATUS", c.status, 0),
                    ("READY", c.ready, 1),
                    ("RESTARTS", c.restarts, 1),
                    ("AGE", c.age, 2),
                    ("IP", c.ip, 2),
                    ("ACTIONS", c.actions, 2),
                ]
                .into_iter()
                .map(|(label, width, align)| {
                    div()
                        .w(px(width))
                        .flex_none()
                        .px(px(if label == "ACTIONS" { 24. } else { 16. }))
                        .py(px(12.))
                        .flex()
                        .when(align == 1, |d| d.justify_center())
                        .when(align == 2, |d| d.justify_end())
                        .child(section_label(label, cx).into_any_element())
                }),
            );
        div()
            .id("pods-view")
            .test_support()
            .size_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(16.))
                    .px(px(24.))
                    .py(px(16.))
                    .border_b_1()
                    .border_color(p.border)
                    .child(
                        div()
                            .flex()
                            .min_w_0()
                            .items_center()
                            .gap(px(8.))
                            .t_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(div().text_color(p.text1).child("Pods"))
                            .when_some(s.deployment.clone(), |d, name| d.child(div().text_color(hex(tw::ACCENT)).child(name))),
                    )
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(div().w(px(256.)).child(search_input(&self.search, cx)))
                            .child(
                                Button::new("pods-settings", Variant::Ghost)
                                    .size(BtnSize::Icon)
                                    .icon(IconName::Settings)
                                    .tooltip("Settings")
                                    .on_click(move |_, _, cx| settings.update(cx, |s, cx| s.open_settings(cx))),
                            ),
                    ),
            )
            .child(div().id("pods-table").flex_1().min_h_0().overflow_x_scroll().flex().flex_col().child(
                div().min_w(px(c.min_width())).flex_1().min_h_0().flex().flex_col().when(loading, |d| d.child(table_skeleton(8, cx))).when(
                    !loading,
                    |d| {
                        d.child(header).child(
                            div()
                                .id("pod-rows")
                                .flex_1()
                                .min_h_0()
                                .overflow_y_scroll()
                                .children(filtered.iter().map(|pod| self.row(pod, c, cx)))
                                .when(filtered.is_empty(), |d| {
                                    d.child(
                                        div()
                                            .id("pods-empty")
                                            .test_support()
                                            .h(px(256.))
                                            .flex()
                                            .flex_col()
                                            .items_center()
                                            .justify_center()
                                            .text_color(p.muted)
                                            .child(Icon::new(IconName::Search).size(px(48.)).mb(px(16.)).opacity(0.5))
                                            .child(div().t_lg().child("No pods found"))
                                            .when(!query.is_empty(), |d| {
                                                d.child(div().mt(px(4.)).t_sm().child("Try adjusting your search query"))
                                            }),
                                    )
                                }),
                        )
                    },
                ),
            ))
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{AppContext, TestAppContext};

    use super::{actions_disabled, restarts_color};
    use crate::app::store::View;
    use crate::app::testing::{boot, pod, with_pods};
    use crate::ui::theme::{hex, palette, tw};

    #[gpui_kit::test]
    fn pods_search_filters_case_insensitive(cx: &mut TestAppContext) {
        let env = boot(cx, with_pods(vec![pod("api-123", "Running"), pod("web-456", "Running")]), None);
        env.store.update(cx, |s, cx| s.set_view(View::Pods, cx));
        cx.run_until_parked();
        let search = env.app.read_with(cx, |app, cx| app.pods.read(cx).search.clone());
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            search.update(cx, |s, cx| s.focus(window, cx));
            window.input("API", cx);
            assert!(window.try_find("pod-row-api-123").is_some());
            assert!(window.try_find("pod-row-web-456").is_none());
            window.input("zzz", cx);
            assert!(window.try_find("pod-row-api-123").is_none());
            assert!(window.try_find("pods-empty").is_some());
        })
        .unwrap();
    }

    #[gpui_kit::test]
    fn logs_button_opens_log_viewer_for_pod(cx: &mut TestAppContext) {
        let api = with_pods(vec![pod("api-123", "Running")]);
        let _details = api.gate(crate::api::fake::Endpoint::PodDetails);
        let env = boot(cx, api, None);
        env.store.update(cx, |s, cx| s.set_view(View::Pods, cx));
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            window.click("pod-logs-api-123", cx);
        })
        .unwrap();
        env.store.read_with(cx, |s, _| {
            assert_eq!(s.view, View::Logs);
            assert_eq!(s.log_viewer_pod.as_deref(), Some("api-123"));
            assert_eq!(s.log_time_range, crate::format::TimeRange::M30);
        });
    }

    #[gpui_kit::test]
    fn disabled_actions_for_terminating_pod(cx: &mut TestAppContext) {
        let env = boot(
            cx,
            with_pods(vec![pod("terminating", "Terminating"), pod("creating", "ContainerCreating"), pod("evicted", "Evicted")]),
            None,
        );
        env.store.update(cx, |s, cx| s.set_view(View::Pods, cx));
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            for name in ["terminating", "creating", "evicted"] {
                for action in ["logs", "info"] {
                    window.click(gpui_kit::SharedString::from(format!("pod-{action}-{name}")), cx);
                }
            }
        })
        .unwrap();
        env.store.read_with(cx, |s, _| {
            assert_eq!(s.view, View::Pods);
            assert_eq!(s.drawer, None);
            assert_eq!(s.log_viewer_pod, None);
        });
    }

    #[gpui_kit::test]
    fn pods_settings_button_opens_drawer(cx: &mut TestAppContext) {
        let env = boot(cx, with_pods(vec![]), None);
        env.store.update(cx, |s, cx| s.set_view(View::Pods, cx));
        cx.update_window(env.handle, |_, window, cx| {
            window.render_frame(cx);
            window.click("pods-settings", cx);
        })
        .unwrap();
        assert_eq!(env.store.read_with(cx, |s, _| s.drawer.clone()), Some(crate::app::store::DrawerKind::Settings));
    }

    #[test]
    fn restarts_colored_only_when_positive() {
        for dark in [false, true] {
            assert_eq!(restarts_color(0, dark), palette(dark).text2);
            assert_eq!(restarts_color(-1, dark), palette(dark).text2);
            assert_eq!(restarts_color(1, dark), hex(if dark { tw::RED_400 } else { tw::RED_500 }));
        }
    }

    #[test]
    fn only_three_statuses_disable_actions() {
        for status in ["Terminating", "ContainerCreating", "Evicted"] {
            assert!(actions_disabled(status));
        }
        for status in ["Running", "Pending", "CrashLoopBackOff", "ContainerStatusUnknown"] {
            assert!(!actions_disabled(status));
        }
    }
}
