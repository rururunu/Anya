//! Split inbound TCP: Companion WebSocket vs local-preview reverse proxy.

use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use tauri::AppHandle;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::TcpStream;

use super::gateway::handle_companion_stream;
use super::preview::{match_preview_path, preview_id_from_cookie, preview_id_from_referer};
use super::state::{PreviewOrigin, RemoteGatewayState};
use std::sync::Arc;

const MAX_HEADER_BYTES: usize = 64 * 1024;
const PREVIEW_COOKIE: &str = "anya_preview";
/// cloudflared will wait on origin until this returns; hang here and the
/// phone's WebSocket upgrade never gets a 101.
const HEADER_READ_TIMEOUT: Duration = Duration::from_secs(10);

struct PrefixedStream {
    prefix: Vec<u8>,
    pos: usize,
    inner: TcpStream,
}

impl PrefixedStream {
    fn new(head: Vec<u8>, extra: Vec<u8>, inner: TcpStream) -> Self {
        let mut prefix = head;
        prefix.extend_from_slice(&extra);
        Self {
            prefix,
            pos: 0,
            inner,
        }
    }
}

impl AsyncRead for PrefixedStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if self.pos < self.prefix.len() {
            let rest = &self.prefix[self.pos..];
            let n = rest.len().min(buf.remaining());
            buf.put_slice(&rest[..n]);
            self.pos += n;
            if self.pos >= self.prefix.len() {
                self.prefix.clear();
                self.pos = 0;
            }
            return Poll::Ready(Ok(()));
        }
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl AsyncWrite for PrefixedStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

struct HttpHead {
    method: String,
    target: String,
    path: String,
    headers: Vec<(String, String)>,
}

impl HttpHead {
    fn header(&self, name: &str) -> Option<&str> {
        let needle = name.to_ascii_lowercase();
        self.headers.iter().find_map(|(k, v)| {
            if k.eq_ignore_ascii_case(&needle) {
                Some(v.as_str())
            } else {
                None
            }
        })
    }

    fn is_websocket_upgrade(&self) -> bool {
        self.header("upgrade")
            .is_some_and(|v| v.eq_ignore_ascii_case("websocket"))
    }
}

pub async fn dispatch(
    app: AppHandle,
    state: Arc<RemoteGatewayState>,
    mut stream: TcpStream,
    peer: SocketAddr,
) -> Result<(), String> {
    let (raw_head, extra) = tokio::time::timeout(HEADER_READ_TIMEOUT, read_http_head(&mut stream))
        .await
        .map_err(|_| "HTTP header read timed out".to_string())??;
    let head = parse_http_head(&raw_head)?;

    if head.path == "/remote/v1" || head.path.starts_with("/remote/v1?") {
        let rewind = PrefixedStream::new(raw_head, extra, stream);
        return handle_companion_stream(app, state, rewind, peer).await;
    }

    if let Some((id, origin_path)) = match_preview_path(&head.path) {
        return proxy_preview(state, stream, head, extra, &id, origin_path, true).await;
    }

    let cookie_id = head
        .header("cookie")
        .and_then(preview_id_from_cookie);
    let referer_id = head
        .header("referer")
        .and_then(preview_id_from_referer);
    let fallback_id = cookie_id.or(referer_id).or_else(|| state.last_preview_id());
    if let Some(id) = fallback_id {
        let origin_path = if head.path.is_empty() {
            "/".to_string()
        } else {
            head.path.clone()
        };
        return proxy_preview(state, stream, head, extra, &id, origin_path, false).await;
    }

    write_simple(&mut stream, 404, "Not found").await
}

async fn proxy_preview(
    state: Arc<RemoteGatewayState>,
    mut client: TcpStream,
    head: HttpHead,
    extra: Vec<u8>,
    preview_id: &str,
    origin_path: String,
    set_cookie: bool,
) -> Result<(), String> {
    let Some(origin) = state.lookup_preview(preview_id) else {
        return write_simple(&mut client, 404, "Unknown preview").await;
    };

    let addr = origin_addr(&origin);
    let mut upstream = TcpStream::connect(&addr)
        .await
        .map_err(|e| format!("preview origin {addr} connect failed: {e}"))?;

    let forwarded = rewrite_request(&head, &origin, &origin_path);
    upstream
        .write_all(&forwarded)
        .await
        .map_err(|e| e.to_string())?;
    if !extra.is_empty() {
        upstream.write_all(&extra).await.map_err(|e| e.to_string())?;
    }

    if head.is_websocket_upgrade() {
        let _ = tokio::io::copy_bidirectional(&mut client, &mut upstream).await;
        return Ok(());
    }

    let (resp_head, resp_extra) = match tokio::time::timeout(
        HEADER_READ_TIMEOUT,
        read_http_head(&mut upstream),
    )
    .await
    {
        Ok(Ok(pair)) => pair,
        _ => return write_simple(&mut client, 502, "Bad gateway").await,
    };
    let secure = head
        .header("x-forwarded-proto")
        .is_some_and(|v| v.eq_ignore_ascii_case("https"))
        || head.header("host").is_some_and(|h| h.contains("trycloudflare.com"));
    let public_host = head.header("host").unwrap_or("").to_string();
    let mut resp = resp_head;
    resp = rewrite_location_headers(&resp, &origin, preview_id, &public_host);
    if set_cookie {
        resp = inject_cookie(resp, preview_id, secure);
    }
    client.write_all(&resp).await.map_err(|e| e.to_string())?;
    if !resp_extra.is_empty() {
        client
            .write_all(&resp_extra)
            .await
            .map_err(|e| e.to_string())?;
    }
    let _ = tokio::io::copy_bidirectional(&mut client, &mut upstream).await;
    Ok(())
}

fn origin_addr(origin: &PreviewOrigin) -> String {
    if origin.host.contains(':') {
        format!("[{}]:{}", origin.host, origin.port)
    } else {
        format!("{}:{}", origin.host, origin.port)
    }
}

fn rewrite_request(head: &HttpHead, origin: &PreviewOrigin, origin_path: &str) -> Vec<u8> {
    let query = head.target.split_once('?').map(|(_, q)| q);
    let uri = match query {
        Some(q) if !origin_path.contains('?') => format!("{origin_path}?{q}"),
        _ => origin_path.to_string(),
    };
    let host = if origin.host.contains(':') {
        format!("[{}]:{}", origin.host, origin.port)
    } else {
        format!("{}:{}", origin.host, origin.port)
    };
    let upgrade = head.is_websocket_upgrade();
    let mut out = format!("{} {} HTTP/1.1\r\nHost: {host}\r\n", head.method, uri);
    for (name, value) in &head.headers {
        if name.eq_ignore_ascii_case("host") {
            continue;
        }
        if !upgrade
            && (name.eq_ignore_ascii_case("connection")
                || name.eq_ignore_ascii_case("proxy-connection")
                || name.eq_ignore_ascii_case("keep-alive"))
        {
            continue;
        }
        if name.eq_ignore_ascii_case("origin") {
            out.push_str(&format!("Origin: {}\r\n", origin.origin_url));
            continue;
        }
        out.push_str(name);
        out.push_str(": ");
        out.push_str(value);
        out.push_str("\r\n");
    }
    if !upgrade {
        out.push_str("Connection: close\r\n");
    }
    out.push_str("\r\n");
    out.into_bytes()
}

fn inject_cookie(head: Vec<u8>, preview_id: &str, secure: bool) -> Vec<u8> {
    let Some(idx) = find_header_end(&head) else {
        return head;
    };
    let mut cookie = format!("{PREVIEW_COOKIE}={preview_id}; Path=/; SameSite=Lax");
    if secure {
        cookie.push_str("; Secure");
    }
    let mut out = head[..idx].to_vec();
    out.extend_from_slice(format!("Set-Cookie: {cookie}\r\n\r\n").as_bytes());
    out
}

fn rewrite_location_headers(
    head: &[u8],
    origin: &PreviewOrigin,
    preview_id: &str,
    public_host: &str,
) -> Vec<u8> {
    let Ok(text) = std::str::from_utf8(head) else {
        return head.to_vec();
    };
    let origin_prefix = origin.origin_url.trim_end_matches('/');
    let public_prefix = if public_host.is_empty() {
        format!("/p/{preview_id}")
    } else {
        let scheme = if public_host.contains("trycloudflare.com") {
            "https"
        } else {
            "http"
        };
        format!("{scheme}://{public_host}/p/{preview_id}")
    };
    if !text.to_ascii_lowercase().contains("location:") {
        return head.to_vec();
    }
    let rewritten = text.replace(origin_prefix, public_prefix.trim_end_matches('/'));
    rewritten.into_bytes()
}

async fn read_http_head(stream: &mut TcpStream) -> Result<(Vec<u8>, Vec<u8>), String> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 2048];
    loop {
        let n = stream.read(&mut tmp).await.map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("connection closed before HTTP headers".into());
        }
        buf.extend_from_slice(&tmp[..n]);
        if buf.len() > MAX_HEADER_BYTES {
            return Err("HTTP headers too large".into());
        }
        if let Some(idx) = find_header_end(&buf) {
            let extra = buf.split_off(idx + 4);
            return Ok((buf, extra));
        }
    }
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

fn parse_http_head(raw: &[u8]) -> Result<HttpHead, String> {
    let text = std::str::from_utf8(raw).map_err(|_| "HTTP headers are not utf-8".to_string())?;
    let mut lines = text.split("\r\n");
    let request_line = lines.next().ok_or_else(|| "empty HTTP request".to_string())?;
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| "missing HTTP method".to_string())?
        .to_string();
    let target = parts
        .next()
        .ok_or_else(|| "missing HTTP target".to_string())?
        .to_string();
    let path = target
        .split_once('?')
        .map(|(p, _)| p.to_string())
        .unwrap_or_else(|| target.clone());
    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    Ok(HttpHead {
        method,
        target,
        path,
        headers,
    })
}

async fn write_simple(stream: &mut TcpStream, status: u16, body: &str) -> Result<(), String> {
    let reason = match status {
        404 => "Not Found",
        502 => "Bad Gateway",
        _ => "Error",
    };
    let resp = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(resp.as_bytes()).await.map_err(|e| e.to_string())
}
