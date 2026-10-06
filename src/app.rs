//! 应用状态与顶栏 / 底栏。

use std::collections::HashSet;
use std::sync::mpsc::{self, Receiver, Sender};

use egui::{
    Align, Button, ComboBox, Context, CornerRadius, Key, Layout, Margin, RichText, Spinner, Stroke,
    StrokeKind, TextEdit, TextStyle, Ui, Vec2,
};

use crate::http::{self, Done};
use crate::model::{self, Exchange, Method, State};
use crate::theme::{mocha, BORDER};
use crate::{theme, ui};

pub struct App {
    pub method: Method,
    pub url: String,
    pub headers: String,
    pub body: String,
    pub tab: Tab,
    pub log: Vec<Exchange>,
    pub expanded: HashSet<u64>,
    pub pending: Option<u64>,
    pub error: Option<String>,
    client: reqwest::blocking::Client,
    tx: Sender<Done>,
    rx: Receiver<Done>,
    next_id: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Headers,
    Body,
}

fn url_id() -> egui::Id {
    egui::Id::new("netqq.url")
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (tx, rx) = mpsc::channel();
        cc.egui_ctx.memory_mut(|m| m.request_focus(url_id()));
        Self {
            method: Method::Get,
            url: String::new(),
            headers: String::new(),
            body: String::new(),
            tab: Tab::Headers,
            log: Vec::new(),
            expanded: HashSet::new(),
            pending: None,
            error: None,
            client: http::client(),
            tx,
            rx,
            next_id: 1,
        }
    }

    /// 重新发送历史里的某条请求
    pub fn resend(&mut self, ctx: &Context, id: u64) {
        if self.pending.is_some() {
            return;
        }
        let Some(ex) = self.log.iter().find(|e| e.id == id) else {
            return;
        };
        let (url, method, headers, body) = (
            ex.url.clone(),
            ex.method,
            ex.headers.clone(),
            ex.body.clone(),
        );
        match reqwest::Url::parse(&url) {
            Ok(url) => self.start(ctx, method, url, headers, body),
            Err(_) => self.error = Some(format!("原始 URL 已无法解析：{url}")),
        }
    }

    fn send(&mut self, ctx: &Context) {
        if self.pending.is_some() {
            return;
        }
        if self.url.trim().is_empty() {
            self.error = Some("请先输入 URL".into());
            return;
        }
        let normalized = model::normalize_url(&self.url);
        let Ok(url) = reqwest::Url::parse(&normalized) else {
            self.error = Some(format!("URL 无效：{normalized}"));
            return;
        };
        let headers = match http::parse_headers(&self.headers) {
            Ok(h) => h,
            Err(e) => {
                self.error = Some(e);
                return;
            }
        };
        self.start(ctx, self.method, url, headers, self.body.clone());
    }

    fn start(
        &mut self,
        ctx: &Context,
        method: Method,
        url: reqwest::Url,
        headers: Vec<(String, String)>,
        body: String,
    ) {
        let id = self.next_id;
        self.next_id += 1;
        let head = model::request_head(method, &url, &headers, &body);
        let ex = Exchange {
            id,
            method,
            url: url.to_string(),
            headers,
            body,
            head,
            stamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            t0: ctx.input(|i| i.time),
            t1: None,
            elapsed_ms: None,
            state: State::Pending,
        };
        http::dispatch(&self.client, &self.tx, ctx, &ex);
        self.log.push(ex);
        self.pending = Some(id);
        self.error = None;
    }

    fn drain(&mut self, now: f64) {
        while let Ok(done) = self.rx.try_recv() {
            if let Some(ex) = self.log.iter_mut().find(|e| e.id == done.id) {
                ex.t1 = Some(now);
                ex.elapsed_ms = Some(done.elapsed_ms);
                ex.state = match done.result {
                    Ok(r) => State::Ok(r),
                    Err(e) => State::Err(e),
                };
            }
            if self.pending == Some(done.id) {
                self.pending = None;
            }
        }
    }

    fn top_bar(&mut self, ui: &mut Ui, ctx: &Context) {
        ui.spacing_mut().interact_size.y = 34.0; // 下拉框与 URL 框同高
        ui.horizontal(|ui| {
            ComboBox::from_id_salt("netqq.method")
                .width(104.0)
                .selected_text(
                    RichText::new(self.method.as_str())
                        .strong()
                        .color(self.method.color()),
                )
                .show_ui(ui, |ui| {
                    for m in Method::ALL {
                        ui.selectable_value(
                            &mut self.method,
                            m,
                            RichText::new(m.as_str()).color(m.color()),
                        );
                    }
                });

            let resp = ui.add_sized(
                [ui.available_width(), 34.0],
                TextEdit::singleline(&mut self.url)
                    .id(url_id())
                    .hint_text("https://example.com/api")
                    .margin(Margin::symmetric(10, 6)),
            );
            if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                self.send(ctx);
            }
            // 请求在途：URL 框外圈呼吸光晕
            if self.pending.is_some() {
                let k = ((ctx.input(|i| i.time) * 3.0).sin() as f32 + 1.0) * 0.5;
                ui.painter().rect_stroke(
                    resp.rect.expand(2.0),
                    CornerRadius::same(10),
                    Stroke::new(BORDER, mocha::SURFACE2.lerp_to_gamma(mocha::BLUE, k)),
                    StrokeKind::Outside,
                );
            }
        });
    }

    fn composer(&mut self, ui: &mut Ui, ctx: &Context) {
        ui.horizontal(|ui| {
            tab(ui, &mut self.tab, Tab::Headers, "Headers");
            tab(ui, &mut self.tab, Tab::Body, "Body");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                self.send_button(ui, ctx);
                if self.pending.is_some() {
                    ui.add(Spinner::new().size(16.0).color(mocha::LAVENDER));
                }
                if let Some(err) = &self.error {
                    ui.label(RichText::new(err).size(13.0).color(mocha::RED));
                }
            });
        });
        ui.add_space(6.0);

        egui::ScrollArea::vertical()
            .id_salt("netqq.composer")
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let edit = match self.tab {
                    Tab::Headers => TextEdit::multiline(&mut self.headers),
                    Tab::Body => TextEdit::multiline(&mut self.body),
                };
                ui.add(
                    edit.font(TextStyle::Monospace)
                        .desired_rows(6)
                        .desired_width(f32::INFINITY)
                        .margin(Margin::symmetric(10, 8)),
                );
            });
    }

    fn send_button(&mut self, ui: &mut Ui, ctx: &Context) {
        let pending = self.pending.is_some();
        let (fill, stroke, fg, label) = if pending {
            (
                mocha::SURFACE0,
                mocha::SURFACE2,
                mocha::OVERLAY1,
                "发送中…",
            )
        } else {
            (mocha::BLUE, mocha::LAVENDER, mocha::CRUST, "发送")
        };
        let button = Button::new(RichText::new(label).strong().color(fg))
            .fill(fill)
            .stroke(Stroke::new(BORDER, stroke))
            .corner_radius(10)
            .min_size(Vec2::new(108.0, 32.0));
        if ui.add_enabled(!pending, button).clicked() {
            self.send(ctx);
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let now = ctx.input(|i| i.time);
        self.drain(now);

        if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::Enter)) {
            self.send(&ctx);
        }

        egui::Panel::top("netqq.top")
            .frame(theme::bar())
            .show(ui, |ui| self.top_bar(ui, &ctx));
        egui::Panel::bottom("netqq.composer")
            .resizable(true)
            .default_size(232.0)
            .min_size(150.0)
            .max_size(520.0)
            .frame(theme::bar())
            .show(ui, |ui| self.composer(ui, &ctx));
        ui::conversation(self, ui);
    }
}

fn tab(ui: &mut Ui, current: &mut Tab, value: Tab, label: &str) {
    let on = *current == value;
    let (fill, stroke, fg) = if on {
        (mocha::SURFACE1, mocha::BLUE, mocha::LAVENDER)
    } else {
        (mocha::SURFACE0, mocha::SURFACE2, mocha::SUBTEXT0)
    };
    let button = Button::new(RichText::new(label).strong().color(fg))
        .fill(fill)
        .stroke(Stroke::new(BORDER, stroke))
        .corner_radius(9)
        .min_size(Vec2::new(92.0, 30.0));
    if ui.add(button).clicked() {
        *current = value;
    }
}
