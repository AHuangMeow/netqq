//! Catppuccin Mocha 主题：配色、字体与「厚边框」风格。

use egui::{
    Color32, Context, CornerRadius, FontData, FontFamily, FontId, Frame, Margin, Stroke, TextStyle,
    Vec2,
};

/// Catppuccin Mocha 调色板
pub mod mocha {
    use egui::Color32;

    pub const RED: Color32 = Color32::from_rgb(243, 139, 168);
    pub const MAROON: Color32 = Color32::from_rgb(235, 160, 172);
    pub const PEACH: Color32 = Color32::from_rgb(250, 179, 135);
    pub const YELLOW: Color32 = Color32::from_rgb(249, 226, 175);
    pub const GREEN: Color32 = Color32::from_rgb(166, 227, 161);
    pub const TEAL: Color32 = Color32::from_rgb(148, 226, 213);
    pub const SKY: Color32 = Color32::from_rgb(137, 220, 235);
    pub const SAPPHIRE: Color32 = Color32::from_rgb(116, 199, 236);
    pub const BLUE: Color32 = Color32::from_rgb(137, 180, 250);
    pub const LAVENDER: Color32 = Color32::from_rgb(180, 190, 254);
    pub const MAUVE: Color32 = Color32::from_rgb(203, 166, 247);
    pub const TEXT: Color32 = Color32::from_rgb(205, 214, 244);
    pub const SUBTEXT1: Color32 = Color32::from_rgb(186, 194, 222);
    pub const SUBTEXT0: Color32 = Color32::from_rgb(166, 173, 200);
    pub const OVERLAY1: Color32 = Color32::from_rgb(127, 132, 156);
    pub const OVERLAY0: Color32 = Color32::from_rgb(108, 112, 134);
    pub const SURFACE2: Color32 = Color32::from_rgb(88, 91, 112);
    pub const SURFACE1: Color32 = Color32::from_rgb(69, 71, 90);
    pub const SURFACE0: Color32 = Color32::from_rgb(49, 50, 68);
    pub const BASE: Color32 = Color32::from_rgb(30, 30, 46);
    pub const MANTLE: Color32 = Color32::from_rgb(24, 24, 37);
    pub const CRUST: Color32 = Color32::from_rgb(17, 17, 27);
}

/// 统一「厚边框」线宽
pub const BORDER: f32 = 2.5;

pub fn setup(ctx: &Context) {
    install_cjk(ctx);
    ctx.set_theme(egui::ThemePreference::Dark);
    let v = visuals();
    ctx.set_visuals_of(egui::Theme::Dark, v.clone());
    ctx.set_visuals_of(egui::Theme::Light, v);
    ctx.all_styles_mut(|s| {
        s.animation_time = 0.22; // 控件 hover / 展开的过渡时长
        s.spacing.item_spacing = Vec2::new(8.0, 8.0);
        s.spacing.button_padding = Vec2::new(10.0, 5.0);
        s.text_styles = [
            (TextStyle::Small, FontId::proportional(12.0)),
            (TextStyle::Body, FontId::proportional(15.0)),
            (TextStyle::Button, FontId::proportional(15.0)),
            (TextStyle::Monospace, FontId::monospace(13.5)),
            (TextStyle::Heading, FontId::proportional(20.0)),
        ]
        .into_iter()
        .collect();
    });
}

/// 顶栏 / 底栏容器：只铺底色，单边分隔线由 `Panel` 自己画
/// （颜色取自 `noninteractive.bg_stroke`，即 BORDER 粗的 SURFACE1）
pub fn bar() -> Frame {
    Frame::NONE
        .fill(mocha::MANTLE)
        .inner_margin(Margin::symmetric(14, 9))
}

fn visuals() -> egui::Visuals {
    use mocha::*;

    let mut v = egui::Visuals::dark();
    v.panel_fill = BASE;
    v.window_fill = BASE;
    v.faint_bg_color = MANTLE;
    v.extreme_bg_color = CRUST;
    v.window_stroke = Stroke::new(BORDER, SURFACE1);
    v.code_bg_color = CRUST;
    v.text_edit_bg_color = Some(CRUST);
    v.hyperlink_color = SAPPHIRE;
    v.warn_fg_color = YELLOW;
    v.error_fg_color = RED;
    v.selection = egui::style::Selection {
        bg_fill: Color32::from_rgba_unmultiplied(137, 180, 250, 70),
        stroke: Stroke::new(1.0, LAVENDER),
    };

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = MANTLE;
    w.noninteractive.weak_bg_fill = MANTLE;
    w.noninteractive.bg_stroke = Stroke::new(BORDER, SURFACE1);
    w.noninteractive.fg_stroke = Stroke::new(1.0, SUBTEXT1);
    w.noninteractive.corner_radius = CornerRadius::same(8);

    w.inactive.bg_fill = SURFACE0;
    w.inactive.weak_bg_fill = SURFACE0;
    w.inactive.bg_stroke = Stroke::new(BORDER, SURFACE2);
    w.inactive.fg_stroke = Stroke::new(1.2, TEXT);
    w.inactive.corner_radius = CornerRadius::same(8);

    w.hovered.bg_fill = SURFACE1;
    w.hovered.weak_bg_fill = SURFACE1;
    w.hovered.bg_stroke = Stroke::new(BORDER, OVERLAY0);
    w.hovered.fg_stroke = Stroke::new(1.4, TEXT);
    w.hovered.corner_radius = CornerRadius::same(8);

    w.active.bg_fill = SURFACE1;
    w.active.weak_bg_fill = SURFACE2;
    w.active.bg_stroke = Stroke::new(BORDER, BLUE);
    w.active.fg_stroke = Stroke::new(1.6, LAVENDER);
    w.active.corner_radius = CornerRadius::same(8);

    w.open.bg_fill = SURFACE1;
    w.open.weak_bg_fill = SURFACE1;
    w.open.bg_stroke = Stroke::new(BORDER, SURFACE2);
    w.open.fg_stroke = Stroke::new(1.4, TEXT);
    w.open.corner_radius = CornerRadius::same(8);

    v
}

/// 系统 CJK 字体候选：(路径, 比例字体 face 索引, 等宽字体 face 索引)
const CJK_FONTS: &[(&str, u32, u32)] = &[
    ("/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc", 2, 7), // Arch
    ("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", 2, 7), // Debian / Ubuntu
    ("/usr/share/fonts/wenquanyi/wqy-microhei/wqy-microhei.ttc", 0, 0),
    ("/System/Library/Fonts/PingFang.ttc", 0, 0), // macOS
    ("C:/Windows/Fonts/msyh.ttc", 0, 0),          // Windows 微软雅黑
];

/// 把系统 CJK 字体挂到内置字体后面作后备，否则中文显示为方块。
fn install_cjk(ctx: &Context) {
    let Some((path, prop, mono)) = CJK_FONTS.iter().find(|(p, ..)| std::path::Path::new(p).exists())
    else {
        return;
    };
    let Ok(bytes) = std::fs::read(path) else { return };
    // 只读一次，两个 face 共享同一份静态数据
    let bytes: &'static [u8] = Box::leak(bytes.into_boxed_slice());
    let face = |index| FontData {
        font: bytes.into(),
        index,
        tweak: Default::default(),
    };

    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("cjk".into(), std::sync::Arc::new(face(*prop)));
    fonts
        .font_data
        .insert("cjk-mono".into(), std::sync::Arc::new(face(*mono)));
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .push("cjk".into());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .push("cjk-mono".into());
    ctx.set_fonts(fonts);
}
