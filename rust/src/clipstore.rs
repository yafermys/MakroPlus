//! Clipboard history (shared by all platforms).
use crate::clipboard::{get_files, get_image, get_text, is_private, set_files, set_image, set_text};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    /// RGBA pixels (downscaled preview).
    pub rgba: Vec<u8>,
    /// Original DIB, to put back on the clipboard.
    pub dib: Vec<u8>,
}

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Entry {
    pub id: String,
    /// "text", "files" or "image".
    pub kind: String,
    pub text: String,
    pub files: Vec<String>,
    pub time: String,
    pub source: String,
    pub pinned: bool,
    #[serde(skip)]
    pub image: Option<Image>,
    #[serde(skip)]
    pub texture: Option<eframe::egui::TextureHandle>,
}

impl Entry {
    pub fn title(&self) -> String {
        match self.kind.as_str() {
            "files" => {
                let first = self.files.first().map(|f| crate::win::file_name(f)).unwrap_or_default();
                if self.files.len() > 1 {
                    format!("{first} +{}", self.files.len() - 1)
                } else {
                    first
                }
            }
            "image" => self.image.as_ref().map(|i| format!("{} {}×{}", crate::i18n::t("Image", "Resim"), i.width, i.height)).unwrap_or_else(|| crate::i18n::t("Image", "Resim").into()),
            _ => crate::model::short(&self.text, 120),
        }
    }
    pub fn restore(&self) -> Result<(), String> {
        match self.kind.as_str() {
            "files" => set_files(&self.files),
            "image" => self.image.as_ref().map(|i| set_image(&i.dib)).unwrap_or(Err("image no longer in memory".into())),
            _ => set_text(&self.text),
        }
    }
}

#[derive(Default)]
pub struct Store {
    pub entries: Vec<Entry>,
    pub ignore_sequence: u32,
    pub dirty: bool,
}

impl Store {
    pub fn load(path: &std::path::Path) -> Store {
        let entries = std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice::<Vec<Entry>>(b.strip_prefix(&[0xEF, 0xBB, 0xBF][..]).unwrap_or(&b)).ok())
            .unwrap_or_default();
        Store { entries, ignore_sequence: 0, dirty: false }
    }

    pub fn save_bytes(&self) -> Vec<u8> {
        let list: Vec<&Entry> = self.entries.iter().filter(|e| e.kind != "image").collect();
        serde_json::to_vec(&list).unwrap_or_default()
    }

    /// Capture the current clipboard into history.
    pub fn capture(&mut self, options: &crate::model::ClipboardOptions, source: String) -> bool {
        if !options.enabled || is_private() {
            return false;
        }
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let files = get_files();
        let entry = if !files.is_empty() {
            Entry { kind: "files".into(), files, ..Default::default() }
        } else if let Some(text) = get_text() {
            if text.trim().is_empty() || text.len() > 4 * 1024 * 1024 {
                return false;
            }
            Entry { kind: "text".into(), text, ..Default::default() }
        } else if options.capture_images {
            match get_image(256) {
                Some(img) => Entry { kind: "image".into(), image: Some(img), ..Default::default() },
                None => return false,
            }
        } else {
            return false;
        };
        // Same content again: move it to the top instead of duplicating.
        if let Some(i) = self.entries.iter().position(|e| e.kind == entry.kind && e.kind != "image" && e.text == entry.text && e.files == entry.files) {
            let mut old = self.entries.remove(i);
            old.time = now;
            old.source = source;
            self.entries.insert(0, old);
        } else {
            self.entries.insert(0, Entry { id: crate::win::uuid_v4(), time: now, source, ..entry });
        }
        self.trim(options.max_items);
        self.dirty = options.persist;
        true
    }

    pub fn trim(&mut self, max: usize) {
        let mut kept = 0;
        self.entries.retain(|e| {
            if e.pinned {
                return true;
            }
            kept += 1;
            kept <= max
        });
    }
}

