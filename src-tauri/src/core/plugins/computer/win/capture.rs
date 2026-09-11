//! GDI / PrintWindow capture to unique JPEGs.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    GetDeviceCaps, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    HDC, HGDIOBJ, LOGPIXELSX, SRCCOPY,
};
use windows::Win32::UI::WindowsAndMessaging::{GetWindowRect, IsIconic};

use super::windows::{find_hwnd, foreground_hwnd, restore_window, virtual_screen, window_title};
use crate::core::plugins::manifest::plugin_dir;
use crate::core::tools::error::ToolError;

const MAX_IMAGE_EDGE: u32 = 1280;
const JPEG_DESKTOP: u8 = 72;
const JPEG_WINDOW: u8 = 85;
const KEEP_SHOTS: usize = 8;
const PW_RENDERFULLCONTENT: u32 = 2;

static SHOT_SEQ: AtomicU32 = AtomicU32::new(1);
static LAST_JPEG_HASH: Mutex<Option<[u8; 32]>> = Mutex::new(None);

#[link(name = "user32")]
extern "system" {
    fn PrintWindow(hwnd: HWND, hdcblt: HDC, nflags: u32) -> i32;
}

#[derive(Clone)]
pub(crate) enum CaptureTarget {
    Desktop,
    Foreground,
    Title(String),
}

pub(crate) struct CaptureFile {
    pub origin_x: i32,
    pub origin_y: i32,
    pub screen_w: i32,
    pub screen_h: i32,
    pub image_w: u32,
    pub image_h: u32,
    pub path: PathBuf,
    pub unchanged: bool,
    pub label: String,
    pub hwnd: isize,
    pub dpi: u32,
}

/// `shot-{unix_ms}-{seq}.jpg` — never `last.jpg`.
pub(crate) fn next_shot_name(now_ms: u128, seq: u32) -> String {
    format!("shot-{now_ms}-{seq}.jpg")
}

pub(crate) fn capture_to_plugin(
    plugin_id: &str,
    target: CaptureTarget,
) -> Result<CaptureFile, ToolError> {
    let (origin_x, origin_y, screen_w, screen_h, image_w, image_h, jpeg, label, hwnd, dpi) =
        capture_encoded(&target)?;
    let hash = jpeg_hash(&jpeg);
    if last_hash_matches(hash) {
        return Ok(CaptureFile {
            origin_x,
            origin_y,
            screen_w,
            screen_h,
            image_w,
            image_h,
            path: PathBuf::new(),
            unchanged: true,
            label,
            hwnd,
            dpi,
        });
    }
    let dir = plugin_dir(plugin_id)?.join("data").join("computer");
    fs::create_dir_all(&dir).map_err(|e| ToolError::new(format!("computer dir: {e}")))?;
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let seq = SHOT_SEQ.fetch_add(1, Ordering::Relaxed);
    let path = dir.join(next_shot_name(now_ms, seq));
    fs::write(&path, jpeg).map_err(|e| ToolError::new(format!("write screenshot: {e}")))?;
    store_hash(hash);
    prune_shots(&dir);
    Ok(CaptureFile {
        origin_x,
        origin_y,
        screen_w,
        screen_h,
        image_w,
        image_h,
        path,
        unchanged: false,
        label,
        hwnd,
        dpi,
    })
}

fn capture_named_window(
    title: Option<&str>,
) -> Result<(HWND, i32, i32, i32, i32, Vec<u8>, String, u8), ToolError> {
    let hwnd = if let Some(title) = title {
        find_hwnd(Some(title), None)?
    } else {
        foreground_hwnd()?
    };
    let was_min = unsafe { IsIconic(hwnd).as_bool() };
    restore_window(hwnd);
    if was_min {
        std::thread::sleep(Duration::from_millis(50));
    }
    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect) }
        .map_err(|e| ToolError::new(format!("GetWindowRect: {e}")))?;
    let w = (rect.right - rect.left).max(1);
    let h = (rect.bottom - rect.top).max(1);
    let mut pixels = capture_print_window(hwnd, w, h).filter(|px| !is_blank(px));
    if pixels.is_none() {
        pixels = Some(capture_rect_bitblt(rect.left, rect.top, w, h)?);
    }
    let title = window_title(hwnd);
    let label = if title.is_empty() {
        "foreground window".into()
    } else {
        format!("window `{title}`")
    };
    Ok((
        hwnd,
        rect.left,
        rect.top,
        w,
        h,
        pixels.unwrap_or_default(),
        label,
        JPEG_WINDOW,
    ))
}

fn capture_desktop(vx: i32, vy: i32, vw: i32, vh: i32) -> Result<(i32, i32, Vec<u8>), ToolError> {
    let vw = vw.max(1);
    let vh = vh.max(1);
    let pixels = capture_rect_bitblt(vx, vy, vw, vh)?;
    Ok((vw, vh, pixels))
}

fn capture_print_window(hwnd: HWND, w: i32, h: i32) -> Option<Vec<u8>> {
    unsafe {
        let screen_dc = GetDC(None);
        if screen_dc.0.is_null() {
            return None;
        }
        let mem_dc = CreateCompatibleDC(screen_dc);
        if mem_dc.0.is_null() {
            ReleaseDC(None, screen_dc);
            return None;
        }
        let bitmap = CreateCompatibleBitmap(screen_dc, w, h);
        if bitmap.0.is_null() {
            let _ = DeleteDC(mem_dc);
            ReleaseDC(None, screen_dc);
            return None;
        }
        let old = SelectObject(mem_dc, HGDIOBJ(bitmap.0));
        let printed = PrintWindow(hwnd, mem_dc, PW_RENDERFULLCONTENT) != 0;
        let pixels = if printed {
            read_dibits(mem_dc, bitmap, w, h)
        } else {
            None
        };
        SelectObject(mem_dc, old);
        let _ = DeleteObject(bitmap);
        let _ = DeleteDC(mem_dc);
        ReleaseDC(None, screen_dc);
        pixels
    }
}

fn capture_rect_bitblt(x: i32, y: i32, w: i32, h: i32) -> Result<Vec<u8>, ToolError> {
    unsafe {
        let screen_dc = GetDC(None);
        if screen_dc.0.is_null() {
            return Err(ToolError::new("GetDC failed"));
        }
        let mem_dc = CreateCompatibleDC(screen_dc);
        if mem_dc.0.is_null() {
            ReleaseDC(None, screen_dc);
            return Err(ToolError::new("CreateCompatibleDC failed"));
        }
        let bitmap = CreateCompatibleBitmap(screen_dc, w, h);
        if bitmap.0.is_null() {
            let _ = DeleteDC(mem_dc);
            ReleaseDC(None, screen_dc);
            return Err(ToolError::new("CreateCompatibleBitmap failed"));
        }
        let old = SelectObject(mem_dc, HGDIOBJ(bitmap.0));
        let blt_ok = BitBlt(mem_dc, 0, 0, w, h, screen_dc, x, y, SRCCOPY).is_ok();
        let pixels = if blt_ok {
            read_dibits(mem_dc, bitmap, w, h)
        } else {
            None
        };
        SelectObject(mem_dc, old);
        let _ = DeleteObject(bitmap);
        let _ = DeleteDC(mem_dc);
        ReleaseDC(None, screen_dc);
        pixels.ok_or_else(|| ToolError::new("screen capture failed"))
    }
}

unsafe fn read_dibits(
    mem_dc: HDC,
    bitmap: windows::Win32::Graphics::Gdi::HBITMAP,
    w: i32,
    h: i32,
) -> Option<Vec<u8>> {
    let mut bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut pixels = vec![0u8; (w as usize) * (h as usize) * 4];
    let lines = GetDIBits(
        mem_dc,
        bitmap,
        0,
        h as u32,
        Some(pixels.as_mut_ptr() as *mut _),
        &mut bmi,
        DIB_RGB_COLORS,
    );
    (lines != 0).then_some(pixels)
}

fn encode_jpeg(bgra: &[u8], w: i32, h: i32, quality: u8) -> Result<(u32, u32, Vec<u8>), ToolError> {
    use image::codecs::jpeg::JpegEncoder;
    use image::{ImageBuffer, Rgba};

    if bgra.len() < (w as usize) * (h as usize) * 4 {
        return Err(ToolError::new("capture buffer empty"));
    }
    let mut rgba = Vec::with_capacity(bgra.len());
    for chunk in bgra.chunks_exact(4) {
        rgba.push(chunk[2]);
        rgba.push(chunk[1]);
        rgba.push(chunk[0]);
        rgba.push(255);
    }
    let image: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_raw(w as u32, h as u32, rgba)
        .ok_or_else(|| ToolError::new("invalid capture buffer"))?;
    let (image_w, image_h, rgb) = if w as u32 > MAX_IMAGE_EDGE || h as u32 > MAX_IMAGE_EDGE {
        let scale = MAX_IMAGE_EDGE as f32 / (w.max(h) as f32);
        let new_w = ((w as f32) * scale).round().max(1.0) as u32;
        let new_h = ((h as f32) * scale).round().max(1.0) as u32;
        let resized =
            image::imageops::resize(&image, new_w, new_h, image::imageops::FilterType::Triangle);
        (
            new_w,
            new_h,
            image::DynamicImage::ImageRgba8(resized).to_rgb8(),
        )
    } else {
        (
            w as u32,
            h as u32,
            image::DynamicImage::ImageRgba8(image).to_rgb8(),
        )
    };
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, quality)
        .encode_image(&rgb)
        .map_err(|e| ToolError::new(format!("jpeg: {e}")))?;
    Ok((image_w, image_h, jpeg))
}

fn is_blank(pixels: &[u8]) -> bool {
    let n = pixels.len() / 4;
    if n == 0 {
        return true;
    }
    let dark = pixels
        .chunks_exact(4)
        .filter(|c| c[0] < 8 && c[1] < 8 && c[2] < 8)
        .count();
    dark * 100 / n > 98
}

fn capture_encoded(
    target: &CaptureTarget,
) -> Result<(i32, i32, i32, i32, u32, u32, Vec<u8>, String, isize, u32), ToolError> {
    let (hwnd, origin_x, origin_y, screen_w, screen_h, pixels, label, jpeg_q) = match target {
        CaptureTarget::Desktop => {
            let (vx, vy, vw, vh) = virtual_screen();
            let (w, h, px) = capture_desktop(vx, vy, vw, vh)?;
            (
                HWND::default(),
                vx,
                vy,
                w,
                h,
                px,
                "desktop".to_string(),
                JPEG_DESKTOP,
            )
        }
        CaptureTarget::Foreground => capture_named_window(None)?,
        CaptureTarget::Title(title) => capture_named_window(Some(title.as_str()))?,
    };
    let (image_w, image_h, jpeg) = encode_jpeg(&pixels, screen_w, screen_h, jpeg_q)?;
    let dpi = window_dpi(hwnd);
    Ok((
        origin_x,
        origin_y,
        screen_w,
        screen_h,
        image_w,
        image_h,
        jpeg,
        label,
        hwnd.0 as isize,
        dpi,
    ))
}

fn window_dpi(hwnd: HWND) -> u32 {
    unsafe {
        let hdc = GetDC(hwnd);
        if hdc.0.is_null() {
            return 96;
        }
        let dpi = GetDeviceCaps(hdc, LOGPIXELSX);
        ReleaseDC(hwnd, hdc);
        if dpi <= 0 {
            96
        } else {
            dpi as u32
        }
    }
}

fn jpeg_hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn last_hash_matches(hash: [u8; 32]) -> bool {
    LAST_JPEG_HASH
        .lock()
        .ok()
        .and_then(|g| *g)
        .is_some_and(|prev| prev == hash)
}

fn store_hash(hash: [u8; 32]) {
    if let Ok(mut g) = LAST_JPEG_HASH.lock() {
        *g = Some(hash);
    }
}

fn prune_shots(dir: &Path) {
    let _ = fs::remove_file(dir.join("last.jpg"));
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = fs::read_dir(dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| n.starts_with("shot-") && n.ends_with(".jpg"))
        })
        .filter_map(|e| {
            let modified = e.metadata().ok()?.modified().ok()?;
            Some((modified, e.path()))
        })
        .collect();
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let extra = files.len().saturating_sub(KEEP_SHOTS);
    for (_, path) in files.into_iter().take(extra) {
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shot_names_are_unique_not_last_jpg() {
        let a = next_shot_name(1_700_000_000_000, 1);
        let b = next_shot_name(1_700_000_000_000, 2);
        assert_ne!(a, "last.jpg");
        assert_ne!(b, "last.jpg");
        assert!(a.starts_with("shot-"));
        assert!(a.ends_with(".jpg"));
        assert_ne!(a, b);
    }
}
