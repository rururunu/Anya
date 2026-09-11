//! UI Automation find / invoke / set-value by name or automation id.

use std::sync::{Mutex, OnceLock};

use windows::core::BSTR;
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationInvokePattern,
    IUIAutomationTreeWalker, IUIAutomationValuePattern, UIA_ButtonControlTypeId,
    UIA_CheckBoxControlTypeId, UIA_ComboBoxControlTypeId, UIA_DocumentControlTypeId,
    UIA_EditControlTypeId, UIA_HyperlinkControlTypeId, UIA_InvokePatternId,
    UIA_ListItemControlTypeId, UIA_MenuControlTypeId, UIA_MenuItemControlTypeId,
    UIA_RadioButtonControlTypeId, UIA_SplitButtonControlTypeId, UIA_TabItemControlTypeId,
    UIA_TextControlTypeId, UIA_TreeItemControlTypeId, UIA_ValuePatternId, UIA_CONTROLTYPE_ID,
};

use super::input::{mouse_click, type_text};
use super::windows::{find_hwnd, foreground_hwnd, hwnd_id, window_title};
use crate::core::tools::error::ToolError;

const MAX_DEPTH: u32 = 12;
const MAX_NODES: u32 = 400;
const MAX_HITS: usize = 20;

#[derive(Clone)]
pub(crate) struct Hit {
    pub name: String,
    pub kind: String,
    pub aid: String,
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
    pub click_x: i32,
    pub click_y: i32,
}

#[derive(Clone)]
struct LastFind {
    hwnd: usize,
    hits: Vec<Hit>,
}

fn last_find() -> &'static Mutex<Option<LastFind>> {
    static LAST: OnceLock<Mutex<Option<LastFind>>> = OnceLock::new();
    LAST.get_or_init(|| Mutex::new(None))
}

pub(crate) fn store_last(hwnd: HWND, hits: Vec<Hit>) {
    if let Ok(mut g) = last_find().lock() {
        *g = Some(LastFind {
            hwnd: hwnd.0 as usize,
            hits,
        });
    }
}

pub(crate) fn find_control(name: &str, title_filter: Option<&str>) -> Result<String, ToolError> {
    let needle = name.trim();
    if needle.is_empty() {
        return Err(ToolError::new("find_control needs name"));
    }
    let hwnd = resolve_hwnd(title_filter)?;
    let hits = with_uia(|automation| collect_hits(automation, hwnd, needle))?;
    store_last(hwnd, hits.clone());
    if hits.is_empty() {
        return Ok(format!(
            "No controls matching `{needle}` in `{}`. Screenshot the window or try a shorter name.",
            window_title(hwnd)
        ));
    }
    let lines: Vec<String> = hits
        .iter()
        .enumerate()
        .map(|(i, hit)| {
            let aid = if hit.aid.is_empty() {
                String::new()
            } else {
                format!(" aid={}", hit.aid)
            };
            format!(
                "[{i}] {} \"{}\"{aid} rect=({},{},{}x{})",
                hit.kind, hit.name, hit.left, hit.top, hit.width, hit.height
            )
        })
        .collect();
    Ok(format!(
        "Found {} in `{}` id={}:\n{}",
        hits.len(),
        window_title(hwnd),
        hwnd_id(hwnd),
        lines.join("\n")
    ))
}

pub(crate) fn click_control(
    name: Option<&str>,
    id: Option<&str>,
    index: Option<u64>,
) -> Result<(String, i32, i32), ToolError> {
    let hit = resolve_hit(name, id, index)?;
    super::windows::prepare_click_target(hit.click_x, hit.click_y);
    let msg = if try_invoke(&hit) {
        format!("invoked `{}` ({})", hit.name, hit.kind)
    } else {
        mouse_click(hit.click_x, hit.click_y, "left", 1, &[])?;
        format!(
            "clicked `{}` ({}) at screen ({},{})",
            hit.name, hit.kind, hit.click_x, hit.click_y
        )
    };
    Ok((msg, hit.click_x, hit.click_y))
}

pub(crate) fn set_value(
    name: Option<&str>,
    id: Option<&str>,
    index: Option<u64>,
    value: &str,
) -> Result<String, ToolError> {
    if value.chars().count() > 4000 {
        return Err(ToolError::new("set_value text is too long (max 4000 chars)"));
    }
    let hit = resolve_hit(name, id, index)?;
    if try_set_value(&hit, value) {
        return Ok(format!("set `{}` ({}) via ValuePattern", hit.name, hit.kind));
    }
    super::windows::prepare_click_target(hit.click_x, hit.click_y);
    mouse_click(hit.click_x, hit.click_y, "left", 1, &[])?;
    type_text(value)?;
    Ok(format!(
        "set `{}` ({}) by click+type (no ValuePattern)",
        hit.name, hit.kind
    ))
}

fn resolve_hwnd(window_title: Option<&str>) -> Result<HWND, ToolError> {
    match window_title.map(str::trim).filter(|s| !s.is_empty()) {
        Some(title) => find_hwnd(Some(title), None),
        None => foreground_hwnd(),
    }
}

fn resolve_hit(
    name: Option<&str>,
    id: Option<&str>,
    index: Option<u64>,
) -> Result<Hit, ToolError> {
    if let Some(idx) = index {
        let g = last_find()
            .lock()
            .ok()
            .and_then(|g| g.clone())
            .ok_or_else(|| ToolError::new("click_control index needs a prior screenshot or find_control"))?;
        return g
            .hits
            .get(idx as usize)
            .cloned()
            .ok_or_else(|| ToolError::new(format!("no control at index {idx}")));
    }
    let id = id.map(str::trim).filter(|s| !s.is_empty());
    let name = name.map(str::trim).filter(|s| !s.is_empty());
    if id.is_none() && name.is_none() {
        return Err(ToolError::new("needs name, id, or index"));
    }
    if let Some(hit) = cached_hit(name, id) {
        return Ok(hit);
    }
    let hwnd = foreground_hwnd()?;
    let needle = id.or(name).unwrap_or("");
    let hits = with_uia(|automation| collect_hits(automation, hwnd, needle))?;
    store_last(hwnd, hits.clone());
    hits.into_iter()
        .next()
        .ok_or_else(|| ToolError::new(format!("no control matching `{needle}`")))
}

fn cached_hit(name: Option<&str>, id: Option<&str>) -> Option<Hit> {
    let last = last_find().lock().ok().and_then(|g| g.clone())?;
    last.hits.into_iter().find(|h| hit_matches(h, name, id))
}

fn hit_matches(hit: &Hit, name: Option<&str>, id: Option<&str>) -> bool {
    if let Some(id) = id {
        let lower = id.to_lowercase();
        if !hit.aid.is_empty() && hit.aid.to_lowercase() == lower {
            return true;
        }
        if hit.name.to_lowercase().contains(&lower) {
            return true;
        }
    }
    if let Some(name) = name {
        let lower = name.to_lowercase();
        if hit.name.to_lowercase().contains(&lower) {
            return true;
        }
        if !hit.aid.is_empty() && hit.aid.to_lowercase().contains(&lower) {
            return true;
        }
    }
    false
}

fn try_invoke(hit: &Hit) -> bool {
    let needle = if hit.aid.is_empty() {
        hit.name.as_str()
    } else {
        hit.aid.as_str()
    };
    with_uia(|automation| {
        let hwnd = last_hwnd()?;
        let root = unsafe { automation.ElementFromHandle(hwnd) }
            .map_err(|e| ToolError::new(format!("ElementFromHandle: {e}")))?;
        let walker = unsafe { automation.ControlViewWalker() }
            .map_err(|e| ToolError::new(format!("ControlViewWalker: {e}")))?;
        invoke_matching(&walker, &root, needle, hit, 0, &mut 0)
            .then_some(())
            .ok_or_else(|| ToolError::new("invoke miss"))
    })
    .is_ok()
}

fn try_set_value(hit: &Hit, value: &str) -> bool {
    let needle = if hit.aid.is_empty() {
        hit.name.as_str()
    } else {
        hit.aid.as_str()
    };
    with_uia(|automation| {
        let hwnd = last_hwnd()?;
        let root = unsafe { automation.ElementFromHandle(hwnd) }
            .map_err(|e| ToolError::new(format!("ElementFromHandle: {e}")))?;
        let walker = unsafe { automation.ControlViewWalker() }
            .map_err(|e| ToolError::new(format!("ControlViewWalker: {e}")))?;
        set_matching(&walker, &root, needle, hit, value, 0, &mut 0)
            .then_some(())
            .ok_or_else(|| ToolError::new("value miss"))
    })
    .is_ok()
}

fn last_hwnd() -> Result<HWND, ToolError> {
    last_find()
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|l| l.hwnd))
        .filter(|h| *h != 0)
        .map(|h| HWND(h as *mut core::ffi::c_void))
        .ok_or_else(|| ToolError::new("no window"))
}

fn invoke_matching(
    walker: &IUIAutomationTreeWalker,
    element: &IUIAutomationElement,
    needle: &str,
    hit: &Hit,
    depth: u32,
    nodes: &mut u32,
) -> bool {
    if depth > MAX_DEPTH || *nodes >= MAX_NODES {
        return false;
    }
    *nodes += 1;
    if element_matches(element, needle, hit) {
        if let Ok(pattern) = unsafe {
            element.GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId)
        } {
            if unsafe { pattern.Invoke() }.is_ok() {
                return true;
            }
        }
    }
    let mut child = unsafe { walker.GetFirstChildElement(element) }.ok();
    while let Some(el) = child {
        if invoke_matching(walker, &el, needle, hit, depth + 1, nodes) {
            return true;
        }
        child = unsafe { walker.GetNextSiblingElement(&el) }.ok();
    }
    false
}

fn set_matching(
    walker: &IUIAutomationTreeWalker,
    element: &IUIAutomationElement,
    needle: &str,
    hit: &Hit,
    value: &str,
    depth: u32,
    nodes: &mut u32,
) -> bool {
    if depth > MAX_DEPTH || *nodes >= MAX_NODES {
        return false;
    }
    *nodes += 1;
    if element_matches(element, needle, hit) {
        if let Ok(pattern) =
            unsafe { element.GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId) }
        {
            if unsafe { pattern.SetValue(&BSTR::from(value)) }.is_ok() {
                return true;
            }
        }
    }
    let mut child = unsafe { walker.GetFirstChildElement(element) }.ok();
    while let Some(el) = child {
        if set_matching(walker, &el, needle, hit, value, depth + 1, nodes) {
            return true;
        }
        child = unsafe { walker.GetNextSiblingElement(&el) }.ok();
    }
    false
}

fn element_matches(element: &IUIAutomationElement, needle: &str, hit: &Hit) -> bool {
    let lower = needle.to_lowercase();
    if let Ok(aid) = unsafe { element.CurrentAutomationId() } {
        let aid = aid.to_string();
        if !aid.is_empty() && (aid.eq_ignore_ascii_case(needle) || aid == hit.aid) {
            return true;
        }
    }
    if let Ok(name) = unsafe { element.CurrentName() } {
        let name = name.to_string();
        if !lower.is_empty() && name.to_lowercase().contains(&lower) {
            return true;
        }
        if !hit.name.is_empty() && name == hit.name {
            return true;
        }
    }
    false
}

fn collect_hits(
    automation: &IUIAutomation,
    hwnd: HWND,
    needle: &str,
) -> Result<Vec<Hit>, ToolError> {
    let root = unsafe { automation.ElementFromHandle(hwnd) }
        .map_err(|e| ToolError::new(format!("ElementFromHandle: {e}")))?;
    let walker = unsafe { automation.ControlViewWalker() }
        .map_err(|e| ToolError::new(format!("ControlViewWalker: {e}")))?;
    let mut hits = Vec::new();
    let mut nodes = 0;
    walk(
        &walker,
        &root,
        &needle.to_lowercase(),
        0,
        &mut nodes,
        &mut hits,
    );
    Ok(hits)
}

fn walk(
    walker: &IUIAutomationTreeWalker,
    element: &IUIAutomationElement,
    needle: &str,
    depth: u32,
    nodes: &mut u32,
    hits: &mut Vec<Hit>,
) {
    if depth > MAX_DEPTH || *nodes >= MAX_NODES || hits.len() >= MAX_HITS {
        return;
    }
    *nodes += 1;
    if let Some(hit) = hit_from_element(element) {
        if hit_matches_needle(&hit, needle) {
            hits.push(hit);
            if hits.len() >= MAX_HITS {
                return;
            }
        }
    }
    let mut child = unsafe { walker.GetFirstChildElement(element) }.ok();
    while let Some(el) = child {
        walk(walker, &el, needle, depth + 1, nodes, hits);
        if hits.len() >= MAX_HITS || *nodes >= MAX_NODES {
            return;
        }
        child = unsafe { walker.GetNextSiblingElement(&el) }.ok();
    }
}

fn hit_matches_needle(hit: &Hit, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    hit.name.to_lowercase().contains(needle) || hit.aid.to_lowercase().contains(needle)
}

pub(crate) fn hit_from_element(element: &IUIAutomationElement) -> Option<Hit> {
    let name = unsafe { element.CurrentName() }
        .ok()
        .map(|s| s.to_string())
        .unwrap_or_default();
    let aid = unsafe { element.CurrentAutomationId() }
        .ok()
        .map(|s| s.to_string())
        .unwrap_or_default();
    if name.is_empty() && aid.is_empty() {
        return None;
    }
    let kind = control_kind(unsafe { element.CurrentControlType() }.ok()?);
    let rect = unsafe { element.CurrentBoundingRectangle() }.ok()?;
    let (click_x, click_y) = clickable_or_center(element, &rect);
    Some(Hit {
        name,
        kind: kind.into(),
        aid,
        left: rect.left,
        top: rect.top,
        width: (rect.right - rect.left).max(0),
        height: (rect.bottom - rect.top).max(0),
        click_x,
        click_y,
    })
}

fn clickable_or_center(element: &IUIAutomationElement, rect: &RECT) -> (i32, i32) {
    let mut pt = POINT::default();
    if let Ok(ok) = unsafe { element.GetClickablePoint(&mut pt) } {
        if ok.as_bool() {
            return (pt.x, pt.y);
        }
    }
    ((rect.left + rect.right) / 2, (rect.top + rect.bottom) / 2)
}

pub(crate) fn control_kind(id: UIA_CONTROLTYPE_ID) -> &'static str {
    if id == UIA_ButtonControlTypeId {
        "button"
    } else if id == UIA_EditControlTypeId {
        "edit"
    } else if id == UIA_HyperlinkControlTypeId {
        "link"
    } else if id == UIA_MenuItemControlTypeId {
        "menu-item"
    } else if id == UIA_MenuControlTypeId {
        "menu"
    } else if id == UIA_ListItemControlTypeId {
        "list-item"
    } else if id == UIA_TreeItemControlTypeId {
        "tree-item"
    } else if id == UIA_TabItemControlTypeId {
        "tab"
    } else if id == UIA_CheckBoxControlTypeId {
        "checkbox"
    } else if id == UIA_RadioButtonControlTypeId {
        "radio"
    } else if id == UIA_ComboBoxControlTypeId {
        "combo"
    } else if id == UIA_SplitButtonControlTypeId {
        "split-button"
    } else if id == UIA_DocumentControlTypeId {
        "document"
    } else if id == UIA_TextControlTypeId {
        "text"
    } else {
        "other"
    }
}

pub(crate) fn with_uia<T>(f: impl FnOnce(&IUIAutomation) -> Result<T, ToolError>) -> Result<T, ToolError> {
    let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok();
    let result = (|| {
        let automation: IUIAutomation =
            unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) }
                .map_err(|e| ToolError::new(format!("UI Automation: {e}")))?;
        f(&automation)
    })();
    if initialized {
        unsafe { CoUninitialize() };
    }
    result
}
