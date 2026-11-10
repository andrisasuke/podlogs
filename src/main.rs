#![allow(dead_code)] // ponytail: most modules are wired up by later tasks

mod api;
mod app;
mod error;
mod format;
mod k8s;
mod models;
mod persist;
mod status;
mod ui;

use gpui_kit::assets::Assets;
use gpui_kit::*;

use ui::theme::{load_fonts, pal, palette, set_palette};

actions!(podlogs, [Quit]);

/// Empty root until the app shell lands (Task 9).
struct Root;

impl Render for Root {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().bg(pal(cx).bg1)
    }
}

fn main() {
    gpui_kit::application().with_assets(Assets).run(|cx: &mut App| {
        gpui_kit::init(cx);
        load_fonts(cx);
        set_palette(palette(true), cx);

        cx.on_action(|_: &Quit, cx| cx.quit());
        let quit_key = if cfg!(target_os = "macos") { "cmd-q" } else { "ctrl-q" };
        cx.bind_keys([KeyBinding::new(quit_key, Quit, None)]);
        cx.on_window_closed(|cx, _| cx.quit()).detach();

        let bounds = Bounds::centered(None, size(px(1400.), px(900.)), cx);
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Maximized(bounds)),
                window_min_size: Some(size(px(1000.), px(700.))),
                titlebar: Some(TitlebarOptions { title: Some("PodLogs".into()), ..Default::default() }),
                window_background: WindowBackgroundAppearance::Opaque,
                ..Default::default()
            },
            cx,
            |_, cx| cx.new(|_| Root),
        )
        .expect("open window");
        cx.activate(true);
    });
}
