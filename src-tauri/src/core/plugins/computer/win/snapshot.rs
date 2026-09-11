//! Compact interactive UIA overlay attached to a screenshot.

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Accessibility::{
    IUIAutomation, IUIAutomationElement, IUIAutomationTreeWalker, UIA_ButtonControlTypeId,
    UIA_CheckBoxControlTypeId, UIA_ComboBoxControlTypeId, UIA_CustomControlTypeId,
    UIA_DocumentControlTypeId, UIA_EditControlTypeId, UIA_HyperlinkControlTypeId,
    UIA_ListItemControlTypeId, UIA_MenuItemControlTypeId, UIA_RadioButtonControlTypeId,
    UIA_SliderControlTypeId, UIA_SplitButtonControlTypeId, UIA_TabItemControlTypeId,
    UIA_TreeItemControlTypeId, UIA_CONTROLTYPE_ID,
};

use super::uia::{store_last, with_uia, Hit};
use crate::core::tools::error::ToolError;

const MAX_DEPTH: u32 = 14;
const MAX_NODES: u32 = 600;
const MAX_HITS: usize = 40;

pub(crate) fn overlay_for_hwnd(
    hwnd: isize,
    origin_x: i32,
    origin_y: i32,
    scale: f64,
) -> String {
    if hwnd == 0 {
        return "Controls: desktop capture has no window tree; launch/key or screenshot a window."
            .into();
    }
    let handle = HWND(hwnd as *mut core::ffi::c_void);
    overlay_for_window(handle, origin_x, origin_y, scale).unwrap_or_else(|e| {
        format!("Controls: UIA unavailable ({e}). Fall back to key / pixel click.")
    })
}

fn overlay_for_window(
    hwnd: HWND,
    origin_x: i32,
    origin_y: i32,
    scale: f64,
) -> Result<String, ToolError> {
    if hwnd.is_invalid() {
        return Ok(String::new());
    }
    let hits = with_uia(|automation| collect_interactive(automation, hwnd))?;
    store_last(hwnd, hits.clone());
    if hits.is_empty() {
        return Ok(
            "Controls: none named. Pixel-click the canvas or find_control a shorter name.".into(),
        );
    }
    let extra = hits.len().saturating_sub(MAX_HITS);
    let lines: Vec<String> = hits
        .iter()
        .take(MAX_HITS)
        .enumerate()
        .map(|(i, hit)| format_hit(i, hit, origin_x, origin_y, scale))
        .collect();
    let more = if extra > 0 {
        format!("\n… +{extra} more. find_control to search by name/id.")
    } else {
        String::new()
    };
    Ok(format!(
        "Controls (prefer click_control index / name / id; pixel click only for canvas):\n{}{more}",
        lines.join("\n")
    ))
}

fn format_hit(i: usize, hit: &Hit, origin_x: i32, origin_y: i32, scale: f64) -> String {
    let img_x = ((hit.left - origin_x) as f64 * scale).round() as i32;
    let img_y = ((hit.top - origin_y) as f64 * scale).round() as i32;
    let img_w = (hit.width as f64 * scale).round().max(1.0) as i32;
    let img_h = (hit.height as f64 * scale).round().max(1.0) as i32;
    let aid = if hit.aid.is_empty() {
        String::new()
    } else {
        format!(" aid={}", hit.aid)
    };
    format!(
        "[{i}] {} \"{}\"{aid} img=({img_x},{img_y}) {img_w}x{img_h}",
        hit.kind, hit.name
    )
}

fn collect_interactive(automation: &IUIAutomation, hwnd: HWND) -> Result<Vec<Hit>, ToolError> {
    let root = unsafe { automation.ElementFromHandle(hwnd) }
        .map_err(|e| ToolError::new(format!("ElementFromHandle: {e}")))?;
    let walker = unsafe { automation.ControlViewWalker() }
        .map_err(|e| ToolError::new(format!("ControlViewWalker: {e}")))?;
    let mut hits = Vec::new();
    let mut nodes = 0;
    walk(&walker, &root, 0, &mut nodes, &mut hits);
    Ok(hits)
}

fn walk(
    walker: &IUIAutomationTreeWalker,
    element: &IUIAutomationElement,
    depth: u32,
    nodes: &mut u32,
    hits: &mut Vec<Hit>,
) {
    if depth > MAX_DEPTH || *nodes >= MAX_NODES || hits.len() >= MAX_HITS {
        return;
    }
    *nodes += 1;
    if let Some(hit) = interesting(element) {
        hits.push(hit);
        if hits.len() >= MAX_HITS {
            return;
        }
    }
    let mut child = unsafe { walker.GetFirstChildElement(element) }.ok();
    while let Some(el) = child {
        walk(walker, &el, depth + 1, nodes, hits);
        if hits.len() >= MAX_HITS || *nodes >= MAX_NODES {
            return;
        }
        child = unsafe { walker.GetNextSiblingElement(&el) }.ok();
    }
}

fn interesting(element: &IUIAutomationElement) -> Option<Hit> {
    if unsafe { element.CurrentIsOffscreen() }.ok()?.as_bool() {
        return None;
    }
    let kind_id = unsafe { element.CurrentControlType() }.ok()?;
    if !is_interactive(kind_id) {
        return None;
    }
    let hit = super::uia::hit_from_element(element)?;
    if hit.name.is_empty() && hit.aid.is_empty() {
        return None;
    }
    Some(hit)
}

fn is_interactive(id: UIA_CONTROLTYPE_ID) -> bool {
    id == UIA_ButtonControlTypeId
        || id == UIA_EditControlTypeId
        || id == UIA_HyperlinkControlTypeId
        || id == UIA_MenuItemControlTypeId
        || id == UIA_ListItemControlTypeId
        || id == UIA_TreeItemControlTypeId
        || id == UIA_TabItemControlTypeId
        || id == UIA_CheckBoxControlTypeId
        || id == UIA_RadioButtonControlTypeId
        || id == UIA_ComboBoxControlTypeId
        || id == UIA_SplitButtonControlTypeId
        || id == UIA_DocumentControlTypeId
        || id == UIA_SliderControlTypeId
        || id == UIA_CustomControlTypeId
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_image_space_rect() {
        let hit = Hit {
            name: "保存".into(),
            kind: "button".into(),
            aid: "SaveButton".into(),
            left: 100,
            top: 40,
            width: 80,
            height: 24,
            click_x: 140,
            click_y: 52,
        };
        let line = format_hit(0, &hit, 100, 40, 0.5);
        assert_eq!(line, "[0] button \"保存\" aid=SaveButton img=(0,0) 40x12");
    }
}
