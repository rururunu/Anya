//! Local-preview registry: map `/p/{id}/` to a loopback HTTP origin.

use tauri::AppHandle;

use super::pairing::local_ipv4_hosts;
use super::state::{remote_state, PreviewOrigin};

/// Parse `http://127.0.0.1|localhost[:port]/...` into `(connect_host, port, origin)`.
pub fn parse_loopback_http_url(raw: &str) -> Result<(String, u16, String), String> {
    let raw = raw.trim();
    let lower = raw.to_ascii_lowercase();
    if !lower.starts_with("http://") {
        return Err("url must be http://127.0.0.1 or http://localhost".into());
    }
    let rest = &raw["http://".len()..];
    let hostport = rest.split(['/', '?']).next().unwrap_or(rest);
    let hostport = hostport
        .rsplit_once('@')
        .map(|(_, host)| host)
        .unwrap_or(hostport);

    let (host, port) = if let Some(inner) = hostport.strip_prefix('[') {
        let end = inner
            .find(']')
            .ok_or_else(|| "invalid IPv6 url".to_string())?;
        let host = &inner[..end];
        let port = inner[end + 1..]
            .strip_prefix(':')
            .and_then(|p| p.parse().ok())
            .unwrap_or(80);
        (host.to_string(), port)
    } else {
        match hostport.rsplit_once(':') {
            Some((h, p)) => (
                h.to_string(),
                p.parse::<u16>().map_err(|_| "invalid port".to_string())?,
            ),
            None => (hostport.to_string(), 80),
        }
    };

    let host_l = host.to_ascii_lowercase();
    if host_l != "127.0.0.1" && host_l != "localhost" && host_l != "::1" {
        return Err("url host must be 127.0.0.1 or localhost".into());
    }
    if port == 0 {
        return Err("invalid port".into());
    }
    let connect_host = if host_l == "localhost" {
        "127.0.0.1".to_string()
    } else {
        host
    };
    let origin = if connect_host.contains(':') {
        format!("http://[{connect_host}]:{port}")
    } else {
        format!("http://{connect_host}:{port}")
    };
    Ok((connect_host, port, origin))
}

pub fn register_preview(
    app: &AppHandle,
    origin_url: &str,
    session_id: &str,
) -> Result<PreviewOrigin, String> {
    let (host, port, origin) = parse_loopback_http_url(origin_url)?;
    let id = uuid::Uuid::new_v4().to_string();
    let preview = PreviewOrigin {
        id: id.clone(),
        host,
        port,
        origin_url: origin,
        session_id: session_id.to_string(),
    };
    remote_state(app).register_preview(preview.clone());
    Ok(preview)
}

pub fn public_preview_url(app: &AppHandle, preview_id: &str) -> String {
    let state = remote_state(app);
    let port = if state.is_running() {
        state.port()
    } else {
        state.preferred_port()
    };
    if let Some(public) = state.tunnel_public_info() {
        let scheme = if public.scheme.is_empty() {
            "https"
        } else {
            public.scheme.as_str()
        };
        return format!(
            "{scheme}://{}/p/{preview_id}/",
            public.host.trim_end_matches('/')
        );
    }
    let lan = local_ipv4_hosts()
        .into_iter()
        .next()
        .unwrap_or_else(|| "127.0.0.1".into());
    format!("http://{lan}:{port}/p/{preview_id}/")
}

pub fn match_preview_path(path: &str) -> Option<(String, String)> {
    let path = path.split('?').next().unwrap_or(path);
    let rest = path.strip_prefix("/p/")?;
    if rest.is_empty() {
        return None;
    }
    let (id, tail) = rest.split_once('/').unwrap_or((rest, ""));
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }
    let origin_path = if tail.is_empty() {
        "/".to_string()
    } else {
        format!("/{tail}")
    };
    Some((id.to_string(), origin_path))
}

pub fn preview_id_from_cookie(cookie: &str) -> Option<String> {
    for part in cookie.split(';') {
        let part = part.trim();
        if let Some(value) = part.strip_prefix("anya_preview=") {
            let id = value.trim();
            if !id.is_empty() {
                return Some(id.to_string());
            }
        }
    }
    None
}

pub fn preview_id_from_referer(referer: &str) -> Option<String> {
    let path = referer
        .split("://")
        .nth(1)
        .and_then(|rest| rest.split_once('/').map(|(_, p)| p))
        .map(|p| format!("/{p}"))
        .unwrap_or_else(|| referer.to_string());
    match_preview_path(&path).map(|(id, _)| id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_loopback_urls() {
        let (host, port, origin) = parse_loopback_http_url("http://127.0.0.1:5173/app").unwrap();
        assert_eq!(host, "127.0.0.1");
        assert_eq!(port, 5173);
        assert_eq!(origin, "http://127.0.0.1:5173");

        let (host, port, _) = parse_loopback_http_url("http://localhost:3000").unwrap();
        assert_eq!(host, "127.0.0.1");
        assert_eq!(port, 3000);
    }

    #[test]
    fn rejects_non_loopback() {
        assert!(parse_loopback_http_url("https://127.0.0.1:5173").is_err());
        assert!(parse_loopback_http_url("http://example.com:80").is_err());
        assert!(parse_loopback_http_url("http://192.168.1.2:5173").is_err());
    }

    #[test]
    fn matches_preview_prefix() {
        let (id, path) = match_preview_path("/p/abc-123/foo?x=1").unwrap();
        assert_eq!(id, "abc-123");
        assert_eq!(path, "/foo");
        assert_eq!(match_preview_path("/p/abc-123").unwrap().1, "/");
        assert!(match_preview_path("/remote/v1").is_none());
    }
}
