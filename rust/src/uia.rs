//! In-app automation: Win32 menus, UI Automation elements, packaged-app activation.
use crate::i18n::t;
use crate::model::{AppCommand, AppInventory};
use crate::win::Hwnd;
use windows::core::{Interface, BSTR, HSTRING, VARIANT};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::*;
use windows::Win32::UI::Accessibility::*;

/// Initialise COM (MTA) for the calling thread once.
pub fn com_init() {
    thread_local!(static INIT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) });
    INIT.with(|i| {
        if !i.get() {
            unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            }
            i.set(true);
        }
    });
}

fn automation() -> Result<IUIAutomation, String> {
    com_init();
    unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).map_err(|e| e.to_string()) }
}

fn hwnd(w: Hwnd) -> HWND {
    HWND(w as *mut core::ffi::c_void)
}

pub fn activate_package(app_user_model_id: &str, args: &str) -> Result<(), String> {
    com_init();
    unsafe {
        let mgr: windows::Win32::UI::Shell::IApplicationActivationManager =
            CoCreateInstance(&windows::Win32::UI::Shell::ApplicationActivationManager, None, CLSCTX_LOCAL_SERVER).map_err(|e| e.to_string())?;
        mgr.ActivateApplication(&HSTRING::from(app_user_model_id), &HSTRING::from(args), windows::Win32::UI::Shell::AO_NONE)
            .map(|_| ())
            .map_err(|e| format!("{}: {e}", t("Packaged app could not be started", "Paket uygulaması başlatılamadı")))
    }
}

// ---------------- Win32 menus ----------------

pub fn clean_label(s: &str) -> String {
    let main = s.split('\t').next().unwrap_or("");
    let mut out = String::new();
    let mut chars = main.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            if chars.peek() == Some(&'&') {
                out.push('&');
                chars.next();
            }
            continue;
        }
        out.push(c);
    }
    out.trim().trim_end_matches("...").trim_end_matches('…').trim().to_string()
}

fn same_label(a: &str, b: &str) -> bool {
    clean_label(a).to_lowercase() == clean_label(b).to_lowercase()
}

struct MenuItem {
    label: String,
    accel: String,
    id: u32,
    sub: isize,
    disabled: bool,
}

fn menu_items(menu: isize) -> Vec<MenuItem> {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    let mut out = Vec::new();
    unsafe {
        let m = menu as HMENU;
        let n = GetMenuItemCount(m);
        for i in 0..n.max(0).min(200) {
            let mut info: MENUITEMINFOW = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as u32;
            info.fMask = MIIM_STRING | MIIM_ID | MIIM_SUBMENU | MIIM_FTYPE | MIIM_STATE;
            if GetMenuItemInfoW(m, i as u32, 1, &mut info) == 0 || info.fType & MFT_SEPARATOR != 0 {
                continue;
            }
            let mut text = String::new();
            if info.cch > 0 {
                let mut buf = vec![0u16; info.cch as usize + 1];
                info.dwTypeData = buf.as_mut_ptr();
                info.cch += 1;
                if GetMenuItemInfoW(m, i as u32, 1, &mut info) != 0 {
                    text = crate::win::from_wide(&buf);
                }
            }
            if text.trim().is_empty() {
                continue;
            }
            let accel = text.split('\t').nth(1).unwrap_or("").trim().to_string();
            out.push(MenuItem { label: clean_label(&text), accel, id: info.wID, sub: info.hSubMenu as isize, disabled: info.fState & MFS_DISABLED != 0 });
        }
    }
    out
}

fn walk_menu(menu: isize, path: &str, depth: usize, out: &mut Vec<AppCommand>) {
    if depth > 5 || out.len() > 800 {
        return;
    }
    for item in menu_items(menu) {
        let full = if path.is_empty() { item.label.clone() } else { format!("{path} › {}", item.label) };
        if item.sub != 0 {
            walk_menu(item.sub, &full, depth + 1, out);
        } else {
            out.push(AppCommand { source: "menu".into(), label: full, accelerator: item.accel, role: if item.disabled { "disabled".into() } else { String::new() }, ..Default::default() });
        }
    }
}

fn window_menu(w: Hwnd) -> isize {
    unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetMenu(w as _) as isize }
}

/// Resolve a menu path ("File › Save As") to its command id.
fn resolve_menu(w: Hwnd, path: &str) -> Option<u32> {
    let parts: Vec<&str> = path.split(['›', '>']).map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    let mut menu = window_menu(w);
    if menu == 0 || parts.is_empty() {
        return None;
    }
    for (i, part) in parts.iter().enumerate() {
        let items = menu_items(menu);
        let item = items.into_iter().find(|it| same_label(&it.label, part))?;
        if i == parts.len() - 1 {
            return if item.sub == 0 { Some(item.id) } else { None };
        }
        menu = item.sub;
        if menu == 0 {
            return None;
        }
    }
    None
}

/// Run a menu command: Win32 menus by WM_COMMAND, otherwise via UI Automation.
pub fn menu_command(w: Hwnd, path: &str) -> Result<(), String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_COMMAND};
    if let Some(id) = resolve_menu(w, path) {
        unsafe { PostMessageW(w as _, WM_COMMAND, id as usize, 0) };
        return Ok(());
    }
    let parts: Vec<String> = path.split(['›', '>']).map(|s| clean_label(s)).filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return Err(t("Empty menu path.", "Menü yolu boş.").into());
    }
    let uia = automation()?;
    unsafe {
        let window = uia.ElementFromHandle(hwnd(w)).map_err(|e| e.to_string())?;
        let root = uia.GetRootElement().map_err(|e| e.to_string())?;
        for (i, part) in parts.iter().enumerate() {
            let last = i == parts.len() - 1;
            let mut found = None;
            for _ in 0..10 {
                for scope_el in [&window, &root] {
                    if let Some(el) = find_named(&uia, scope_el, part, Some(UIA_MenuItemControlTypeId.0))? {
                        found = Some(el);
                        break;
                    }
                }
                if found.is_some() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(120));
            }
            let el = found.ok_or_else(|| format!("{}: {part}", t("Menu item not found", "Menü öğesi bulunamadı")))?;
            if last {
                invoke_any(&el)?;
            } else if let Ok(p) = el.GetCurrentPatternAs::<IUIAutomationExpandCollapsePattern>(UIA_ExpandCollapsePatternId) {
                let _ = p.Expand();
            } else {
                invoke_any(&el)?;
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    }
    Ok(())
}

// ---------------- UI Automation ----------------

unsafe fn bstr_prop(r: windows::core::Result<BSTR>) -> String {
    r.map(|b| b.to_string()).unwrap_or_default()
}

unsafe fn find_named(uia: &IUIAutomation, scope: &IUIAutomationElement, name: &str, control: Option<i32>) -> Result<Option<IUIAutomationElement>, String> {
    let mut cond = uia.CreatePropertyCondition(UIA_NamePropertyId, &VARIANT::from(BSTR::from(name))).map_err(|e| e.to_string())?;
    if let Some(ct) = control {
        let c2 = uia.CreatePropertyCondition(UIA_ControlTypePropertyId, &VARIANT::from(ct)).map_err(|e| e.to_string())?;
        cond = uia.CreateAndCondition(&cond, &c2).map_err(|e| e.to_string())?;
    }
    match scope.FindFirst(TreeScope_Descendants, &cond) {
        Ok(el) => Ok(Some(el)),
        Err(_) => Ok(None),
    }
}

unsafe fn invoke_any(el: &IUIAutomationElement) -> Result<(), String> {
    if let Ok(p) = el.GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId) {
        return p.Invoke().map_err(|e| e.to_string());
    }
    if let Ok(p) = el.GetCurrentPatternAs::<IUIAutomationTogglePattern>(UIA_TogglePatternId) {
        return p.Toggle().map_err(|e| e.to_string());
    }
    if let Ok(p) = el.GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(UIA_SelectionItemPatternId) {
        return p.Select().map_err(|e| e.to_string());
    }
    if let Ok(p) = el.GetCurrentPatternAs::<IUIAutomationExpandCollapsePattern>(UIA_ExpandCollapsePatternId) {
        return p.Expand().map_err(|e| e.to_string());
    }
    if let Ok(p) = el.GetCurrentPatternAs::<IUIAutomationLegacyIAccessiblePattern>(UIA_LegacyIAccessiblePatternId) {
        if p.DoDefaultAction().is_ok() {
            return Ok(());
        }
    }
    // Last resort: click the element's centre.
    let r = el.CurrentBoundingRectangle().map_err(|e| e.to_string())?;
    if r.right > r.left && r.bottom > r.top {
        return crate::win::mouse_at("Mouse click", 0, (r.left + r.right) / 2, (r.top + r.bottom) / 2, 0, 0, 0, false, 0);
    }
    Err(t("The element cannot be activated.", "Öğe etkinleştirilemiyor.").into())
}

/// Find exactly one enabled, visible element by automation id / name / control type.
unsafe fn locate(uia: &IUIAutomation, w: Hwnd, automation_id: &str, name: &str, control: i32) -> Result<IUIAutomationElement, String> {
    let root = uia.ElementFromHandle(hwnd(w)).map_err(|e| e.to_string())?;
    let mut cond = uia.CreatePropertyCondition(UIA_IsEnabledPropertyId, &VARIANT::from(true)).map_err(|e| e.to_string())?;
    if !automation_id.is_empty() {
        let c = uia.CreatePropertyCondition(UIA_AutomationIdPropertyId, &VARIANT::from(BSTR::from(automation_id))).map_err(|e| e.to_string())?;
        cond = uia.CreateAndCondition(&cond, &c).map_err(|e| e.to_string())?;
    }
    if !name.is_empty() {
        let c = uia.CreatePropertyCondition(UIA_NamePropertyId, &VARIANT::from(BSTR::from(name))).map_err(|e| e.to_string())?;
        cond = uia.CreateAndCondition(&cond, &c).map_err(|e| e.to_string())?;
    }
    if control > 0 {
        let c = uia.CreatePropertyCondition(UIA_ControlTypePropertyId, &VARIANT::from(control)).map_err(|e| e.to_string())?;
        cond = uia.CreateAndCondition(&cond, &c).map_err(|e| e.to_string())?;
    }
    let all = root.FindAll(TreeScope_Descendants, &cond).map_err(|e| e.to_string())?;
    let n = all.Length().unwrap_or(0);
    let mut visible = Vec::new();
    for i in 0..n {
        if let Ok(el) = all.GetElement(i) {
            if !el.CurrentIsOffscreen().map(|b| b.as_bool()).unwrap_or(false) {
                visible.push(el);
            }
        }
    }
    let label = if name.is_empty() { automation_id } else { name };
    match visible.len() {
        1 => Ok(visible.remove(0)),
        0 => Err(format!("{} “{label}”", t("Element not found:", "Öğe bulunamadı:"))),
        _ => {
            // Prefer an exact automation-id match when names repeat.
            if !automation_id.is_empty() {
                Ok(visible.remove(0))
            } else {
                Err(format!("{} “{label}”", t("Several elements match; the target is ambiguous:", "Birden çok öğe eşleşiyor; hedef belirsiz:")))
            }
        }
    }
}

pub fn click_element(w: Hwnd, automation_id: &str, name: &str, control: i32) -> Result<(), String> {
    let uia = automation()?;
    unsafe {
        let el = locate(&uia, w, automation_id, name, control)?;
        invoke_any(&el)
    }
}

pub fn set_field(w: Hwnd, automation_id: &str, name: &str, value: &str) -> Result<(), String> {
    let uia = automation()?;
    unsafe {
        let el = locate(&uia, w, automation_id, name, 0)?;
        if let Ok(p) = el.GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId) {
            if p.SetValue(&BSTR::from(value)).is_ok() {
                return Ok(());
            }
        }
        el.SetFocus().map_err(|e| e.to_string())?;
        std::thread::sleep(std::time::Duration::from_millis(60));
        crate::win::send_chord(2, 65, false)?;
        crate::win::type_text(value)
    }
}

pub fn read_field(w: Hwnd, automation_id: &str, name: &str) -> Result<String, String> {
    let uia = automation()?;
    unsafe {
        let el = locate(&uia, w, automation_id, name, 0)?;
        if let Ok(p) = el.GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId) {
            if let Ok(v) = p.CurrentValue() {
                return Ok(v.to_string());
            }
        }
        if let Ok(p) = el.GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId) {
            if let Ok(range) = p.DocumentRange() {
                if let Ok(text) = range.GetText(200_000) {
                    return Ok(text.to_string());
                }
            }
        }
        Ok(bstr_prop(el.CurrentName()))
    }
}

fn role_name(ct: i32) -> &'static str {
    match ct {
        50000 => "button",
        50002 => "checkbox",
        50003 => "combobox",
        50004 => "field",
        50005 => "link",
        50007 => "list item",
        50011 => "menu item",
        50013 => "radio",
        50019 => "tab",
        50031 => "split button",
        50030 => "document",
        50020 => "text",
        50024 => "tree item",
        _ => "",
    }
}

/// Collect useful elements of a window, bounded in count and depth.
fn scan_elements(w: Hwnd, out: &mut Vec<AppCommand>) -> Result<(), String> {
    let uia = automation()?;
    unsafe {
        let root = uia.ElementFromHandle(hwnd(w)).map_err(|e| e.to_string())?;
        let walker = uia.ControlViewWalker().map_err(|e| e.to_string())?;
        let mut stack: Vec<(IUIAutomationElement, usize, String)> = vec![(root, 0, String::new())];
        let mut visited = 0;
        while let Some((el, depth, ctx)) = stack.pop() {
            visited += 1;
            if visited > 2500 || out.len() > 600 {
                break;
            }
            let ct = el.CurrentControlType().map(|c| c.0).unwrap_or(0);
            let name = bstr_prop(el.CurrentName());
            let role = role_name(ct);
            let mut child_ctx = ctx.clone();
            if !role.is_empty() && ct != 50030 {
                let id = bstr_prop(el.CurrentAutomationId());
                let accel = bstr_prop(el.CurrentAcceleratorKey());
                let label = if name.trim().is_empty() { id.clone() } else { name.trim().to_string() };
                if !label.is_empty() && label.chars().count() <= 120 {
                    let full = if ctx.is_empty() { label.clone() } else { format!("{ctx} › {label}") };
                    if !out.iter().any(|c| c.label == full && c.automation_id == id) {
                        out.push(AppCommand { source: "element".into(), label: full, accelerator: accel, automation_id: id, control_type: ct, role: role.into() });
                    }
                }
            }
            if matches!(ct, 50033 | 50018 | 50021 | 50026 | 50009 | 50010) && !name.trim().is_empty() && name.chars().count() < 40 {
                // Panes, toolbars, groups, menus: carry their name as context.
                child_ctx = if ctx.is_empty() { name.trim().to_string() } else { format!("{ctx} › {}", name.trim()) };
            }
            if depth >= 14 {
                continue;
            }
            let mut children = Vec::new();
            let mut child = walker.GetFirstChildElement(&el).ok();
            let mut guard = 0;
            while let Some(c) = child {
                guard += 1;
                if guard > 300 {
                    break;
                }
                let next = walker.GetNextSiblingElement(&c).ok();
                children.push(c);
                child = next;
            }
            for c in children.into_iter().rev() {
                stack.push((c, depth + 1, child_ctx.clone()));
            }
        }
    }
    Ok(())
}

/// Scan a running application's window for menu commands and controls.
pub fn scan_app(app_path: &str) -> Result<AppInventory, String> {
    let wins = crate::win::app_windows(app_path);
    let fg = crate::win::foreground();
    let w = if wins.contains(&fg) { fg } else { *wins.first().ok_or_else(|| t("Open the application first; its window is needed for the scan.", "Önce uygulamayı açın; tarama için penceresi gerekli.").to_string())? };
    let mut commands = Vec::new();
    let menu = window_menu(w);
    if menu != 0 {
        walk_menu(menu, "", 0, &mut commands);
    }
    let _ = scan_elements(w, &mut commands);
    Ok(AppInventory {
        app_path: app_path.to_string(),
        scanned: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
        window_title: crate::win::window_title(w),
        commands,
    })
}

#[allow(dead_code)]
fn _iface_check(x: &IUIAutomationElement) -> windows::core::Result<IUIAutomationElement> {
    x.cast()
}
