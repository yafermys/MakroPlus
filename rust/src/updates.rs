//! Release check: reads the latest stable GitHub release on every start.
//! Read-only; nothing is downloaded or installed automatically.
use crate::i18n::{t, ts};
use parking_lot::Mutex;
use serde_json::{json, Value};

const API: &str = "https://api.github.com/repos/yafermys/MakroPlus/releases/latest";
pub const RELEASES: &str = "https://github.com/yafermys/MakroPlus/releases";

#[derive(Clone, Debug)]
pub struct Available {
    pub tag: String,
    pub url: String,
    pub notes: String,
}

#[derive(Clone, Debug, Default)]
pub struct Status {
    pub checking: bool,
    pub available: Option<Available>,
    pub message: String,
}

static STATUS: Mutex<Option<Status>> = Mutex::new(None);

pub fn status() -> Status {
    STATUS.lock().clone().unwrap_or_default()
}

fn set(f: impl FnOnce(&mut Status)) {
    let mut g = STATUS.lock();
    let s = g.get_or_insert_with(Status::default);
    f(s);
}

/// "v3.1.0-beta.2" → ([3,1,0,0], "beta.2")
fn parse(v: &str) -> Option<([u32; 4], String)> {
    let v = v.trim().trim_start_matches(['v', 'V']);
    let (core, pre) = match v.split_once('-') {
        Some((c, p)) => (c, p.to_string()),
        None => (v, String::new()),
    };
    let mut n = [0u32; 4];
    for (i, part) in core.split('.').enumerate() {
        if i >= 4 {
            return None;
        }
        n[i] = part.parse().ok()?;
    }
    Some((n, pre))
}

pub fn newer(remote: &str, local: &str) -> bool {
    match (parse(remote), parse(local)) {
        (Some((a, ap)), Some((b, bp))) => a > b || (a == b && !bp.is_empty() && ap.is_empty()),
        _ => false,
    }
}

pub fn enabled(st: &crate::model::State) -> bool {
    st.updates.get("Enabled").and_then(|v| v.as_bool()).unwrap_or(true)
}

pub fn set_enabled(st: &mut crate::model::State, on: bool) {
    if !st.updates.is_object() {
        st.updates = json!({});
    }
    st.updates["Enabled"] = json!(on);
}

#[cfg(not(feature = "net"))]
fn fetch() -> Result<Option<Available>, String> {
    Err("no network support".into())
}

#[cfg(feature = "net")]
fn fetch() -> Result<Option<Available>, String> {
    let resp = crate::llm::agent(std::time::Duration::from_secs(20))
        .get(API)
        .set("User-Agent", "MakroPlus-update-check")
        .set("Accept", "application/vnd.github+json")
        .call();
    let body: Value = match resp {
        Ok(r) => r.into_json().map_err(|e| e.to_string())?,
        Err(ureq::Error::Status(404, _)) => return Ok(None),
        Err(ureq::Error::Status(code, _)) => return Err(format!("HTTP {code}")),
        Err(e) => return Err(e.to_string()),
    };
    if body["draft"].as_bool().unwrap_or(false) || body["prerelease"].as_bool().unwrap_or(false) {
        return Ok(None);
    }
    let tag = body["tag_name"].as_str().unwrap_or("").to_string();
    if tag.is_empty() || !newer(&tag, env!("CARGO_PKG_VERSION")) {
        return Ok(None);
    }
    let url = body["html_url"].as_str().filter(|u| u.starts_with("https://github.com/")).unwrap_or(RELEASES).to_string();
    Ok(Some(Available { tag, url, notes: crate::model::short(body["body"].as_str().unwrap_or(""), 400) }))
}

/// Check in the background; `notify` shows a tray notification for a new version.
pub fn check_async(notify: bool) {
    if status().checking {
        return;
    }
    set(|s| {
        s.checking = true;
        s.message = t("Checking for updates…", "Güncellemeler denetleniyor…").into();
    });
    crate::send_ui(crate::UiEvent::UpdateChecked);
    std::thread::spawn(move || {
        let r = fetch();
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
        let mut tray = None;
        {
            let mut st = crate::app_state().lock();
            if !st.updates.is_object() {
                st.updates = json!({"Enabled": true});
            }
            st.updates["LastCheckedUtc"] = json!(chrono::Utc::now().to_rfc3339());
            st.updates["LastCheckSucceeded"] = json!(r.is_ok());
            match &r {
                Ok(Some(a)) => {
                    st.updates["AvailableTag"] = json!(a.tag);
                    if notify && st.updates.get("LastNotifiedVersion").and_then(|v| v.as_str()) != Some(a.tag.as_str()) {
                        st.updates["LastNotifiedVersion"] = json!(a.tag);
                        tray = Some(a.tag.clone());
                    }
                }
                Ok(None) => {
                    if let Some(o) = st.updates.as_object_mut() {
                        o.remove("AvailableTag");
                    }
                }
                Err(_) => {}
            }
        }
        crate::request_save();
        set(|s| {
            s.checking = false;
            match r {
                Ok(Some(a)) => {
                    s.message = ts(format!("Makro+ {} is available ({now}).", a.tag), format!("Makro+ {} yayımlandı ({now}).", a.tag));
                    s.available = Some(a);
                }
                Ok(None) => {
                    s.available = None;
                    s.message = ts(format!("You have the latest version ({now})."), format!("En güncel sürümü kullanıyorsunuz ({now})."));
                }
                Err(e) => s.message = ts(format!("Update check failed: {e}"), format!("Güncelleme denetlenemedi: {e}")),
            }
        });
        if let Some(tag) = tray {
            crate::shell::notify("Makro+", &ts(format!("Makro+ {tag} is available. Open Makro+ to download it."), format!("Makro+ {tag} yayımlandı. İndirmek için Makro+'ı açın.")));
        }
        crate::send_ui(crate::UiEvent::UpdateChecked);
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn versions() {
        assert!(super::newer("v3.1.0", "3.0.0"));
        assert!(super::newer("3.0.1", "3.0.0"));
        assert!(!super::newer("v2.8.0", "3.0.0"));
        assert!(!super::newer("v3.0.0", "3.0.0"));
        assert!(super::newer("3.0.0", "3.0.0-beta.1"));
        assert!(!super::newer("3.0.0-beta.2", "3.0.0"));
        assert!(!super::newer("garbage", "3.0.0"));
    }
}
