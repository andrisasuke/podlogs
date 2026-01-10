mod error;
mod k8s;
mod models;

use k8s::{clusters, deployments, logs, namespaces, pods};
use tauri::{
    image::Image,
    menu::{AboutMetadata, Menu, PredefinedMenuItem, Submenu},
    Manager,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Get version from tauri.conf.json
            let version = app.package_info().version.to_string();

            // Load custom icon for About dialog
            let icon = Image::from_path(
                app.path()
                    .resource_dir()
                    .unwrap_or_default()
                    .join("icons/icon_256x256.png"),
            )
            .ok();

            // Create About metadata with custom icon
            // short_version = display version, version = build number (empty to hide)
            let about_metadata = AboutMetadata {
                name: Some("PodLogs".into()),
                short_version: Some(version),
                version: Some("".into()),
                icon,
                ..Default::default()
            };

            // Create custom app menu with our About metadata
            let app_submenu = Submenu::with_items(
                app,
                "PodLogs",
                true,
                &[
                    &PredefinedMenuItem::about(app, Some("About PodLogs"), Some(about_metadata))?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::services(app, Some("Services"))?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::hide(app, Some("Hide PodLogs"))?,
                    &PredefinedMenuItem::hide_others(app, Some("Hide Others"))?,
                    &PredefinedMenuItem::show_all(app, Some("Show All"))?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::quit(app, Some("Quit PodLogs"))?,
                ],
            )?;

            // Create Edit menu
            let edit_submenu = Submenu::with_items(
                app,
                "Edit",
                true,
                &[
                    &PredefinedMenuItem::undo(app, Some("Undo"))?,
                    &PredefinedMenuItem::redo(app, Some("Redo"))?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::cut(app, Some("Cut"))?,
                    &PredefinedMenuItem::copy(app, Some("Copy"))?,
                    &PredefinedMenuItem::paste(app, Some("Paste"))?,
                    &PredefinedMenuItem::select_all(app, Some("Select All"))?,
                ],
            )?;

            // Create Window menu
            let window_submenu = Submenu::with_items(
                app,
                "Window",
                true,
                &[
                    &PredefinedMenuItem::minimize(app, Some("Minimize"))?,
                    &PredefinedMenuItem::maximize(app, Some("Zoom"))?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::close_window(app, Some("Close"))?,
                ],
            )?;

            // Build the menu
            let menu = Menu::with_items(app, &[&app_submenu, &edit_submenu, &window_submenu])?;
            app.set_menu(menu)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            clusters::get_clusters,
            namespaces::get_namespaces,
            deployments::get_deployments,
            deployments::get_deployment_details,
            pods::get_pods,
            pods::get_pod_details,
            logs::get_pod_logs,
            logs::search_deployment_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
