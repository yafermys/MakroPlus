//! Settings location, legacy import and atomic writes.
use crate::model::State;
use std::path::{Path, PathBuf};

pub fn exe_dir() -> PathBuf {
    std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())).unwrap_or_else(|| PathBuf::from("."))
}

/// Portable mode: a `portable.txt` next to the exe keeps all data in `data\`.
pub fn is_portable() -> bool {
    exe_dir().join("portable.txt").exists()
}

pub fn data_dir() -> PathBuf {
    if is_portable() {
        return exe_dir().join("data");
    }
    #[cfg(windows)]
    {
        let base = std::env::var("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(|_| exe_dir());
        base.join("MakroPlus")
    }
    #[cfg(not(windows))]
    {
        let base = std::env::var("XDG_CONFIG_HOME").map(PathBuf::from).ok().filter(|p| p.is_absolute()).unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into())).join(".config"));
        base.join("makroplus")
    }
}

pub fn settings_path() -> PathBuf {
    data_dir().join("settings.json")
}

pub fn clipboard_path() -> PathBuf {
    data_dir().join("clipboard.json")
}

/// Settings written by Makro+ 2.x.
pub fn legacy_path() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA").map(PathBuf::from).unwrap_or_default();
    base.join("AkilliKisayol").join("settings.json")
}

pub struct Loaded {
    pub state: State,
    pub notice: String,
}

fn parse(path: &Path) -> Result<State, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("settings file exceeds 64 MB".into());
    }
    let text = String::from_utf8_lossy(&bytes);
    let text = text.trim_start_matches('\u{feff}');
    let mut state: State = serde_json::from_str(text).map_err(|e| e.to_string())?;
    state.normalize();
    Ok(state)
}

pub fn load() -> Loaded {
    use crate::i18n::t;
    let path = settings_path();
    if path.exists() {
        match parse(&path) {
            Ok(state) => return Loaded { state, notice: String::new() },
            Err(err) => {
                let stamp = chrono::Local::now().format("%Y%m%d%H%M%S");
                let copy = path.with_extension(format!("corrupt-{stamp}.json"));
                let _ = std::fs::copy(&path, &copy);
                let bak = path.with_extension("json.bak");
                if let Ok(mut state) = parse(&bak) {
                    state.paused = true;
                    let _ = save(&state);
                    return Loaded {
                        state,
                        notice: format!(
                            "{} {}\n{}: {}",
                            t("Settings could not be read; the previous backup was restored and shortcuts are paused.", "Ayarlar okunamadı; önceki yedek geri yüklendi ve kısayollar duraklatıldı."),
                            err,
                            t("Damaged copy", "Bozuk kopya"),
                            copy.display()
                        ),
                    };
                }
                let state = State { paused: true, ..State::default() };
                let _ = save(&state);
                return Loaded {
                    state,
                    notice: format!("{} {}", t("Settings and backup could not be read. A paused empty session was started. Damaged file:", "Ayarlar ve yedek okunamadı. Duraklatılmış boş bir oturum başlatıldı. Bozuk dosya:"), copy.display()),
                };
            }
        }
    }
    let legacy = legacy_path();
    if legacy.exists() {
        if let Ok(mut state) = parse(&legacy) {
            let count = state.bindings.len();
            state.extra.insert("ImportedFrom".into(), serde_json::Value::String(legacy.display().to_string()));
            if state.language.is_empty() {
                state.language = "tr".into();
            }
            let _ = save(&state);
            return Loaded {
                state,
                notice: crate::i18n::ts(
                    format!("Imported {count} shortcuts from Makro+ 2.x. Your old settings were left unchanged."),
                    format!("Makro+ 2.x sürümünden {count} kısayol içe aktarıldı. Eski ayarlarınız değiştirilmedi."),
                ),
            };
        }
    }
    Loaded { state: State::default(), notice: String::new() }
}

pub fn serialize(state: &State) -> Result<Vec<u8>, String> {
    serde_json::to_vec_pretty(state).map_err(|e| e.to_string())
}

/// Atomic write: temp file, previous version kept as `.bak`.
pub fn write_atomic(path: &Path, data: &[u8]) -> Result<(), String> {
    use std::io::Write;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension(format!("{}.tmp", crate::win::uuid_v4().replace('-', "")));
    {
        let mut f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
        f.write_all(data).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
    }
    if path.exists() {
        let bak = PathBuf::from(format!("{}.bak", path.display()));
        let _ = std::fs::copy(path, &bak);
    }
    let r = std::fs::rename(&tmp, path).map_err(|e| e.to_string());
    if r.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    r
}

pub fn save(state: &State) -> Result<(), String> {
    let data = serialize(state)?;
    write_atomic(&settings_path(), &data)
}

/// Resolve a model path: relative paths are relative to the exe folder.
pub fn resolve_model(path: &str) -> String {
    if path.trim().is_empty() {
        let dir = exe_dir().join("models");
        if let Ok(rd) = std::fs::read_dir(&dir) {
            let found: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.extension().map(|x| x.eq_ignore_ascii_case("gguf")).unwrap_or(false)).collect();
            if found.len() == 1 {
                return found[0].display().to_string();
            }
        }
        return String::new();
    }
    let p = PathBuf::from(path);
    if p.is_absolute() {
        path.to_string()
    } else {
        exe_dir().join(p).display().to_string()
    }
}

pub fn model_candidates() -> Vec<String> {
    let mut out = Vec::new();
    for dir in [exe_dir().join("models"), data_dir().join("models")] {
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().map(|x| x.eq_ignore_ascii_case("gguf")).unwrap_or(false) {
                    out.push(p.display().to_string());
                }
            }
        }
    }
    out
}
