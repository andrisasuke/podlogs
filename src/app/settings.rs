//! `Settings.tsx`: Theme and Refresh Interval dropdowns.
use gpui_kit::*;

use super::store::Store;
use crate::format::{REFRESH_INTERVALS_MS, refresh_label};
use crate::persist::ThemePref;
use crate::ui::dropdown::{DropdownEvent, DropdownState};
use crate::ui::section_label;
use crate::ui::theme::{Tw, pal};

pub struct SettingsView {
    store: Entity<Store>,
    theme: Entity<DropdownState>,
    interval: Entity<DropdownState>,
    _subs: Vec<Subscription>,
}

impl SettingsView {
    pub fn new(store: Entity<Store>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let theme = cx.new(|cx| DropdownState::new("", None, window, cx));
        let interval = cx.new(|cx| DropdownState::new("", None, window, cx));
        let opt = |v: &str, l: &str| (v.to_string(), l.to_string());
        theme.update(cx, |d, cx| d.set_options(vec![opt("dark", "Dark"), opt("light", "Light"), opt("system", "System")], cx));
        interval.update(cx, |d, cx| {
            d.set_options(REFRESH_INTERVALS_MS.iter().map(|ms| (ms.to_string(), refresh_label(*ms))).collect(), cx)
        });
        let subs = vec![
            cx.observe(&store, |this, _, cx| this.sync(cx)),
            cx.subscribe(&theme, |this, _, ev: &DropdownEvent, cx| {
                let DropdownEvent::Change(v) = ev;
                let t = match v.as_str() {
                    "light" => ThemePref::Light,
                    "system" => ThemePref::System,
                    _ => ThemePref::Dark,
                };
                this.store.update(cx, |s, cx| s.set_theme(t, cx));
            }),
            cx.subscribe(&interval, |this, _, ev: &DropdownEvent, cx| {
                let DropdownEvent::Change(v) = ev;
                if let Ok(ms) = v.parse() {
                    this.store.update(cx, |s, cx| s.set_refresh_interval(ms, cx));
                }
            }),
        ];
        let mut this = Self { store, theme, interval, _subs: subs };
        this.sync(cx);
        this
    }

    fn sync(&mut self, cx: &mut Context<Self>) {
        let s = self.store.read(cx);
        let theme = match s.theme {
            ThemePref::Dark => "dark",
            ThemePref::Light => "light",
            ThemePref::System => "system",
        };
        let ms = s.refresh_interval_ms.to_string();
        self.theme.update(cx, |d, cx| d.set_value(Some(theme.into()), cx));
        self.interval.update(cx, |d, cx| d.set_value(Some(ms), cx));
    }
}

impl Render for SettingsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = pal(cx);
        let block = |id: &'static str, label: &'static str, dd: &Entity<DropdownState>, help: &'static str| {
            div()
                .child(div().mb(px(8.)).child(section_label(label, cx)))
                .child(div().id(id).test_support().child(dd.clone()))
                .child(div().mt(px(8.)).t_xs().text_color(p.muted).child(help))
        };
        div()
            .p(px(16.))
            .flex()
            .flex_col()
            .gap(px(24.))
            .child(block("settings-theme", "THEME", &self.theme, "Choose your preferred color theme"))
            .child(block("settings-interval", "REFRESH INTERVAL", &self.interval, "How often to refresh logs and pod status"))
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{AppContext, TestAppContext};

    use crate::app::testing::{boot, seeded, settle_animation};

    #[gpui_kit::test]
    fn settings_interval_change_updates_store_and_countdown(cx: &mut TestAppContext) {
        let env = boot(cx, seeded(vec![]), None);
        env.store.update(cx, |s, cx| s.open_settings(cx));
        cx.update_window(env.handle, |_, w, c| w.render_frame(c)).unwrap();
        settle_animation(cx);
        cx.update_window(env.handle, |_, w, c| {
            w.render_frame(c);
            w.within("settings-interval").click("dd-trigger", c);
            w.within("settings-interval").click("dd-opt-15000", c);
        })
        .unwrap();
        assert_eq!(env.store.read_with(cx, |s, _| (s.refresh_interval_ms, s.countdown_s)), (15000, 15));
    }
}
