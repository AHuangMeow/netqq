//! 在后台线程发请求，结果经 channel 回传 UI 线程。

use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use egui::Context;
use reqwest::blocking::Client;

use crate::model::{Exchange, Resp};

pub struct Done {
    pub id: u64,
    pub elapsed_ms: u64,
    pub result: Result<Resp, String>,
}

pub fn client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .expect("无法创建 HTTP 客户端")
}

pub fn dispatch(client: &Client, tx: &Sender<Done>, ctx: &Context, ex: &Exchange) {
    let (client, tx, ctx) = (client.clone(), tx.clone(), ctx.clone());
    let id = ex.id;
    let (method, url, headers, body) = (
        ex.method,
        ex.url.clone(),
        ex.headers.clone(),
        ex.body.clone(),
    );
    std::thread::spawn(move || {
        let started = Instant::now();
        let result = run(client, method, url, headers, body);
        let elapsed_ms = started.elapsed().as_millis() as u64;
        let _ = tx.send(Done {
            id,
            elapsed_ms,
            result,
        });
        ctx.request_repaint();
    });
}

fn run(
    client: Client,
    method: crate::model::Method,
    url: String,
    headers: Vec<(String, String)>,
    body: String,
) -> Result<Resp, String> {
    let mut req = client.request(method.reqwest(), &url);
    for (k, v) in headers {
        req = req.header(k, v);
    }
    if !body.is_empty() {
        req = req.body(body);
    }

    let resp = req.send().map_err(friendly)?;
    let status = resp.status();
    let version = format!("{:?}", resp.version());
    let final_url = resp.url().to_string();
    let headers = resp
        .headers()
        .iter()
        .map(|(k, v)| {
            let v = v
                .to_str()
                .map(str::to_owned)
                .unwrap_or_else(|_| format!("<{} 字节二进制值>", v.len()));
            (k.as_str().to_owned(), v)
        })
        .collect();

    let bytes = resp.bytes().map_err(|e| format!("读取响应正文失败：{e}"))?;
    let binary = std::str::from_utf8(&bytes).is_err();

    Ok(Resp {
        status: status.as_u16(),
        reason: status.canonical_reason().unwrap_or("").to_owned(),
        version,
        headers,
        body_text: String::from_utf8_lossy(&bytes).into_owned(),
        body_len: bytes.len(),
        binary,
        final_url,
    })
}

fn friendly(e: reqwest::Error) -> String {
    let kind = if e.is_timeout() {
        "请求超时"
    } else if e.is_connect() {
        "连接失败"
    } else if e.is_redirect() {
        "重定向次数过多"
    } else if e.is_decode() {
        "响应解码失败"
    } else {
        "请求失败"
    };
    let root = std::error::Error::source(&e)
        .map(|mut s| {
            while let Some(next) = s.source() {
                s = next;
            }
            s.to_string()
        })
        .unwrap_or_default();
    if root.is_empty() {
        format!("{kind}：{e}")
    } else {
        format!("{kind}：{root}")
    }
}

/// 解析「每行一个 Header」的文本，跳过空行与 # 注释。
pub fn parse_headers(text: &str) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once(':') else {
            return Err(format!("Headers 第 {} 行缺少「:」：{line}", i + 1));
        };
        let (k, v) = (k.trim(), v.trim());
        if reqwest::header::HeaderName::from_bytes(k.as_bytes()).is_err() {
            return Err(format!("Headers 第 {} 行的名称无效：{k}", i + 1));
        }
        if reqwest::header::HeaderValue::from_str(v).is_err() {
            return Err(format!("Headers 第 {} 行的值无效：{v}", i + 1));
        }
        out.push((k.to_owned(), v.to_owned()));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headers_parsed_and_comments_skipped() {
        let hs = parse_headers("# 注释\n\ncontent-type: application/json\nX-N: 2 ").unwrap();
        assert_eq!(
            hs,
            vec![
                ("content-type".into(), "application/json".into()),
                ("X-N".into(), "2".into())
            ]
        );
    }

    #[test]
    fn bad_header_line_reports_line_number() {
        let err = parse_headers("X-A: 1\nnope").unwrap_err();
        assert!(err.contains("第 2 行"), "{err}");
    }
}
