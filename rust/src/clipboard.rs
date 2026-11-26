//! Clipboard access and the Makro+ clipboard history.
use crate::win::{from_wide, wide};
pub use crate::clipstore::{Entry, Image, Store};
use std::ffi::c_void;
use std::ptr::null_mut;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::DataExchange::*;
use windows_sys::Win32::System::Memory::*;

const CF_UNICODETEXT: u32 = 13;
const CF_HDROP: u32 = 15;
const CF_DIB: u32 = 8;

struct Open;
impl Open {
    fn new() -> Option<Open> {
        for _ in 0..20 {
            if unsafe { OpenClipboard(null_mut()) } != 0 {
                return Some(Open);
            }
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
        None
    }
}
impl Drop for Open {
    fn drop(&mut self) {
        unsafe { CloseClipboard() };
    }
}

pub fn sequence() -> u32 {
    unsafe { GetClipboardSequenceNumber() }
}

fn format_id(name: &str) -> u32 {
    unsafe { RegisterClipboardFormatW(wide(name).as_ptr()) }
}

/// Password managers mark secrets with these formats; such content is never stored.
pub fn is_private() -> bool {
    unsafe {
        if IsClipboardFormatAvailable(format_id("ExcludeClipboardContentFromMonitorProcessing")) != 0 || IsClipboardFormatAvailable(format_id("Clipboard Viewer Ignore")) != 0 {
            return true;
        }
        let can = format_id("CanIncludeInClipboardHistory");
        if IsClipboardFormatAvailable(can) != 0 {
            if let Some(_g) = Open::new() {
                let hnd = GetClipboardData(can);
                if !hnd.is_null() {
                    let p = GlobalLock(hnd) as *const u32;
                    if !p.is_null() {
                        let v = *p;
                        GlobalUnlock(hnd);
                        return v == 0;
                    }
                }
            }
        }
    }
    false
}

pub fn get_text() -> Option<String> {
    unsafe {
        if IsClipboardFormatAvailable(CF_UNICODETEXT) == 0 {
            return None;
        }
        let _g = Open::new()?;
        let hnd = GetClipboardData(CF_UNICODETEXT);
        if hnd.is_null() {
            return None;
        }
        let p = GlobalLock(hnd) as *const u16;
        if p.is_null() {
            return None;
        }
        let size = GlobalSize(hnd) / 2;
        let slice = std::slice::from_raw_parts(p, size);
        let s = from_wide(slice);
        GlobalUnlock(hnd);
        Some(s)
    }
}

pub fn set_text(text: &str) -> Result<(), String> {
    unsafe {
        let _g = Open::new().ok_or("clipboard busy")?;
        EmptyClipboard();
        let w = wide(text);
        let bytes = w.len() * 2;
        let mem = GlobalAlloc(GMEM_MOVEABLE, bytes);
        if mem.is_null() {
            return Err("GlobalAlloc failed".into());
        }
        let p = GlobalLock(mem) as *mut u16;
        std::ptr::copy_nonoverlapping(w.as_ptr(), p, w.len());
        GlobalUnlock(mem);
        if SetClipboardData(CF_UNICODETEXT, mem as HANDLE).is_null() {
            GlobalFree(mem);
            return Err("SetClipboardData failed".into());
        }
        Ok(())
    }
}

pub fn get_files() -> Vec<String> {
    use windows_sys::Win32::UI::Shell::{DragQueryFileW, HDROP};
    let mut out = Vec::new();
    unsafe {
        if IsClipboardFormatAvailable(CF_HDROP) == 0 {
            return out;
        }
        let Some(_g) = Open::new() else { return out };
        let hnd = GetClipboardData(CF_HDROP);
        if hnd.is_null() {
            return out;
        }
        let drop = hnd as HDROP;
        let n = DragQueryFileW(drop, 0xFFFF_FFFF, null_mut(), 0);
        for i in 0..n.min(500) {
            let len = DragQueryFileW(drop, i, null_mut(), 0);
            let mut buf = vec![0u16; len as usize + 1];
            DragQueryFileW(drop, i, buf.as_mut_ptr(), buf.len() as u32);
            out.push(from_wide(&buf));
        }
    }
    out
}

pub fn get_image(max_preview: usize) -> Option<Image> {
    unsafe {
        if IsClipboardFormatAvailable(CF_DIB) == 0 {
            return None;
        }
        let _g = Open::new()?;
        let hnd = GetClipboardData(CF_DIB);
        if hnd.is_null() {
            return None;
        }
        let p = GlobalLock(hnd) as *const u8;
        if p.is_null() {
            return None;
        }
        let size = GlobalSize(hnd);
        let dib = std::slice::from_raw_parts(p, size).to_vec();
        GlobalUnlock(hnd);
        dib_preview(&dib, max_preview)
    }
}

fn dib_preview(dib: &[u8], max: usize) -> Option<Image> {
    if dib.len() < 40 {
        return None;
    }
    let rd32 = |o: usize| i32::from_le_bytes([dib[o], dib[o + 1], dib[o + 2], dib[o + 3]]);
    let rd16 = |o: usize| u16::from_le_bytes([dib[o], dib[o + 1]]);
    let header = rd32(0) as usize;
    let w = rd32(4);
    let hgt = rd32(8);
    let bpp = rd16(14) as usize;
    let compression = rd32(16);
    if w <= 0 || hgt == 0 || !(bpp == 24 || bpp == 32) || !(compression == 0 || compression == 3) {
        return None;
    }
    let (w, top_down, ah) = (w as usize, hgt < 0, hgt.unsigned_abs() as usize);
    let masks = if compression == 3 && header == 40 { 12 } else { 0 };
    let offset = header + masks;
    let stride = (w * bpp / 8).div_ceil(4) * 4;
    if dib.len() < offset + stride * ah {
        return None;
    }
    let scale = (w.max(ah) as f32 / max as f32).max(1.0);
    let pw = ((w as f32 / scale) as usize).max(1);
    let ph = ((ah as f32 / scale) as usize).max(1);
    let mut rgba = vec![0u8; pw * ph * 4];
    for y in 0..ph {
        let sy = ((y as f32 * scale) as usize).min(ah - 1);
        let row = if top_down { sy } else { ah - 1 - sy };
        for x in 0..pw {
            let sx = ((x as f32 * scale) as usize).min(w - 1);
            let o = offset + row * stride + sx * bpp / 8;
            let i = (y * pw + x) * 4;
            rgba[i] = dib[o + 2];
            rgba[i + 1] = dib[o + 1];
            rgba[i + 2] = dib[o];
            rgba[i + 3] = 255;
        }
    }
    Some(Image { width: pw, height: ph, rgba, dib: dib.to_vec() })
}

pub fn set_image(dib: &[u8]) -> Result<(), String> {
    unsafe {
        let _g = Open::new().ok_or("clipboard busy")?;
        EmptyClipboard();
        let mem = GlobalAlloc(GMEM_MOVEABLE, dib.len());
        if mem.is_null() {
            return Err("GlobalAlloc failed".into());
        }
        let p = GlobalLock(mem) as *mut u8;
        std::ptr::copy_nonoverlapping(dib.as_ptr(), p, dib.len());
        GlobalUnlock(mem);
        if SetClipboardData(CF_DIB, mem as HANDLE).is_null() {
            GlobalFree(mem);
            return Err("SetClipboardData failed".into());
        }
        Ok(())
    }
}

pub fn set_files(files: &[String]) -> Result<(), String> {
    // DROPFILES header (20 bytes) followed by a double-null-terminated wide list.
    let mut list: Vec<u16> = Vec::new();
    for f in files {
        list.extend(f.encode_utf16());
        list.push(0);
    }
    list.push(0);
    let bytes = 20 + list.len() * 2;
    unsafe {
        let _g = Open::new().ok_or("clipboard busy")?;
        EmptyClipboard();
        let mem = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, bytes);
        if mem.is_null() {
            return Err("GlobalAlloc failed".into());
        }
        let p = GlobalLock(mem) as *mut u8;
        *(p as *mut u32) = 20; // pFiles
        *(p.add(16) as *mut u32) = 1; // fWide
        std::ptr::copy_nonoverlapping(list.as_ptr() as *const u8, p.add(20), list.len() * 2);
        GlobalUnlock(mem);
        if SetClipboardData(CF_HDROP, mem as HANDLE).is_null() {
            GlobalFree(mem);
            return Err("SetClipboardData failed".into());
        }
        Ok(())
    }
}

