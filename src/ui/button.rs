//! `Button.tsx`: primary/secondary/ghost × sm/md/lg/icon; disabled = 50 % opacity, no hover/click/tooltip.
use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, tooltip::Tooltip};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::theme::{Tw, hex, pal, tw};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Variant {
    Primary,
    Secondary,
    Ghost,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BtnSize {
    Sm,
    Md,
    Lg,
    Icon,
}

type ClickFn = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    variant: Variant,
    size: BtnSize,
    icon: Option<IconName>,
    icon_gap: f32,
    label: Option<SharedString>,
    disabled: bool,
    tooltip: Option<&'static str>,
    on_click: Option<ClickFn>,
}

impl Button {
    pub fn new(id: impl Into<ElementId>, variant: Variant) -> Self {
        Self {
            id: id.into(),
            variant,
            size: BtnSize::Md,
            icon: None,
            icon_gap: 4.,
            label: None,
            disabled: false,
            tooltip: None,
            on_click: None,
        }
    }
    pub fn size(mut self, s: BtnSize) -> Self {
        self.size = s;
        self
    }
    pub fn icon(mut self, i: IconName) -> Self {
        self.icon = Some(i);
        self
    }
    /// Gap after the icon in px (`mr-1` = 4 default; `mr-2` = 8 for the Search button).
    pub fn icon_gap(mut self, px: f32) -> Self {
        self.icon_gap = px;
        self
    }
    pub fn label(mut self, l: impl Into<SharedString>) -> Self {
        self.label = Some(l.into());
        self
    }
    pub fn disabled(mut self, d: bool) -> Self {
        self.disabled = d;
        self
    }
    pub fn tooltip(mut self, t: &'static str) -> Self {
        self.tooltip = Some(t);
        self
    }
    pub fn on_click(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(f));
        self
    }
}

impl RenderOnce for Button {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = pal(cx);
        let (variant, disabled) = (self.variant, self.disabled);
        let base = div()
            .id(self.id)
            .test_support()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .font_weight(FontWeight::MEDIUM)
            .rounded(px(6.))
            .map(|b| match self.size {
                BtnSize::Sm => b.h(px(32.)).px(px(12.)).t_sm(),
                BtnSize::Md => b.h(px(36.)).px(px(16.)).t_sm(),
                BtnSize::Lg => b.h(px(40.)).px(px(24.)).t_base(),
                BtnSize::Icon => b.size(px(32.)),
            })
            .map(|b| match variant {
                Variant::Primary => {
                    b.bg(hex(tw::ACCENT)).text_color(white()).when(!disabled, |b| b.hover(|s| s.bg(hex(tw::ACCENT_HOVER))))
                }
                Variant::Secondary => {
                    b.border_1().border_color(p.border).text_color(p.text2).when(!disabled, |b| b.hover(|s| s.bg(p.bg3)))
                }
                Variant::Ghost => b.text_color(p.text2).when(!disabled, |b| b.hover(|s| s.bg(p.bg3))),
            })
            .when_some(self.icon, |b, i| {
                b.child(Icon::new(i).size_4().when(self.label.is_some(), |i| i.mr(px(self.icon_gap))))
            })
            .children(self.label);
        if disabled {
            return base.opacity(0.5);
        }
        base.cursor_pointer()
            .when_some(self.tooltip, |b, t| b.tooltip(move |window, cx| Tooltip::new(t).build(window, cx)))
            .when_some(self.on_click, |b, f| b.on_click(move |e, w, cx| f(e, w, cx)))
    }
}
