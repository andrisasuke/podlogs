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

use std::sync::Arc;

use gpui_kit::*;

use api::KubeApi;
use app::{menus, root::PodLogsApp, store::Store};
use ui::icons::AppAssets;
use ui::theme::{load_fonts, palette, set_palette};

fn main() {
    // One multi-thread tokio runtime for every kube-rs call; gpui has its own executor.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("tokio runtime");
    let api = Arc::new(KubeApi::new(runtime.handle().clone()));
    let persist_path = persist::default_path();
    let persisted = persist_path.as_deref().map(persist::load).unwrap_or_default();

    gpui_kit::application().with_assets(AppAssets).run(move |cx: &mut App| {
        gpui_kit::init(cx);
        load_fonts(cx);
        ui::init(cx);
        menus::install(cx);
        set_palette(palette(!ui::gallery::enabled() || ui::gallery::start_dark()), cx);
        cx.on_window_closed(|cx, _| cx.quit()).detach();

        let bounds = Bounds::centered(None, size(px(1400.), px(900.)), cx);
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Maximized(bounds)),
            window_min_size: Some(size(px(1000.), px(700.))),
            titlebar: Some(TitlebarOptions { title: Some("PodLogs".into()), ..Default::default() }),
            window_background: WindowBackgroundAppearance::Opaque,
            ..Default::default()
        };
        if ui::gallery::enabled() {
            gpui_kit::open_window(opts, cx, |window, cx| cx.new(|cx| ui::gallery::Gallery::new(window, cx))).map(drop)
        } else {
            gpui_kit::open_window(opts, cx, move |window, cx| {
                let store = cx.new(|cx| Store::new(api, persisted, persist_path, cx));
                cx.new(|cx| PodLogsApp::new(store, window, cx))
            })
            .map(drop)
        }
        .expect("open window");
        cx.activate(true);
    });
    drop(runtime);
}
