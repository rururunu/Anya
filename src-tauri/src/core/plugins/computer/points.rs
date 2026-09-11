//! Screenshot-image pixels → screen pixels for click / move / drag.

use std::sync::{Mutex, OnceLock};

use serde_json::Value;

use crate::core::tools::error::ToolError;

const DRAG_PATH_MIN: usize = 2;
const DRAG_PATH_MAX: usize = 128;
const ARC_POINTS_MAX: usize = 48;

#[derive(Clone, Copy)]
pub(super) struct CaptureMeta {
    pub scale: f64,
    pub origin_x: i32,
    pub origin_y: i32,
}

fn last_meta() -> &'static Mutex<Option<CaptureMeta>> {
    static LAST: OnceLock<Mutex<Option<CaptureMeta>>> = OnceLock::new();
    LAST.get_or_init(|| Mutex::new(None))
}

pub(super) fn remember_capture(scale: f64, origin_x: i32, origin_y: i32) {
    if let Ok(mut guard) = last_meta().lock() {
        *guard = Some(CaptureMeta {
            scale,
            origin_x,
            origin_y,
        });
    }
}

pub(super) fn to_screen_xy(args: &Value) -> Result<(i32, i32), ToolError> {
    let x = args["x"]
        .as_f64()
        .ok_or_else(|| ToolError::new("x is required"))?;
    let y = args["y"]
        .as_f64()
        .ok_or_else(|| ToolError::new("y is required"))?;
    Ok(image_to_screen(x, y))
}

pub(super) fn image_to_screen(x: f64, y: f64) -> (i32, i32) {
    let meta = last_meta().lock().ok().and_then(|g| *g);
    match meta {
        Some(meta) if meta.scale > 0.0 => (
            (x / meta.scale).round() as i32 + meta.origin_x,
            (y / meta.scale).round() as i32 + meta.origin_y,
        ),
        _ => (x.round() as i32, y.round() as i32),
    }
}

/// Start `(x,y)` plus `x2,y2`, a `path`, or `cx,cy,r` (optional startDeg/endDeg) for an arc.
pub(super) fn drag_screen_points(args: &Value) -> Result<Vec<(i32, i32)>, ToolError> {
    if let Some(points) = arc_points(args)? {
        return Ok(points);
    }
    if let Some(arr) = args.get("path").and_then(Value::as_array) {
        if !(DRAG_PATH_MIN..=DRAG_PATH_MAX).contains(&arr.len()) {
            return Err(ToolError::new(format!(
                "drag path needs {DRAG_PATH_MIN}–{DRAG_PATH_MAX} points"
            )));
        }
        return arr.iter().map(point_from_value).collect();
    }
    let start = to_screen_xy(args)?;
    let x2 = args
        .get("x2")
        .or_else(|| args.get("toX"))
        .and_then(Value::as_f64);
    let y2 = args
        .get("y2")
        .or_else(|| args.get("toY"))
        .and_then(Value::as_f64);
    match (x2, y2) {
        (Some(x), Some(y)) => Ok(vec![start, image_to_screen(x, y)]),
        _ => Err(ToolError::new(
            "drag needs x2,y2, path:[{x,y},...], or cx,cy,r for an arc/circle",
        )),
    }
}

fn arc_points(args: &Value) -> Result<Option<Vec<(i32, i32)>>, ToolError> {
    let circle = args.get("circle");
    let cx = num_arg(args, "cx")
        .or_else(|| circle.and_then(|c| num_arg(c, "cx")))
        .or_else(|| circle.and_then(|c| num_arg(c, "x")));
    let cy = num_arg(args, "cy")
        .or_else(|| circle.and_then(|c| num_arg(c, "cy")))
        .or_else(|| circle.and_then(|c| num_arg(c, "y")));
    let r = num_arg(args, "r")
        .or_else(|| circle.and_then(|c| num_arg(c, "r")))
        .or_else(|| num_arg(args, "radius"));
    let (Some(cx), Some(cy), Some(r)) = (cx, cy, r) else {
        return Ok(None);
    };
    if r < 2.0 {
        return Err(ToolError::new("arc radius must be at least 2 image pixels"));
    }
    let start = num_arg(args, "startDeg")
        .or_else(|| num_arg(args, "start"))
        .or_else(|| circle.and_then(|c| num_arg(c, "startDeg")))
        .unwrap_or(0.0);
    let end = num_arg(args, "endDeg")
        .or_else(|| num_arg(args, "end"))
        .or_else(|| circle.and_then(|c| num_arg(c, "endDeg")))
        .unwrap_or(360.0);
    let sweep = (end - start).abs().max(1.0);
    let arc_len = std::f64::consts::PI * r * sweep / 180.0;
    let n = ((arc_len / 10.0).ceil() as usize).clamp(8, ARC_POINTS_MAX);
    let mut points = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let t = i as f64 / n as f64;
        let deg = start + (end - start) * t;
        let rad = deg.to_radians();
        points.push(image_to_screen(cx + r * rad.cos(), cy + r * rad.sin()));
    }
    Ok(Some(points))
}

fn num_arg(args: &Value, key: &str) -> Option<f64> {
    args.get(key).and_then(|v| {
        v.as_f64()
            .or_else(|| v.as_i64().map(|n| n as f64))
            .or_else(|| v.as_u64().map(|n| n as f64))
    })
}

fn point_from_value(value: &Value) -> Result<(i32, i32), ToolError> {
    let x = value["x"]
        .as_f64()
        .ok_or_else(|| ToolError::new("path point needs x"))?;
    let y = value["y"]
        .as_f64()
        .ok_or_else(|| ToolError::new("path point needs y"))?;
    Ok(image_to_screen(x, y))
}

/// `modifiers: "ctrl+shift"` or `["ctrl","shift"]`. Only ctrl/alt/shift/win.
pub(super) fn parse_modifiers(args: &Value) -> Result<Vec<String>, ToolError> {
    let raw = args.get("modifiers").or_else(|| args.get("mods"));
    let parts: Vec<String> = match raw {
        None => return Ok(Vec::new()),
        Some(Value::String(text)) => text
            .split(['+', ',', ' '])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_ascii_lowercase())
            .collect(),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .collect(),
        Some(_) => return Err(ToolError::new("modifiers must be a string or array")),
    };
    let mut out = Vec::new();
    for part in parts {
        let name = match part.as_str() {
            "ctrl" | "control" => "ctrl",
            "alt" | "option" => "alt",
            "shift" => "shift",
            "win" | "windows" | "meta" | "super" | "cmd" => "win",
            other => {
                return Err(ToolError::new(format!(
                    "unsupported modifier `{other}` (use ctrl, alt, shift, win)"
                )));
            }
        };
        if !out.iter().any(|n| n == name) {
            out.push(name.to_string());
        }
    }
    Ok(out)
}

pub(super) fn wait_ms(args: &Value, default: u64) -> u64 {
    args.get("waitMs")
        .or_else(|| args.get("wait_ms"))
        .or_else(|| args.get("hoverMs"))
        .or_else(|| args.get("ms"))
        .and_then(|v| v.as_u64().or_else(|| v.as_f64().map(|f| f.max(0.0) as u64)))
        .unwrap_or(default)
        .clamp(0, 4000)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn meta_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    #[test]
    fn maps_image_coords_with_scale() {
        let _guard = meta_lock();
        remember_capture(0.5, 0, 0);
        assert_eq!(image_to_screen(100.0, 40.0), (200, 80));
        remember_capture(0.5, -1920, 0);
        assert_eq!(image_to_screen(100.0, 40.0), (-1720, 80));
        *last_meta().lock().unwrap() = None;
        assert_eq!(image_to_screen(100.0, 40.0), (100, 40));
    }

    #[test]
    fn drag_points_from_x2_and_path() {
        let _guard = meta_lock();
        *last_meta().lock().unwrap() = None;
        let line = drag_screen_points(&json!({ "x": 10, "y": 20, "x2": 30, "y2": 40 })).unwrap();
        assert_eq!(line, vec![(10, 20), (30, 40)]);
        let path = drag_screen_points(&json!({
            "path": [{ "x": 0, "y": 0 }, { "x": 8, "y": 0 }, { "x": 8, "y": 8 }]
        }))
        .unwrap();
        assert_eq!(path, vec![(0, 0), (8, 0), (8, 8)]);
        assert!(drag_screen_points(&json!({ "x": 1, "y": 2 })).is_err());
        let circle = drag_screen_points(&json!({ "cx": 50.0, "cy": 50.0, "r": 20.0 })).unwrap();
        assert!(circle.len() >= 8);
        assert_eq!(circle.first(), circle.last());
        let arc = drag_screen_points(&json!({
            "cx": 10.0, "cy": 10.0, "r": 8.0, "startDeg": 0.0, "endDeg": 90.0
        }))
        .unwrap();
        assert!(arc.len() >= 8);
        assert_ne!(arc.first(), arc.last());
    }

    #[test]
    fn modifiers_and_wait_ms() {
        assert!(parse_modifiers(&json!({})).unwrap().is_empty());
        assert_eq!(
            parse_modifiers(&json!({ "modifiers": "ctrl+shift" })).unwrap(),
            vec!["ctrl", "shift"]
        );
        assert_eq!(
            parse_modifiers(&json!({ "mods": ["alt", "win"] })).unwrap(),
            vec!["alt", "win"]
        );
        assert!(parse_modifiers(&json!({ "modifiers": "ctrl+c" })).is_err());
        assert_eq!(wait_ms(&json!({}), 400), 400);
        assert_eq!(wait_ms(&json!({ "waitMs": 9000 }), 0), 4000);
        assert_eq!(wait_ms(&json!({ "ms": 120 }), 0), 120);
    }
}
