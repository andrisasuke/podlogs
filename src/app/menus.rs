//! App menus (`src-tauri/src/lib.rs` at c70bd05): PodLogs / Edit / Window.
//! macOS: native menu bar. Elsewhere: the same menus in an in-window `AppMenuBar`, minus Services/Hide Others/Show All.
use gpui_kit::base::GlobalState;
use gpui_kit::component::{
    input::{Copy, Cut, Paste, Redo, SelectAll, Undo},
    menu::AppMenuBar,
};
use gpui_kit::*;

use super::root::show_about;

actions!(podlogs, [Quit, About, Hide, HideOthers, ShowAll, Minimize, Zoom, CloseWindow]);

fn menus() -> Vec<Menu> {
    let mac = cfg!(target_os = "macos");
    let mut app = vec![MenuItem::action("About PodLogs", About), MenuItem::separator()];
    if mac {
        app.extend([MenuItem::os_submenu("Services", SystemMenuType::Services), MenuItem::separator()]);
    }
    app.push(MenuItem::action("Hide PodLogs", Hide));
    if mac {
        app.extend([MenuItem::action("Hide Others", HideOthers), MenuItem::action("Show All", ShowAll)]);
    }
    app.extend([MenuItem::separator(), MenuItem::action("Quit PodLogs", Quit)]);
    vec![
        Menu::new("PodLogs").items(app),
        Menu::new("Edit").items([
            MenuItem::os_action("Undo", Undo, OsAction::Undo),
            MenuItem::os_action("Redo", Redo, OsAction::Redo),
            MenuItem::separator(),
            MenuItem::os_action("Cut", Cut, OsAction::Cut),
            MenuItem::os_action("Copy", Copy, OsAction::Copy),
            MenuItem::os_action("Paste", Paste, OsAction::Paste),
            MenuItem::os_action("Select All", SelectAll, OsAction::SelectAll),
        ]),
        Menu::new("Window").items([
            MenuItem::action("Minimize", Minimize),
            MenuItem::action("Zoom", Zoom),
            MenuItem::separator(),
            MenuItem::action("Close", CloseWindow),
        ]),
    ]
}

fn with_window(cx: &mut App, f: impl FnOnce(&mut Window) + 'static) {
    if let Some(handle) = cx.active_window() {
        let _ = handle.update(cx, |_, window, _| f(window));
    }
}

/// Registers the action handlers and key bindings and installs the menus (native on macOS,
/// otherwise stored for `menu_bar`). Call once, after `gpui_kit::init`.
pub fn install(cx: &mut App) {
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.on_action(|_: &About, cx| show_about(cx));
    cx.on_action(|_: &Hide, cx| cx.hide());
    cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
    cx.on_action(|_: &Minimize, cx| with_window(cx, |w| w.minimize_window()));
    cx.on_action(|_: &Zoom, cx| with_window(cx, |w| w.zoom_window()));
    cx.on_action(|_: &CloseWindow, cx| with_window(cx, |w| w.remove_window()));
    let keys: [(&'static str, &'static str, &'static str, &'static str, &'static str); 1] = if cfg!(target_os = "macos") {
        [("cmd-q", "cmd-h", "alt-cmd-h", "cmd-m", "cmd-w")]
    } else {
        [("ctrl-q", "ctrl-h", "ctrl-alt-h", "ctrl-m", "ctrl-w")]
    };
    let [(quit, hide, others, min, close)] = keys;
    cx.bind_keys([
        KeyBinding::new(quit, Quit, None),
        KeyBinding::new(hide, Hide, None),
        KeyBinding::new(others, HideOthers, None),
        KeyBinding::new(min, Minimize, None),
        KeyBinding::new(close, CloseWindow, None),
    ]);
    if cfg!(target_os = "macos") {
        cx.set_menus(menus());
    } else {
        GlobalState::global_mut(cx).set_app_menus(menus().into_iter().map(Menu::owned).collect());
    }
}

/// The in-window menu bar for Linux/Windows (macOS has the native one).
pub fn menu_bar(_window: &mut Window, cx: &mut App) -> Entity<AppMenuBar> {
    AppMenuBar::new(cx)
}
