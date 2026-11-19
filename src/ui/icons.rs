//! Icon set (Lucide via gpui-kit) plus the app's own PNGs, as one `AssetSource`.
use std::borrow::Cow;

use gpui_kit::assets::{Assets, icon_assets};
use gpui_kit::{AssetSource, Result, SharedString};

icon_assets!(pub AppIcons, [
    Layers, Search, Moon, Sun, Settings, ChevronDown, Check, ArrowLeft, Download, Copy, X,
    TriangleAlert, RefreshCw, Package, PanelLeftClose, PanelLeftOpen,
]);

const ICON: &[u8] = include_bytes!("../../assets/icon.png");
const ICON_256: &[u8] = include_bytes!("../../assets/icon_256x256.png");

/// `AppIcons` first, then gpui-kit's default bundle (gpui-component widgets load their own icons from it).
pub struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        match path {
            "assets/icon.png" => Ok(Some(Cow::Borrowed(ICON))),
            "assets/icon_256x256.png" => Ok(Some(Cow::Borrowed(ICON_256))),
            _ => match AppIcons.load(path)? {
                Some(b) => Ok(Some(b)),
                None => Assets.load(path),
            },
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut v = AppIcons.list(path)?;
        v.extend(Assets.list(path)?);
        for p in ["assets/icon.png", "assets/icon_256x256.png"] {
            if p.starts_with(path) {
                v.push(p.into());
            }
        }
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::assets::IconName;

    #[test]
    fn serves_icons_and_pngs() {
        assert!(AppAssets.load(&IconName::Check.path()).unwrap().is_some());
        assert!(AppAssets.load(&IconName::TriangleAlert.path()).unwrap().is_some());
        assert!(AppAssets.load("assets/icon.png").unwrap().is_some());
        assert!(AppAssets.load("assets/icon_256x256.png").unwrap().is_some());
    }
}
