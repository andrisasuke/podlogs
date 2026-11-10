//! `Input.tsx` on top of gpui-component's `Input`. Autocorrect/capitalize are off by default in gpui-base.
use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, input::{Input, InputState}};
use gpui_kit::*;

use super::theme::{Tw, pal};

pub fn text_input(state: &Entity<InputState>, cx: &App) -> Input {
    let p = pal(cx);
    Input::new(state).h(px(36.)).t_sm().rounded(px(6.)).bg(p.bg3).border_color(p.border)
}

pub fn search_input(state: &Entity<InputState>, cx: &App) -> Input {
    text_input(state, cx).prefix(Icon::new(IconName::Search).size_4().text_color(pal(cx).muted))
}
