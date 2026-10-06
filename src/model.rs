//! 会话数据模型：一次「请求 → 响应」往返就是一条消息。

use egui::Color32;

use crate::theme::mocha;

/// 展示用截断上限：(字节, 行数)
pub const CLIP_COLLAPSED: (usize, usize) = (64 * 1024, 400);
pub const CLIP_EXPANDED: (usize, usize) = (1024 * 1024, 20_000);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl Method {
    pub const ALL: [Self; 7] = [
        Self::Get,
        Self::Post,
        Self::Put,
        Self::Patch,
        Self::Delete,
        Self::Head,
        Self::Options,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
        }
    }

    /// 请求气泡描边色（与响应状态色区分）
    pub fn color(self) -> Color32 {
        match self {
            Self::Get => mocha::GREEN,
            Self::Post => mocha::BLUE,
            Self::Put => mocha::PEACH,
            Self::Patch => mocha::YELLOW,
            Self::Delete => mocha::RED,
            Self::Head => mocha::MAUVE,
            Self::Options => mocha::TEAL,
        }
    }

    pub fn reqwest(self) -> reqwest::Method {
        match self {
            Self::Get => reqwest::Method::GET,
            Self::Post => reqwest::Method::POST,
            Self::Put => reqwest::Method::PUT,
            Self::Patch => reqwest::Method::PATCH,
            Self::Delete => reqwest::Method::DELETE,
            Self::Head => reqwest::Method::HEAD,
            Self::Options => reqwest::Method::OPTIONS,
        }
    }
}

/// 响应及其「原始报文」素材
pub struct Resp {
    pub status: u16,
    pub reason: String,
    pub version: String,
    pub headers: Vec<(String, String)>,
    /// UTF-8 lossy 解码一次，避免每帧重解码
    pub body_text: String,
    pub body_len: usize,
    pub binary: bool,
    pub final_url: String,
}

impl Resp {
    pub fn head(&self) -> String {
        let reason = if self.reason.is_empty() {
            String::new()
        } else {
            format!(" {}", self.reason)
        };
        let mut out = format!("{} {}{}\n", self.version, self.status, reason);
        for (k, v) in &self.headers {
            out += &format!("{k}: {v}\n");
        }
        out.trim_end().to_owned()
    }
}

pub enum State {
    Pending,
    Ok(Resp),
    Err(String),
}

/// 一条聊天记录
pub struct Exchange {
    pub id: u64,
    pub method: Method,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    /// 请求行 + 请求头（含补出的 Host / Content-Length）
    pub head: String,
    pub stamp: String,
    pub t0: f64,
    pub t1: Option<f64>,
    pub elapsed_ms: Option<u64>,
    pub state: State,
}

impl Exchange {
    pub fn request_text(&self) -> String {
        join_raw(&self.head, &self.body)
    }

    /// 尚无响应时为 None
    pub fn response_text(&self) -> Option<String> {
        match &self.state {
            State::Pending => None,
            State::Ok(r) => Some(join_raw(&r.head(), &r.body_text)),
            State::Err(e) => Some(e.clone()),
        }
    }
}

fn join_raw(head: &str, body: &str) -> String {
    if body.is_empty() {
        head.to_owned()
    } else {
        format!("{head}\n\n{body}")
    }
}

pub fn status_color(status: u16) -> Color32 {
    match status {
        200..=299 => mocha::GREEN,
        300..=399 => mocha::SKY,
        400..=499 => mocha::PEACH,
        _ => mocha::RED,
    }
}

pub fn pretty_size(n: usize) -> String {
    const K: f64 = 1024.0;
    let n = n as f64;
    if n < K {
        format!("{n:.0} B")
    } else if n < K * K {
        format!("{:.1} KB", n / K)
    } else {
        format!("{:.1} MB", n / K / K)
    }
}

/// 没写协议就补 http://
pub fn normalize_url(raw: &str) -> String {
    let s = raw.trim();
    if s.contains("://") {
        s.to_owned()
    } else {
        format!("http://{s}")
    }
}

/// 组装「原始请求」的请求行与请求头，自动补 Host / Content-Length（用户已写则不重复）。
pub fn request_head(
    method: Method,
    url: &reqwest::Url,
    headers: &[(String, String)],
    body: &str,
) -> String {
    let mut path = url.path().to_owned();
    if let Some(q) = url.query() {
        path.push('?');
        path.push_str(q);
    }
    let has = |name: &str| headers.iter().any(|(k, _)| k.eq_ignore_ascii_case(name));

    let mut out = format!("{} {} HTTP/1.1\n", method.as_str(), path);
    if !has("host") {
        match url.port() {
            Some(p) => out += &format!("Host: {}:{p}\n", url.host_str().unwrap_or("")),
            None => out += &format!("Host: {}\n", url.host_str().unwrap_or("")),
        }
    }
    for (k, v) in headers {
        out += &format!("{k}: {v}\n");
    }
    if !body.is_empty() && !has("content-length") && !has("transfer-encoding") {
        out += &format!("Content-Length: {}\n", body.len());
    }
    out.trim_end().to_owned()
}

/// 展示用截断：先按字节上限、再按行数上限裁剪，保证落在字符边界。
pub fn clip(text: &str, (max_bytes, max_lines): (usize, usize)) -> (&str, bool) {
    let mut end = text.len();
    if end > max_bytes {
        end = max_bytes;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
    }
    let mut lines = 1;
    for (i, b) in text.as_bytes()[..end].iter().enumerate() {
        if *b == b'\n' {
            lines += 1;
            if lines > max_lines {
                end = i;
                break;
            }
        }
    }
    (&text[..end], end < text.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_keeps_char_boundary() {
        let text = "中文".repeat(10);
        let (cut, truncated) = clip(&text, (7, 100));
        assert!(truncated);
        assert_eq!(cut, "中文");
    }

    #[test]
    fn clip_by_lines() {
        let (cut, truncated) = clip("a\nb\nc\nd", (1000, 2));
        assert!(truncated);
        assert_eq!(cut, "a\nb");
    }

    #[test]
    fn head_adds_host_and_length() {
        let url = reqwest::Url::parse("http://example.com:8080/a/b?q=1").unwrap();
        let head = request_head(Method::Post, &url, &[("X-A".into(), "1".into())], "hi");
        assert_eq!(
            head,
            "POST /a/b?q=1 HTTP/1.1\nHost: example.com:8080\nX-A: 1\nContent-Length: 2"
        );
    }

    #[test]
    fn normalize() {
        assert_eq!(normalize_url(" example.com "), "http://example.com");
        assert_eq!(normalize_url("https://a.b"), "https://a.b");
    }
}
