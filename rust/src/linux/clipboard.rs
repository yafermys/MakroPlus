//! Linux clipboard through arboard (X11 and Wayland data-control).
pub use crate::clipstore::{Entry, Image, Store};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

static SEQ: AtomicU32 = AtomicU32::new(1);
static LAST: Mutex<Option<u64>> = Mutex::new(None);

fn board() -> Option<arboard::Clipboard> {
    arboard::Clipboard::new().ok()
}

fn hash(s: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

/// Linux has no global clipboard sequence number; it is derived from content changes
/// observed by [`poll_changed`] and [`sequence`].
pub fn sequence() -> u32 {
    poll_changed();
    SEQ.load(Ordering::SeqCst)
}

/// True when the clipboard content differs from the last observation.
pub fn poll_changed() -> bool {
    let Some(mut b) = board() else { return false };
    let h = match b.get_text() {
        Ok(t) => hash(t.as_bytes()),
        Err(_) => match b.get_image() {
            Ok(img) => hash(&img.bytes[..img.bytes.len().min(65536)]),
            Err(_) => 0,
        },
    };
    let mut last = LAST.lock();
    if *last != Some(h) {
        let first = last.is_none();
        *last = Some(h);
        if !first {
            SEQ.fetch_add(1, Ordering::SeqCst);
            return true;
        }
    }
    false
}

pub fn is_private() -> bool {
    false
}

pub fn get_text() -> Option<String> {
    board()?.get_text().ok()
}

pub fn set_text(text: &str) -> Result<(), String> {
    let mut b = board().ok_or("clipboard unavailable")?;
    b.set_text(text.to_string()).map_err(|e| e.to_string())?;
    // Keep ownership alive briefly on X11 so the receiving app can read it.
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(2));
        drop(b);
    });
    Ok(())
}

/// File lists are pasted as text/uri-list content (paths, one per line).
pub fn get_files() -> Vec<String> {
    get_text()
        .filter(|t| t.lines().all(|l| l.starts_with("file://")) && !t.trim().is_empty())
        .map(|t| t.lines().map(|l| l.trim_start_matches("file://").to_string()).collect())
        .unwrap_or_default()
}

pub fn set_files(files: &[String]) -> Result<(), String> {
    set_text(&files.iter().map(|f| format!("file://{f}")).collect::<Vec<_>>().join("\n"))
}

pub fn get_image(max_preview: usize) -> Option<Image> {
    let img = board()?.get_image().ok()?;
    let (w, h) = (img.width, img.height);
    let scale = (w.max(h) as f32 / max_preview as f32).max(1.0);
    let (pw, ph) = (((w as f32) / scale).max(1.0) as usize, ((h as f32) / scale).max(1.0) as usize);
    let mut rgba = vec![0u8; pw * ph * 4];
    for y in 0..ph {
        for x in 0..pw {
            let sx = ((x as f32 * scale) as usize).min(w - 1);
            let sy = ((y as f32 * scale) as usize).min(h - 1);
            let o = (sy * w + sx) * 4;
            let i = (y * pw + x) * 4;
            rgba[i..i + 4].copy_from_slice(&img.bytes[o..o + 4]);
        }
    }
    // Keep the full image (RGBA) so it can be restored.
    let mut dib = Vec::with_capacity(8 + img.bytes.len());
    dib.extend_from_slice(&(w as u32).to_le_bytes());
    dib.extend_from_slice(&(h as u32).to_le_bytes());
    dib.extend_from_slice(&img.bytes);
    Some(Image { width: pw, height: ph, rgba, dib })
}

pub fn set_image(data: &[u8]) -> Result<(), String> {
    if data.len() < 8 {
        return Err("invalid image".into());
    }
    let w = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let h = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
    let mut b = board().ok_or("clipboard unavailable")?;
    b.set_image(arboard::ImageData { width: w, height: h, bytes: std::borrow::Cow::Borrowed(&data[8..]) }).map_err(|e| e.to_string())
}
