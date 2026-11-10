//! Palette (exact `src/index.css` values), Tailwind colors, text-size helpers and fonts.
use std::borrow::Cow;

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::*;

pub fn hex(c: u32) -> Hsla {
    rgb(c).into()
}

pub fn hexa(c: u32, alpha: f32) -> Hsla {
    let mut h = hex(c);
    h.a = alpha;
    h
}

/// Mirrors the CSS variables in `src/index.css` (light = Solarized, dark = Slate).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub dark: bool,
    pub bg1: Hsla,
    pub bg2: Hsla,
    pub bg3: Hsla,
    pub text1: Hsla,
    pub text2: Hsla,
    pub muted: Hsla,
    pub border: Hsla,
    pub border_subtle: Hsla,
    pub accent_subtle: Hsla,
}

pub fn palette(dark: bool) -> Palette {
    if dark {
        Palette {
            dark,
            bg1: hex(0x0F172A),
            bg2: hex(0x1E293B),
            bg3: hex(0x334155),
            text1: hex(0xF8FAFC),
            text2: hex(0xCBD5E1),
            muted: hex(0x64748B),
            border: hex(0x334155),
            border_subtle: hex(0x1E293B),
            accent_subtle: hex(0x0C4A6E),
        }
    } else {
        Palette {
            dark,
            bg1: hex(0xFDF6E3),
            bg2: hex(0xEEE8D5),
            bg3: hex(0xE6DFC6),
            text1: hex(0x073642),
            text2: hex(0x586E75),
            muted: hex(0x93A1A1),
            border: hex(0xD3CDB8),
            border_subtle: hex(0xE6DFC6),
            accent_subtle: hex(0xD4E9E2),
        }
    }
}

/// Tailwind v3 palette values used by the components.
pub mod tw {
    pub const EMERALD_400: u32 = 0x34D399;
    pub const EMERALD_500: u32 = 0x10B981;
    pub const EMERALD_600: u32 = 0x059669;
    pub const AMBER_400: u32 = 0xFBBF24;
    pub const AMBER_500: u32 = 0xF59E0B;
    pub const AMBER_600: u32 = 0xD97706;
    pub const RED_400: u32 = 0xF87171;
    pub const RED_500: u32 = 0xEF4444;
    pub const RED_600: u32 = 0xDC2626;
    pub const PURPLE_400: u32 = 0xC084FC;
    pub const PURPLE_500: u32 = 0xA855F7;
    pub const PURPLE_600: u32 = 0x9333EA;
    pub const GRAY_400: u32 = 0x9CA3AF;
    pub const GRAY_500: u32 = 0x6B7280;
    pub const BLUE_500: u32 = 0x3B82F6;
    pub const CYAN_400: u32 = 0x22D3EE;
    pub const CYAN_500: u32 = 0x06B6D4;
    pub const ACCENT: u32 = 0x0EA5E9;
    pub const ACCENT_HOVER: u32 = 0x0284C7;
}

pub const UI_FONT: &str = "IBM Plex Sans";
pub const MONO_FONT: &str = "JetBrains Mono";

/// Tailwind `text-*` sets size AND line-height; gpui's `text_*` only sets size.
pub trait Tw: Styled + Sized {
    fn t_xs(self) -> Self {
        self.text_size(px(12.)).line_height(px(16.))
    }
    fn t_sm(self) -> Self {
        self.text_size(px(14.)).line_height(px(20.))
    }
    fn t_base(self) -> Self {
        self.text_size(px(16.)).line_height(px(24.))
    }
    fn t_lg(self) -> Self {
        self.text_size(px(18.)).line_height(px(28.))
    }
    fn mono(self) -> Self {
        self.font_family(MONO_FONT)
    }
}
impl<T: Styled> Tw for T {}

pub struct CurrentPalette(pub Palette);
impl Global for CurrentPalette {}

pub fn pal(cx: &App) -> Palette {
    cx.global::<CurrentPalette>().0
}

/// Sets the palette global and pushes it into gpui-component's `Theme` so its widgets match.
pub fn set_palette(p: Palette, cx: &mut App) {
    cx.set_global(CurrentPalette(p));
    Theme::change(if p.dark { ThemeMode::Dark } else { ThemeMode::Light }, None, cx);
    let t = Theme::global_mut(cx);
    t.font_family = UI_FONT.into();
    t.font_size = px(14.);
    t.mono_font_family = MONO_FONT.into();
    t.radius = px(6.);
    let c = &mut t.colors;
    c.background = p.bg1;
    c.foreground = p.text1;
    c.muted_foreground = p.muted;
    c.border = p.border;
    c.input = p.border;
    c.ring = hex(tw::ACCENT);
    c.caret = p.text1;
    c.selection = hexa(tw::ACCENT, 0.3);
    cx.refresh_windows();
}

pub fn load_fonts(cx: &mut App) {
    cx.text_system()
        .add_fonts(vec![
            Cow::Borrowed(include_bytes!("../../assets/fonts/IBMPlexSans-Regular.ttf").as_slice()),
            Cow::Borrowed(include_bytes!("../../assets/fonts/IBMPlexSans-Medium.ttf").as_slice()),
            Cow::Borrowed(include_bytes!("../../assets/fonts/IBMPlexSans-SemiBold.ttf").as_slice()),
            Cow::Borrowed(include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf").as_slice()),
            Cow::Borrowed(include_bytes!("../../assets/fonts/JetBrainsMono-Medium.ttf").as_slice()),
        ])
        .expect("embedded fonts are valid");
}

#[cfg(test)]
mod tests {
    use super::{hex, palette}; // not `*`: gpui re-exports a `test` macro that shadows #[test]

    #[test]
    fn palette_matches_index_css() {
        assert!(palette(true).bg1 == hex(0x0F172A));
        assert!(palette(false).bg1 == hex(0xFDF6E3));
        assert!(palette(false).border == hex(0xD3CDB8));
        assert!(palette(true).accent_subtle == hex(0x0C4A6E));
    }
}
