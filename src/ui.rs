//! 中部会话区：像聊天一样展示请求与响应。

use std::collections::HashSet;

use egui::{
    Align, Button, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Sense,
    Stroke, Ui, Vec2,
};

use crate::app::App;
use crate::model::{clip, pretty_size, status_color, Exchange, Resp, State, CLIP_COLLAPSED, CLIP_EXPANDED};
use crate::theme::{mocha, BORDER};

/// 气泡入场动画时长（秒）
const ENTER: f64 = 0.34;

pub fn conversation(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let now = ctx.input(|i| i.time);
    let frame = Frame::NONE
        .fill(mocha::BASE)
        .inner_margin(Margin::symmetric(26, 10));
    egui::CentralPanel::default().frame(frame).show(ui, |ui| {
        ScrollArea::vertical()
            .stick_to_bottom(true)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.add_space(12.0);
                let App { log, expanded, .. } = &mut *app;
                let mut resend = None;
                for ex in log.iter() {
                    bubble(ui, ex, expanded, now, &mut resend);
                }
                ui.add_space(12.0);
                if let Some(id) = resend {
                    app.resend(&ctx, id);
                }
            });
    });

    // 有气泡正在入场，或请求在途（打字动画）时持续重绘
    if app.pending.is_some()
        || app
            .log
            .iter()
            .any(|e| now - e.t0 < ENTER || e.t1.is_some_and(|t| now - t < ENTER))
    {
        ctx.request_repaint();
    }
}

fn bubble(
    ui: &mut Ui,
    ex: &Exchange,
    expanded: &mut HashSet<u64>,
    now: f64,
    resend: &mut Option<u64>,
) {
    let max_w = (ui.available_width() * 0.74).clamp(220.0, 780.0);

    // —— 请求（靠右）——
    side(
        ui,
        true,
        enter(now, ex.t0),
        max_w,
        mocha::SURFACE0,
        ex.method.color(),
        CornerRadius {
            nw: 14,
            ne: 4,
            sw: 14,
            se: 14,
        },
        |ui| {
            let key = ex.id * 2;
            let (body, cut) = clip(&ex.body, limits(expanded.contains(&key)));
            raw(ui, &ex.head, ex.method.color(), body, mocha::TEXT);
            if cut {
                truncated_row(ui, expanded, key, ex.body.len());
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ghost(ui, "复制") {
                    ui.ctx().copy_text(ex.request_text());
                }
                if ghost(ui, "重发") {
                    *resend = Some(ex.id);
                }
                ui.label(RichText::new(&ex.stamp).size(11.5).color(mocha::OVERLAY0));
            });
        },
    );
    ui.add_space(6.0);

    // —— 响应（靠左）——
    match &ex.state {
        State::Pending => side(
            ui,
            false,
            enter(now, ex.t0),
            max_w,
            mocha::MANTLE,
            mocha::SURFACE1,
            rim(false),
            |ui| typing(ui, now),
        ),
        State::Err(err) => {
            let t = enter(now, ex.t1.unwrap_or(now));
            side(
                ui,
                false,
                t,
                max_w,
                mocha::MANTLE,
                mocha::RED,
                rim(false),
                |ui| {
                    raw(ui, "⚠ 请求失败", mocha::RED, err, mocha::MAROON);
                    footer(ui, ex, None);
                },
            );
        }
        State::Ok(resp) => {
            let t = enter(now, ex.t1.unwrap_or(now));
            let status = resp.status;
            side(
                ui,
                false,
                t,
                max_w,
                mocha::MANTLE,
                status_color(status),
                rim(false),
                |ui| {
                    let key = ex.id * 2 + 1;
                    let (body, cut) = clip(&resp.body_text, limits(expanded.contains(&key)));
                    raw(ui, &resp.head(), status_color(status), body, mocha::TEXT);
                    if resp.binary {
                        ui.label(
                            RichText::new("（非 UTF-8 正文，已按替换字符显示）")
                                .size(12.0)
                                .color(mocha::PEACH),
                        );
                    }
                    if cut {
                        truncated_row(ui, expanded, key, resp.body_len);
                    }
                    footer(ui, ex, Some(resp));
                },
            );
        }
    }
    ui.add_space(14.0);
}

/// 气泡骨架：入场位移 + 淡入 + 厚边框圆角 Frame
#[allow(clippy::too_many_arguments)]
fn side(
    ui: &mut Ui,
    right: bool,
    t: f32,
    max_w: f32,
    fill: Color32,
    stroke: Color32,
    radius: CornerRadius,
    content: impl FnOnce(&mut Ui),
) {
    let layout = if right {
        Layout::right_to_left(Align::Min)
    } else {
        Layout::left_to_right(Align::Min)
    };
    ui.with_layout(layout, |ui| {
        ui.add_space((1.0 - t) * 28.0);
        ui.set_opacity(t);
        Frame::NONE
            .fill(fill)
            .stroke(Stroke::new(BORDER, stroke))
            .corner_radius(radius)
            .inner_margin(Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.set_max_width(max_w);
                ui.with_layout(Layout::top_down(Align::Min), content);
            });
    });
}

fn rim(left: bool) -> CornerRadius {
    if left {
        CornerRadius {
            nw: 4,
            ne: 14,
            sw: 14,
            se: 14,
        }
    } else {
        CornerRadius {
            nw: 14,
            ne: 4,
            sw: 14,
            se: 14,
        }
    }
}

fn limits(open: bool) -> (usize, usize) {
    if open {
        CLIP_EXPANDED
    } else {
        CLIP_COLLAPSED
    }
}

/// 报文：首行（请求行 / 状态行）高亮，其余头信息压暗，正文正常
fn raw(ui: &mut Ui, head: &str, head_color: Color32, body: &str, body_color: Color32) {
    let (first, rest) = head.split_once('\n').unwrap_or((head, ""));
    ui.label(RichText::new(first).monospace().strong().color(head_color));
    if !rest.is_empty() {
        ui.label(RichText::new(rest).monospace().color(mocha::OVERLAY1));
    }
    if !body.is_empty() {
        ui.add_space(5.0);
        ui.label(RichText::new(body).monospace().color(body_color));
    }
}

/// 「服务器正在输入…」
fn typing(ui: &mut Ui, now: f64) {
    ui.horizontal(|ui| {
        for i in 0..3 {
            let k = ((now * 3.0 - i as f64 * 0.45).sin() * 0.5 + 0.5) as f32;
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
            let dot = mocha::OVERLAY1.lerp_to_gamma(mocha::LAVENDER, k);
            ui.painter().circle_filled(rect.center(), 3.5, dot);
        }
        ui.add_space(2.0);
        ui.label(
            RichText::new("服务器正在输入…")
                .size(13.0)
                .color(mocha::OVERLAY1),
        );
    });
}

fn truncated_row(ui: &mut Ui, expanded: &mut HashSet<u64>, key: u64, total: usize) {
    let open = expanded.contains(&key);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("… 已截断 · 全文 {}", pretty_size(total)))
                .size(12.0)
                .color(mocha::PEACH),
        );
        if ghost(ui, if open { "收起" } else { "展开全部" }) {
            if open {
                expanded.remove(&key);
            } else {
                expanded.insert(key);
            }
        }
    });
}

fn footer(ui: &mut Ui, ex: &Exchange, resp: Option<&Resp>) {
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if let Some(text) = ex.response_text()
            && ghost(ui, "复制")
        {
            ui.ctx().copy_text(text);
        }
        if let Some(ms) = ex.elapsed_ms {
            ui.label(RichText::new(format!("{ms} ms")).size(11.5).color(mocha::OVERLAY0));
        }
        if let Some(r) = resp {
            ui.label(RichText::new(pretty_size(r.body_len)).size(11.5).color(mocha::OVERLAY0));
        }
        ui.label(RichText::new(&ex.stamp).size(11.5).color(mocha::OVERLAY0));
    });
    if let Some(r) = resp
        && r.final_url != ex.url
    {
        ui.label(
            RichText::new(format!("↪ 重定向至 {}", r.final_url))
                .size(11.5)
                .color(mocha::SKY),
        );
    }
}

/// 小号「幽灵按钮」，点击返回 true
fn ghost(ui: &mut Ui, text: &str) -> bool {
    ui.add(
        Button::new(RichText::new(text).size(12.0).color(mocha::SUBTEXT0))
            .fill(mocha::SURFACE1)
            .stroke(Stroke::new(1.5, mocha::SURFACE2))
            .corner_radius(6)
            .small(),
    )
    .clicked()
}

/// 0→1 的入场进度（带缓出）
fn enter(now: f64, t0: f64) -> f32 {
    let k = ((now - t0) / ENTER).clamp(0.0, 1.0) as f32;
    egui::emath::easing::cubic_out(k)
}
