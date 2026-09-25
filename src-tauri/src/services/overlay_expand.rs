//! First-message expansion: preserve the draft's bottom edge and commit the
//! native rectangle before mounting the chat UI. No per-frame window IPC.
use tauri::{LogicalSize, PhysicalPosition, PhysicalSize, WebviewWindow};

#[derive(Debug, PartialEq)]
struct Bounds {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

fn chat_bounds(old: Bounds, scale: f64, zoom: f64, area: Option<Bounds>) -> Bounds {
    let factor = scale * zoom;
    let mut width = (640.0 * factor).round() as u32;
    let mut height = (520.0 * factor).round() as u32;
    if let Some(area) = &area {
        width = width.min(area.width);
        height = height.min(
            area.height
                .saturating_sub((48.0 * scale).round() as u32)
                .max(1),
        );
    }
    let mut x = old.x + (old.width as i32 - width as i32) / 2;
    let mut y = old.y + old.height as i32 - height as i32;
    if let Some(area) = area {
        x = x.clamp(area.x, area.x + area.width as i32 - width as i32);
        y = y.clamp(area.y, area.y + area.height as i32 - height as i32);
    }
    Bounds {
        x,
        y,
        width,
        height,
    }
}

pub fn expand(window: &WebviewWindow, zoom: f64) -> Result<(), String> {
    if !super::window::is_overlay_label(window.label()) {
        return Err("Only overlay windows can expand into chat".into());
    }
    if !zoom.is_finite() || !(0.25..=4.0).contains(&zoom) {
        return Err("Invalid overlay zoom".into());
    }
    if !window.is_visible().map_err(|e| e.to_string())? {
        return Ok(());
    }
    super::window::set_overlay_chat_mode(window.label(), true);
    if window.is_maximized().map_err(|e| e.to_string())? {
        return Ok(());
    }
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    // Capture BEFORE changing constraints: set_min_size can grow a 56px window
    // to 240px itself, losing the original anchor and causing a second jump.
    let position = window.outer_position().map_err(|e| e.to_string())?;
    let size = window.outer_size().map_err(|e| e.to_string())?;
    let area = window
        .current_monitor()
        .map_err(|e| e.to_string())?
        .map(|m| {
            let a = m.work_area();
            Bounds {
                x: a.position.x,
                y: a.position.y,
                width: a.size.width,
                height: a.size.height,
            }
        });
    let bounds = chat_bounds(
        Bounds {
            x: position.x,
            y: position.y,
            width: size.width,
            height: size.height,
        },
        scale,
        zoom,
        area,
    );
    window
        .set_max_size(None::<LogicalSize<f64>>)
        .map_err(|e| e.to_string())?;
    // Drop the old minimum in case UI zoom or monitor work area became smaller.
    window
        .set_min_size(None::<LogicalSize<f64>>)
        .map_err(|e| e.to_string())?;
    window.set_resizable(true).map_err(|e| e.to_string())?;
    super::overlay_native::set_overlay_bounds(
        window,
        PhysicalPosition::new(bounds.x, bounds.y),
        PhysicalSize::new(bounds.width, bounds.height),
    )?;
    // Install the chat minimum only after the final rectangle is in place.
    window
        .set_min_size(Some(PhysicalSize::new(
            bounds.width,
            ((240.0 * scale * zoom).round() as u32).min(bounds.height),
        )))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expansion_preserves_the_original_bottom_edge() {
        let result = chat_bounds(
            Bounds {
                x: 200,
                y: 600,
                width: 640,
                height: 56,
            },
            1.0,
            1.0,
            None,
        );
        assert_eq!(
            result,
            Bounds {
                x: 200,
                y: 136,
                width: 640,
                height: 520
            }
        );
    }

    #[test]
    fn high_dpi_and_zoom_are_applied_once() {
        let result = chat_bounds(
            Bounds {
                x: 200,
                y: 1200,
                width: 1200,
                height: 105,
            },
            1.5,
            1.25,
            None,
        );
        assert_eq!(
            result,
            Bounds {
                x: 200,
                y: 330,
                width: 1200,
                height: 975
            }
        );
    }

    #[test]
    fn fits_small_negative_coordinate_monitor_work_area() {
        let result = chat_bounds(
            Bounds {
                x: -750,
                y: 20,
                width: 640,
                height: 56,
            },
            1.0,
            2.0,
            Some(Bounds {
                x: -800,
                y: 0,
                width: 800,
                height: 600,
            }),
        );
        assert_eq!(
            result,
            Bounds {
                x: -800,
                y: 0,
                width: 800,
                height: 552
            }
        );
    }
}
