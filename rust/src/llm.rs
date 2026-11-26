//! Language model: local llama.cpp server or any OpenAI-compatible API.
//! Turns prompts into validated shortcut plans and runs AI text steps.
use crate::i18n::{t, ts};
use crate::keys;
use crate::model::{ActionStep, Binding, ModelOptions};
use parking_lot::Mutex;
use serde_json::{json, Map, Value};
use std::io::{BufRead, BufReader};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const STEP_KINDS: &[&str] = &[
    "key", "text", "paste_text", "wait", "click", "click_element", "set_field", "read_field", "menu", "copy_selection", "clipboard_set", "clipboard_append", "write_file", "read_file", "copy_file", "move_file", "open_app", "open_resource",
    "open_website", "focus_app", "wait_window", "window", "powershell", "cmd", "system", "ai", "notify", "set_var", "copy", "paste", "select_all", "cut", "save", "save_as", "undo", "redo", "find", "new_tab", "reopen_tab", "close_tab", "address_bar",
    "command_palette", "toggle_mic", "print", "new_file", "open_file",
];

fn aliases(kind: &str) -> Option<(i32, i32)> {
    Some(match kind {
        "copy" => (2, 67),
        "paste" => (2, 86),
        "select_all" => (2, 65),
        "cut" => (2, 88),
        "save" => (2, 83),
        "save_as" => (6, 83),
        "undo" => (2, 90),
        "redo" => (2, 89),
        "find" => (2, 70),
        "new_tab" => (2, 84),
        "reopen_tab" => (6, 84),
        "close_tab" => (2, 87),
        "address_bar" => (2, 76),
        "command_palette" => (6, 80),
        "toggle_mic" => (6, 77),
        "print" => (2, 80),
        "new_file" => (2, 78),
        "open_file" => (2, 79),
        _ => return None,
    })
}

fn schema() -> Value {
    json!({
        "type": "object", "additionalProperties": false,
        "properties": {
            "status": {"type": "string", "enum": ["ready", "clarify", "unsupported"]},
            "name": {"type": "string", "maxLength": 100},
            "trigger": {"type": "string", "maxLength": 70},
            "app_id": {"type": "string", "maxLength": 20},
            "message": {"type": "string", "maxLength": 1500},
            "notes": {"type": "array", "maxItems": 8, "items": {"type": "string", "maxLength": 600}},
            "steps": {"type": "array", "maxItems": 200, "items": {
                "type": "object", "additionalProperties": false,
                "properties": {
                    "kind": {"type": "string", "enum": STEP_KINDS},
                    "app_id": {"type": "string", "maxLength": 20},
                    "combo": {"type": "string", "maxLength": 70},
                    "text": {"type": "string", "maxLength": 100000},
                    "ms": {"type": "integer", "minimum": 0, "maximum": 600000},
                    "path": {"type": "string", "maxLength": 2000},
                    "element": {"type": "string", "maxLength": 300},
                    "var": {"type": "string", "maxLength": 40},
                    "instruction": {"type": "string", "maxLength": 8000},
                    "output": {"type": "string", "enum": ["", "var", "type", "paste", "clipboard", "notify", "file"]},
                    "append": {"type": "boolean"}
                },
                "required": ["kind"]
            }}
        },
        "required": ["status", "name", "trigger", "app_id", "message", "notes", "steps"]
    })
}

const INSTRUCTIONS: &str = r#"You are the planner inside Makro+, a Windows automation app. Convert the user's request (English or Turkish) into ONE shortcut plan and return only the JSON object. You plan actions; never claim they ran.

ACTIVATION: trigger_override wins, else trigger_hint, else a key the user explicitly named. Never invent a trigger and never repeat it as a step. If none was supplied, return status "clarify", a short question in message, steps []. "Del+5" means hold Del then press 5. Mouse buttons (LButton, RButton, MButton, XButton1, XButton2) and wheel (WheelUp/Down/Left/Right) are valid triggers.

APPS: use the supplied app ids only. selected_app_id is the activation scope. Root app_id "global" means the steps go to whatever window is in front when the shortcut runs. Each step may set app_id to act on a different registered app — this is how ONE shortcut works across SEVERAL apps. Registered apps do not need to be running: focus_app launches the app if needed and waits for its window. Before keys/text/clicks in another app, add focus_app for it.

IN-APP COMMANDS: app_commands lists what a scan found inside each app: menu paths (use kind "menu", text = the exact path like "File › Save As"), UI elements (kind "click_element" with element = automation_id or text = name; "set_field"/"read_field" for edit fields), and keyboard accelerators (kind "key" with that combo). Prefer these precise commands over guessing keys.

STEP KINDS (unused fields may be omitted):
- key{combo} • text{text: literal typed text} • paste_text{text: pasted via clipboard, fast for long text} • wait{ms}
- click{text: visible button name} • click_element{element: automation id, text: name} • set_field{element, text: value} • read_field{element|text, var}
- menu{text: menu path, app_id} • window{text: minimize|maximize|restore|close|topmost|untopmost}
- copy_selection{var: default "selection"} copies the current selection and stores it in a variable
- clipboard_set{text} • clipboard_append{text}
- write_file{path, text, append} • read_file{path, var: default "file"} • copy_file{path: source, text: destination file or folder} • move_file{path, text}
- open_app{app_id, text: optional arguments} • open_resource{text: URL/URI/file, optional app_id} • open_website{text: URL, app_id: browser}
- focus_app{app_id, ms: optional timeout} • wait_window{text: part of the window title, app_id optional, ms timeout} — use wait_window after opening dialogs such as Save As.
- powershell{text: script, var: optional, stdout also in {last_output}} • cmd{text}
- system{text: lock|desktop|task-view|screenshot|clipboard|makro-clipboard|emoji|explorer|task-manager|audio|network|storage|updates|update-settings|close-apps|shutdown|restart} — prefer these over scripts.
- ai{instruction, text: input (default "{clipboard}"), output: var|type|paste|clipboard|notify|file, var: default "ai", path for output file} runs the language model WHEN THE SHORTCUT RUNS: translate, summarise, rewrite, extract, answer, generate.
- notify{text} shows a Windows notification • set_var{var, text}
- aliases with default keys (no other fields): copy paste select_all cut save save_as undo redo find new_tab reopen_tab close_tab address_bar command_palette toggle_mic print new_file open_file

VARIABLES in text/path/instruction: {selection} {clipboard} {ai} {file} {field} {last_output} {last_file} {date} {time} {datetime} {timestamp} {app} {title} {desktop} {documents} {downloads} {newline} {env:NAME}, plus any var you set. Environment %VARS% work in paths.

PATTERNS:
- "Save this file and send it to X": save (or save_as → wait_window "Save" → text the full path → key Enter → wait 800), then copy_file{path: that file, text: destination}, or focus_app X and open_resource{app_id: X, text: the path}.
- "Take the selection from A and put it into B": copy_selection → focus_app B → paste_text{text: "{selection}"}.
- "Translate/fix/summarise the selected text": copy_selection → ai{instruction, text: "{selection}", output: "paste"}.
- "Collect copied lines into a file": write_file{path, text: "{clipboard}", append: true}.

FIDELITY: type quoted text exactly. Keep explicit wait values exactly (100 ms is 100). "Copy the selection" never adds select_all. Opening one URL needs only one open_resource/open_website step. Do not invent files, paths or apps the user did not give; ask with clarify when a required path or target is missing. Explain assumptions in notes. Names, messages and notes follow the user's language.

OPERATING SYSTEM: operating_system tells where the shortcut runs. On "linux", cmd runs with `sh -c` (use bash/coreutils syntax), powershell needs pwsh, paths use "/", and click_element/set_field/read_field/menu are not available (use keys and text instead).

OUTPUT: status ready needs steps; clarify/unsupported need steps []. Use unsupported only when no tool, script or AI step can do it. At most 200 steps."#;

pub struct PlanRequest {
    pub prompt: String,
    pub trigger: String,
    pub scope_app: String,
    pub target_app: String,
    pub apps: Vec<String>,
    pub inventories: Vec<crate::model::AppInventory>,
    pub share_commands: bool,
    pub clipboard: Option<String>,
    pub rounds: u32,
}

#[derive(Default, Clone)]
pub struct PlanResult {
    pub binding: Option<Binding>,
    pub errors: Vec<String>,
    pub notes: Vec<String>,
    pub raw: String,
}

fn app_map(req: &PlanRequest) -> Vec<(String, String)> {
    let mut paths: Vec<String> = Vec::new();
    for p in [&req.scope_app, &req.target_app].into_iter().chain(req.apps.iter()) {
        if !p.trim().is_empty() && !paths.iter().any(|x| crate::win::same_path(x, p)) {
            paths.push(p.clone());
        }
    }
    paths.into_iter().enumerate().map(|(i, p)| (format!("app{i}"), p)).collect()
}

fn mentioned(prompt: &str, path: &str) -> bool {
    let fold = |s: &str| s.to_lowercase().replace('ı', "i");
    let p = fold(prompt);
    let names = [crate::win::file_stem(path), crate::apps::display_name(path)];
    names.iter().any(|n| {
        let n = fold(n);
        n.len() > 1 && p.contains(&n)
    })
}

/// Find "when I press X" / "X'e basınca" style trigger hints in the prompt.
pub fn trigger_hint(prompt: &str) -> String {
    let combo = r"((?:(?:ctrl|control|alt|shift|win)\s*\+\s*)*[a-z0-9]+(?:\s*\+\s*[a-z0-9]+)?)";
    let patterns = [
        format!(r"(?i)\b(?:when i press|pressing|press|assign(?: it)? to|bind(?: it)? to|trigger(?: key)?(?: is)?|shortcut)\s+{combo}"),
        format!(r"(?i){combo}\s*(?:'?[ey]e|'?[ae]|'?y[ae])?\s+(?:bas[ıi]nca|bast[ıi]ğ[ıi]mda|bas[ıi]ld[ıi]ğ[ıi]nda|ile|kısayol)"),
        format!(r"(?i)(?:kısayol(?:u)?|tuş(?:u)?)\s*[:=]?\s*{combo}"),
    ];
    for p in patterns {
        if let Ok(re) = regex::Regex::new(&p) {
            for c in re.captures_iter(prompt) {
                let cand = c[1].replace(' ', "");
                if let Some((m, k, pre)) = keys::parse_trigger(&cand) {
                    // Single letters/digits without modifiers are too ambiguous.
                    if m == 0 && pre == 0 && ((48..=57).contains(&k) || (65..=90).contains(&k)) {
                        continue;
                    }
                    return keys::trigger_text(m, k, pre);
                }
            }
        }
    }
    String::new()
}

fn build_messages(req: &PlanRequest, previous: Option<(&str, &str)>, include_schema: bool) -> Vec<Value> {
    let apps = app_map(req);
    let id_of = |p: &str| apps.iter().find(|(_, x)| crate::win::same_path(x, p)).map(|(i, _)| i.clone()).unwrap_or_default();
    let running = crate::win::running_apps();
    let app_list: Vec<Value> = apps
        .iter()
        .map(|(id, p)| {
            let d = crate::apps::describe(p);
            json!({"id": id, "name": d.name, "exe": crate::win::file_name(p), "path": p, "package_id": d.package_id, "protocols": d.protocols.iter().take(8).collect::<Vec<_>>(), "running": running.iter().any(|r| crate::win::same_path(r, p))})
        })
        .collect();
    let mut commands = Map::new();
    if req.share_commands {
        for (id, p) in &apps {
            let relevant = crate::win::same_path(p, &req.scope_app) || crate::win::same_path(p, &req.target_app) || mentioned(&req.prompt, p);
            if !relevant {
                continue;
            }
            if let Some(inv) = req.inventories.iter().find(|i| crate::win::same_path(&i.app_path, p)) {
                let list: Vec<Value> = inv
                    .commands
                    .iter()
                    .filter(|c| c.role != "disabled")
                    .take(160)
                    .map(|c| {
                        let mut o = Map::new();
                        o.insert("kind".into(), json!(c.source));
                        o.insert("label".into(), json!(c.label));
                        if !c.accelerator.is_empty() {
                            o.insert("keys".into(), json!(c.accelerator));
                        }
                        if !c.automation_id.is_empty() {
                            o.insert("automation_id".into(), json!(c.automation_id));
                        }
                        if !c.role.is_empty() {
                            o.insert("role".into(), json!(c.role));
                        }
                        Value::Object(o)
                    })
                    .collect();
                commands.insert(id.clone(), Value::Array(list));
            }
        }
    }
    let hint = trigger_hint(&req.prompt);
    let mut data = json!({
        "prompt": req.prompt,
        "selected_app_id": id_of(&req.scope_app),
        "target_app_id": id_of(&req.target_app),
        "trigger_override": req.trigger,
        "trigger_hint": hint,
        "applications": app_list,
        "app_commands": commands,
        "operating_system": crate::platform::OS,
    });
    if let Some(c) = &req.clipboard {
        data["current_clipboard"] = json!(crate::model::short(c, 2000));
    }
    let mut system = INSTRUCTIONS.to_string();
    if include_schema {
        system += "\n\nJSON schema of the answer:\n";
        system += &schema().to_string();
    }
    let example_user = json!({"prompt": "When I press Ctrl+Alt+T, translate the selected text to English and paste it.", "selected_app_id": "", "target_app_id": "", "trigger_override": "", "trigger_hint": "Ctrl+Alt+T", "applications": [], "app_commands": {}});
    let example_answer = json!({"status": "ready", "name": "Translate selection", "trigger": "Ctrl+Alt+T", "app_id": "global", "message": "", "notes": [], "steps": [
        {"kind": "copy_selection", "var": "selection"},
        {"kind": "ai", "instruction": "Translate to English. Return only the translation.", "text": "{selection}", "output": "paste"}
    ]});
    let mut msgs = vec![
        json!({"role": "system", "content": system}),
        json!({"role": "user", "content": example_user.to_string()}),
        json!({"role": "assistant", "content": example_answer.to_string()}),
        json!({"role": "user", "content": data.to_string()}),
    ];
    if let Some((prev, err)) = previous {
        msgs.push(json!({"role": "assistant", "content": prev}));
        msgs.push(json!({"role": "user", "content": format!("The plan was rejected: {err}\nRe-read the request, keep exact texts and waits, and return only the corrected JSON (clarify/unsupported have steps []).")}));
    }
    msgs
}

// ---------------- response parsing ----------------

fn content_text(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Array(blocks) => {
            let mut out = String::new();
            for b in blocks {
                if matches!(b.get("type").and_then(|x| x.as_str()), Some("text") | Some("output_text")) {
                    if let Some(s) = b.get("text").and_then(|x| x.as_str()) {
                        out += s;
                    } else if let Some(s) = b.get("text").and_then(|x| x.get("value")).and_then(|x| x.as_str()) {
                        out += s;
                    }
                }
            }
            Some(out)
        }
        _ => None,
    }
}

pub fn completion_text(body: &str) -> Result<String, String> {
    let root: Value = serde_json::from_str(body).map_err(|e| format!("{}: {e}", t("Invalid server response", "Geçersiz sunucu yanıtı")))?;
    let choice = root.get("choices").and_then(|c| c.get(0)).ok_or_else(|| t("The server returned no completion.", "Sunucu yanıt üretmedi.").to_string())?;
    let finish = choice.get("finish_reason").and_then(|f| f.as_str()).unwrap_or("");
    if finish == "length" || finish == "max_tokens" {
        return Err(t("The model hit its output token limit. Increase it in AI model settings.", "Model çıktı jetonu sınırına ulaştı. Yapay zekâ ayarlarından artırın.").into());
    }
    let msg = choice.get("message").ok_or_else(|| t("No assistant message in the response.", "Yanıtta asistan mesajı yok.").to_string())?;
    if let Some(c) = msg.get("content").and_then(content_text) {
        if !c.trim().is_empty() {
            return Ok(c);
        }
    }
    if let Some(args) = msg.get("tool_calls").and_then(|c| c.get(0)).and_then(|c| c.get("function")).and_then(|f| f.get("arguments")) {
        return Ok(if let Some(s) = args.as_str() { s.to_string() } else { args.to_string() });
    }
    Err(t("The model returned no final text. Choose an instruction model or raise the token budget.", "Model son metni döndürmedi. Talimat modeli seçin veya jeton bütçesini artırın.").into())
}

fn strip_reasoning(s: &str) -> String {
    let mut c = s.trim().trim_start_matches('\u{feff}').to_string();
    if let Some(end) = c.find("</think>") {
        c = c[end + 8..].trim().to_string();
    }
    if c.starts_with("```") {
        let re = regex::Regex::new(r"(?s)^```[a-zA-Z]*\s*(.*?)\s*```$").unwrap();
        if let Some(m) = re.captures(&c) {
            c = m[1].to_string();
        }
    }
    c
}

fn parse_object(content: &str) -> Result<Map<String, Value>, String> {
    let c = strip_reasoning(content);
    if let Ok(Value::Object(m)) = serde_json::from_str::<Value>(&c) {
        return Ok(m);
    }
    // Find balanced top-level objects in surrounding prose.
    let bytes: Vec<char> = c.chars().collect();
    let mut found = Vec::new();
    let (mut depth, mut start, mut quoted, mut esc) = (0, 0, false, false);
    for (i, &ch) in bytes.iter().enumerate() {
        if depth == 0 {
            if ch == '{' {
                start = i;
                depth = 1;
                quoted = false;
                esc = false;
            }
            continue;
        }
        if quoted {
            if esc {
                esc = false;
            } else if ch == '\\' {
                esc = true;
            } else if ch == '"' {
                quoted = false;
            }
            continue;
        }
        match ch {
            '"' => quoted = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let s: String = bytes[start..=i].iter().collect();
                    if let Ok(Value::Object(m)) = serde_json::from_str::<Value>(&s) {
                        found.push(m);
                    }
                }
            }
            _ => {}
        }
    }
    if found.len() == 1 {
        Ok(found.remove(0))
    } else {
        Err(t("Return exactly one complete JSON plan.", "Tam olarak bir eksiksiz JSON planı döndürün.").into())
    }
}

fn s_of(m: &Map<String, Value>, k: &str) -> String {
    match m.get(k) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}

fn ms_of(m: &Map<String, Value>) -> Result<i32, String> {
    match m.get("ms") {
        None | Some(Value::Null) => Ok(0),
        Some(Value::Number(n)) => {
            let v = n.as_f64().unwrap_or(-1.0);
            if (0.0..=600_000.0).contains(&v) && v.fract() == 0.0 {
                Ok(v as i32)
            } else {
                Err(t("ms must be a whole number between 0 and 600000.", "ms 0 ile 600000 arasında tam sayı olmalı.").into())
            }
        }
        Some(Value::String(s)) => s.trim().parse::<i32>().map_err(|_| "ms".to_string()),
        _ => Err("ms".into()),
    }
}

pub fn read_plan(content: &str, req: &PlanRequest) -> Result<PlanResult, String> {
    let root = parse_object(content)?;
    let apps = app_map(req);
    let app_of = |id: &str| -> Result<String, String> {
        if id.is_empty() || id == "global" {
            return Ok(String::new());
        }
        apps.iter().find(|(i, _)| i == id).map(|(_, p)| p.clone()).ok_or_else(|| format!("{} {id}", t("Unknown app_id:", "Bilinmeyen app_id:")))
    };
    let mut result = PlanResult { raw: content.to_string(), ..Default::default() };
    let status = s_of(&root, "status");
    let message = s_of(&root, "message");
    if let Some(Value::Array(notes)) = root.get("notes") {
        for n in notes.iter().take(8) {
            if let Some(s) = n.as_str() {
                result.notes.push(s.to_string());
            }
        }
    }
    let steps = match root.get("steps") {
        Some(Value::Array(a)) => a.clone(),
        _ => Vec::new(),
    };
    let status = if status.is_empty() { if steps.is_empty() { "clarify".to_string() } else { "ready".to_string() } } else { status };
    if status != "ready" || steps.is_empty() {
        result.errors.push(if message.trim().is_empty() { t("Clarify the request: which key, and what should happen?", "İsteği netleştirin: hangi tuş ve ne olsun?").to_string() } else { message });
        return Ok(result);
    }
    if steps.len() > 200 {
        return Err(t("At most 200 steps are allowed.", "En fazla 200 adıma izin verilir.").into());
    }
    let root_app = app_of(&s_of(&root, "app_id"))?;
    // Trigger: explicit field > hint in prompt > model's proposal.
    let hint = trigger_hint(&req.prompt);
    let model_trigger = s_of(&root, "trigger");
    let chosen = if !req.trigger.trim().is_empty() { req.trigger.clone() } else if !hint.is_empty() { hint } else { model_trigger };
    let mut binding = Binding { name: s_of(&root, "name"), kind: "Workflow".into(), prompt: Some(req.prompt.clone()), ..Default::default() };
    if binding.name.trim().is_empty() {
        binding.name = t("AI shortcut", "AI kısayolu").into();
    }
    if chosen.trim().is_empty() {
        result.errors.push(t("Which trigger key should run this? Choose it in the Trigger field.", "Bunu hangi tuş çalıştırsın? Tetikleyici alanından seçin.").into());
    } else {
        match keys::parse_trigger(&chosen) {
            Some((m, k, p)) => {
                binding.mods = m;
                binding.key = k;
                binding.prefix_key = p;
            }
            None => return Err(format!("{} “{chosen}”. {}", t("Trigger not understood:", "Tetikleyici anlaşılamadı:"), t("Use something like Ctrl+Alt+5 or Del+5.", "Ctrl+Alt+5 veya Del+5 gibi yazın."))),
        }
    }
    let mut out = Vec::new();
    let mut opening = false;
    for (i, raw) in steps.iter().enumerate() {
        let m = raw.as_object().ok_or_else(|| format!("step {}: not an object", i + 1))?;
        let kind = s_of(m, "kind");
        let text = s_of(m, "text");
        let combo = s_of(m, "combo");
        let path = s_of(m, "path");
        let element = s_of(m, "element");
        let var = s_of(m, "var");
        let ms = ms_of(m)?;
        let step_app = match m.get("app_id") {
            Some(Value::String(id)) if !id.is_empty() => app_of(id)?,
            _ => root_app.clone(),
        };
        let err = |msg: &str| format!("{} {} ({kind}): {msg}", t("Step", "Adım"), i + 1);
        let need_app = |a: &str| if a.is_empty() { Err(err(t("needs a registered app_id", "kayıtlı bir app_id gerekli"))) } else { Ok(a.to_string()) };
        let mut s = ActionStep::default();
        match kind.as_str() {
            "key" => {
                let (mo, k) = keys::parse_combo(&combo).ok_or_else(|| err(&format!("{} “{combo}”", t("invalid combo", "geçersiz tuş"))))?;
                if let Some(e) = keys::sendable(mo, k) {
                    return Err(err(e));
                }
                s.kind = "Key".into();
                s.mods = mo;
                s.key = k;
            }
            "text" | "paste_text" => {
                if text.is_empty() {
                    return Err(err(t("text is empty", "metin boş")));
                }
                s.kind = if kind == "text" { "Text".into() } else { "Paste text".into() };
                s.text = text;
            }
            "wait" => {
                s.kind = "Wait".into();
                s.delay = ms;
            }
            "click" => {
                s.kind = "Click button".into();
                s.text = text;
                if s.text.is_empty() {
                    return Err(err(t("button name is empty", "düğme adı boş")));
                }
            }
            "click_element" => {
                s.kind = "Click element".into();
                s.text = text;
                s.automation_id = Some(element).filter(|e| !e.is_empty());
                if s.text.is_empty() && s.automation_id.is_none() {
                    return Err(err(t("element or text is required", "element veya text gerekli")));
                }
            }
            "set_field" => {
                s.kind = "Set field".into();
                s.automation_id = Some(element).filter(|e| !e.is_empty());
                s.path = Some(path).filter(|p| !p.is_empty());
                s.text = text;
                if s.automation_id.is_none() && s.path.is_none() {
                    return Err(err(t("element is required", "element gerekli")));
                }
            }
            "read_field" => {
                s.kind = "Read field".into();
                s.automation_id = Some(element).filter(|e| !e.is_empty());
                s.text = text;
                s.var = Some(var).filter(|v| !v.is_empty());
            }
            "menu" => {
                s.kind = "Menu command".into();
                s.text = text;
                s.app_path = Some(step_app.clone()).filter(|a| !a.is_empty());
                if s.text.is_empty() {
                    return Err(err(t("menu path is empty", "menü yolu boş")));
                }
            }
            "copy_selection" => {
                s.kind = "Copy selection".into();
                s.var = Some(var).filter(|v| !v.is_empty());
            }
            "clipboard_set" | "clipboard_append" => {
                s.kind = if kind == "clipboard_set" { "Clipboard set".into() } else { "Clipboard append".into() };
                s.text = text;
            }
            "write_file" => {
                s.kind = "Write file".into();
                s.path = Some(path.clone());
                s.text = text;
                s.append = m.get("append").and_then(|v| v.as_bool()).unwrap_or(false);
                if path.is_empty() {
                    return Err(err(t("path is required", "path gerekli")));
                }
            }
            "read_file" => {
                s.kind = "Read file".into();
                s.path = Some(if path.is_empty() { text.clone() } else { path.clone() });
                s.var = Some(var).filter(|v| !v.is_empty());
            }
            "copy_file" | "move_file" => {
                s.kind = if kind == "copy_file" { "Copy file".into() } else { "Move file".into() };
                s.path = Some(path.clone());
                s.text = text;
                if path.is_empty() || s.text.is_empty() {
                    return Err(err(t("path (source) and text (destination) are required", "path (kaynak) ve text (hedef) gerekli")));
                }
            }
            "open_app" => {
                s.kind = "Open application".into();
                s.app_path = Some(need_app(&step_app)?);
                s.text = if crate::win::same_path(&text, &step_app) { String::new() } else { text };
                opening = true;
            }
            "open_resource" | "open_website" => {
                s.kind = "Open content".into();
                s.app_path = Some(step_app.clone()).filter(|a| !a.is_empty());
                s.text = text;
                if s.text.is_empty() {
                    return Err(err(t("resource is empty", "kaynak boş")));
                }
                opening = true;
            }
            "focus_app" => {
                s.kind = "Focus".into();
                s.app_path = Some(need_app(&step_app)?);
                s.delay = ms;
                opening = true;
            }
            "wait_window" => {
                s.kind = "Wait for window".into();
                s.text = text;
                s.delay = ms;
                s.app_path = m.get("app_id").and_then(|v| v.as_str()).filter(|v| !v.is_empty()).map(|id| app_of(id)).transpose()?;
            }
            "window" => {
                s.kind = "Window action".into();
                s.text = text.to_lowercase();
                s.app_path = m.get("app_id").and_then(|v| v.as_str()).filter(|v| !v.is_empty()).map(|id| app_of(id)).transpose()?;
                if !["minimize", "maximize", "restore", "close", "topmost", "untopmost"].contains(&s.text.as_str()) {
                    return Err(err(t("unknown window action", "bilinmeyen pencere eylemi")));
                }
            }
            "powershell" | "cmd" => {
                s.kind = if kind == "powershell" { "PowerShell".into() } else { "Terminal (cmd)".into() };
                s.text = text;
                s.var = Some(var).filter(|v| !v.is_empty());
                if s.text.trim().is_empty() {
                    return Err(err(t("script is empty", "betik boş")));
                }
                result.notes.push(t("Commands run only when the shortcut is triggered; review them in the preview.", "Komutlar yalnızca kısayol tetiklenince çalışır; önizlemede kontrol edin.").into());
                opening = true;
            }
            "system" => {
                if crate::system_actions::find(&text).is_none() {
                    return Err(err(t("unknown system action id", "bilinmeyen sistem eylemi")));
                }
                s.kind = "System".into();
                s.text = text;
                opening = true;
            }
            "ai" => {
                s.kind = "AI".into();
                s.instruction = Some(s_of(m, "instruction"));
                if s.instruction.as_deref().unwrap_or("").trim().is_empty() {
                    return Err(err(t("instruction is required", "instruction gerekli")));
                }
                s.text = text;
                s.output = Some(s_of(m, "output")).filter(|o| !o.is_empty());
                s.var = Some(var).filter(|v| !v.is_empty());
                s.path = Some(path).filter(|p| !p.is_empty());
            }
            "notify" => {
                s.kind = "Notify".into();
                s.text = text;
            }
            "set_var" => {
                s.kind = "Set variable".into();
                s.var = Some(var.clone());
                s.text = text;
                if var.is_empty() {
                    return Err(err(t("var is required", "var gerekli")));
                }
            }
            other => {
                let (mo, k) = aliases(other).ok_or_else(|| err(t("unsupported kind", "desteklenmeyen tür")))?;
                s.kind = "Key".into();
                s.mods = mo;
                s.key = k;
                if other == "save_as" {
                    result.notes.push(t("Save As uses Ctrl+Shift+S; some apps (Office) use F12.", "Farklı kaydet Ctrl+Shift+S kullanır; bazı uygulamalar (Office) F12 kullanır.").into());
                }
            }
        }
        if matches!(s.kind.as_str(), "Key" | "Text" | "Paste text" | "Click button" | "Click element" | "Set field") && !step_app.is_empty() {
            s.app_path = Some(step_app.clone());
        }
        if s.kind == "Key" && s.mods == binding.mods && s.key == binding.key && binding.key != 0 {
            return Err(t("The trigger key was repeated as a step. Name the key to send instead.", "Tetikleyici tuş adım olarak tekrarlandı. Gönderilecek tuşu belirtin.").into());
        }
        out.push(s);
    }
    binding.steps = Some(out);
    binding.app_path = if !req.scope_app.is_empty() { req.scope_app.clone() } else if !opening { root_app.clone() } else { String::new() };
    if let Some(e) = crate::rules::validate(&binding, &[]) {
        result.errors.push(e);
    }
    if !message.trim().is_empty() {
        result.notes.push(message);
    }
    if binding.app_path.is_empty() {
        result.notes.push(t("Global scope: steps go to the window in front when triggered.", "Genel kapsam: adımlar tetiklendiği anda öndeki pencereye gider.").into());
    }
    result.binding = Some(binding);
    Ok(result)
}

// ---------------- transport ----------------

struct Session {
    model: String,
    gpu: bool,
    fast: bool,
    port: u16,
    key: String,
    child: std::process::Child,
    _job: isize,
    log: Arc<Mutex<Vec<String>>>,
    ready: bool,
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        #[cfg(windows)]
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self._job as _);
        }
    }
}

static SESSION: Mutex<Option<Session>> = Mutex::new(None);
static GATE: Mutex<()> = Mutex::new(());

pub fn find_runtime() -> Option<std::path::PathBuf> {
    #[cfg(not(windows))]
    {
        if let Some(p) = std::env::var_os("PATH").and_then(|paths| std::env::split_paths(&paths).map(|d| d.join("llama-server")).find(|p| p.exists())) {
            return Some(p);
        }
    }
    let exe_name = if cfg!(windows) { "llama-server.exe" } else { "llama-server" };
    let mut dirs = vec![crate::storage::exe_dir()];
    if let Some(dev) = option_env!("CARGO_MANIFEST_DIR") {
        dirs.push(std::path::PathBuf::from(dev).join(".."));
    }
    for base in dirs {
        let mut d = Some(base);
        for _ in 0..5 {
            let Some(dir) = d else { break };
            let exe = dir.join("runtime").join("llama").join(exe_name);
            if exe.exists() {
                return Some(exe);
            }
            d = dir.parent().map(|p| p.to_path_buf());
        }
    }
    None
}

#[cfg(not(windows))]
fn job_kill_on_close(_child: &std::process::Child) -> isize {
    0
}

#[cfg(windows)]
fn job_kill_on_close(child: &std::process::Child) -> isize {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::JobObjects::*;
    unsafe {
        let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job.is_null() {
            return 0;
        }
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        SetInformationJobObject(job, JobObjectExtendedLimitInformation, &info as *const _ as *const _, std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32);
        AssignProcessToJobObject(job, child.as_raw_handle() as _);
        job as isize
    }
}

fn start_local(model: &str, gpu: bool, fast: bool) -> Result<Session, String> {
    let exe = find_runtime().ok_or_else(|| t("Local model engine (runtime\\llama\\llama-server.exe) was not found next to MakroPlus.exe.", "Yerel model motoru (runtime\\llama\\llama-server.exe) MakroPlus.exe yanında bulunamadı.").to_string())?;
    let port = std::net::TcpListener::bind("127.0.0.1:0").and_then(|l| l.local_addr()).map(|a| a.port()).map_err(|e| e.to_string())?;
    let mut keyb = [0u8; 24];
    crate::win::random_bytes(&mut keyb);
    let key: String = keyb.iter().map(|b| format!("{b:02x}")).collect();
    let mut cmd = crate::apps::hidden_command(&exe.display().to_string());
    cmd.current_dir(exe.parent().unwrap());
    for (k, _) in std::env::vars() {
        if k.to_uppercase().starts_with("LLAMA_") {
            cmd.env_remove(k);
        }
    }
    cmd.env("LLAMA_API_KEY", &key);
    cmd.args(["--model", model, "--host", "127.0.0.1", "--port", &port.to_string(), "--alias", "makro-local", "--ctx-size", "12288", "--parallel", "1", "--batch-size", "512", "--ubatch-size", "256", "--flash-attn", "auto", "--cache-prompt", "--gpu-layers", if gpu { "auto" } else { "0" }, "--reasoning", if fast { "off" } else { "auto" }, "--jinja", "--no-webui", "--no-agent"]);
    cmd.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::piped()).stdin(std::process::Stdio::null());
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let job = job_kill_on_close(&child);
    let log = Arc::new(Mutex::new(Vec::new()));
    if let Some(err) = child.stderr.take() {
        let l = log.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(err).lines().map_while(Result::ok) {
                let mut g = l.lock();
                g.push(line);
                if g.len() > 15 {
                    g.remove(0);
                }
            }
        });
    }
    Ok(Session { model: model.to_string(), gpu, fast, port, key, child, _job: job, log, ready: false })
}

#[cfg(feature = "net")]
pub fn agent(timeout: Duration) -> ureq::Agent {
    #[allow(unused_mut)]
    let mut b = ureq::AgentBuilder::new().timeout(timeout).redirects(0);
    #[cfg(windows)]
    if let Ok(tls) = native_tls::TlsConnector::new() {
        b = b.tls_connector(Arc::new(tls));
    }
    b.build()
}

#[cfg(feature = "net")]
fn local_chat(payload: &mut Value, progress: &dyn Fn(String)) -> Result<String, String> {
    let st = crate::app_state().lock();
    let model = crate::storage::resolve_model(&st.model_path);
    let gpu = st.use_gpu.unwrap_or(true);
    let fast = st.model_options.fast_mode;
    drop(st);
    if model.is_empty() || !std::path::Path::new(&model).exists() {
        return Err(t("No GGUF model selected. Put a .gguf into the models folder next to MakroPlus.exe or choose one in AI model settings.", "GGUF modeli seçilmedi. MakroPlus.exe yanındaki models klasörüne bir .gguf koyun veya Yapay zekâ ayarlarından seçin.").into());
    }
    let mut header = [0u8; 4];
    if std::fs::File::open(&model).and_then(|mut f| std::io::Read::read_exact(&mut f, &mut header)).is_err() || &header != b"GGUF" {
        return Err(t("The selected file is not a GGUF model.", "Seçilen dosya bir GGUF modeli değil.").into());
    }
    let _gate = GATE.lock();
    let mut tried_cpu = !gpu;
    loop {
        let mut guard = SESSION.lock();
        let stale = match guard.as_mut() {
            Some(s) => s.model != model || s.gpu != !tried_cpu || s.fast != fast || s.child.try_wait().ok().flatten().is_some(),
            None => true,
        };
        if stale {
            *guard = None;
            progress(format!("{} • {}", if tried_cpu { "CPU" } else { "GPU" }, t("Starting the model…", "Model başlatılıyor…")));
            *guard = Some(start_local(&model, !tried_cpu, fast)?);
        }
        let s = guard.as_mut().unwrap();
        let base = format!("http://127.0.0.1:{}", s.port);
        let key = s.key.clone();
        if !s.ready {
            let deadline = Instant::now() + Duration::from_secs(180);
            let a = agent(Duration::from_secs(2));
            let mut failed: Option<String> = None;
            loop {
                if s.child.try_wait().ok().flatten().is_some() {
                    failed = Some(s.log.lock().join("\n"));
                    break;
                }
                if a.get(&format!("{base}/health")).call().map(|r| r.status() == 200).unwrap_or(false) {
                    s.ready = true;
                    break;
                }
                if Instant::now() > deadline {
                    return Err(t("The model was not ready within 3 minutes. Check memory and the model file.", "Model 3 dakika içinde hazır olmadı. Belleği ve model dosyasını kontrol edin.").into());
                }
                std::thread::sleep(Duration::from_millis(150));
            }
            if let Some(tail) = failed {
                *guard = None;
                if !tried_cpu {
                    tried_cpu = true;
                    progress(t("GPU start failed; trying CPU…", "GPU başlatılamadı; CPU deneniyor…").into());
                    continue;
                }
                return Err(format!("{}\n{}", t("The model engine stopped.", "Model motoru durdu."), crate::model::short(&tail, 900)));
            }
        }
        drop(guard);
        payload["model"] = json!("makro-local");
        payload["cache_prompt"] = json!(true);
        payload["seed"] = json!(42);
        progress(format!("{} • {}", if tried_cpu { "CPU" } else { "GPU" }, t("Thinking…", "Düşünüyor…")));
        let resp = agent(Duration::from_secs(if tried_cpu { 900 } else { 300 })).post(&format!("{base}/v1/chat/completions")).set("Authorization", &format!("Bearer {key}")).send_json(payload.clone());
        return match resp {
            Ok(r) => completion_text(&r.into_string().map_err(|e| e.to_string())?),
            Err(ureq::Error::Status(code, r)) => Err(format!("HTTP {code}: {}", crate::model::short(&r.into_string().unwrap_or_default(), 300))),
            Err(e) => Err(e.to_string()),
        };
    }
}

fn base_url(o: &ModelOptions) -> Result<String, String> {
    let mut a = o.endpoint.trim().trim_end_matches('/').to_string();
    if a.to_lowercase().ends_with("/chat/completions") {
        a.truncate(a.len() - 17);
    }
    if !(a.starts_with("http://") || a.starts_with("https://")) {
        return Err(t("Enter an HTTP(S) API base URL, for example http://localhost:1234/v1", "HTTP(S) API adresi girin, örneğin http://localhost:1234/v1").into());
    }
    Ok(a)
}

#[cfg(feature = "net")]
fn remote_chat(o: &ModelOptions, payload: &mut Value, progress: &dyn Fn(String)) -> Result<String, String> {
    let base = base_url(o)?;
    if o.model.trim().is_empty() {
        return Err(t("Enter the model ID exposed by your server.", "Sunucunuzun model kimliğini girin.").into());
    }
    payload["model"] = json!(o.model);
    let key = o.api_key();
    progress(t("Server • Thinking…", "Sunucu • Düşünüyor…").into());
    for _ in 0..5 {
        let mut req = agent(Duration::from_secs(600)).post(&format!("{base}/chat/completions"));
        if !key.is_empty() {
            req = req.set("Authorization", &format!("Bearer {key}"));
        }
        match req.send_json(payload.clone()) {
            Ok(r) => return completion_text(&r.into_string().map_err(|e| e.to_string())?),
            Err(ureq::Error::Status(code, r)) if code == 400 || code == 422 => {
                let body = r.into_string().unwrap_or_default().to_lowercase();
                let unsupported = ["unsupported", "not support", "not permitted", "unknown", "unrecognized", "extra inputs", "only the default", "not allowed", "invalid"].iter().any(|w| body.contains(w));
                let obj = payload.as_object_mut().unwrap();
                let mut changed = false;
                if unsupported && (body.contains("response_format") || body.contains("json_schema")) && obj.contains_key("response_format") {
                    if body.contains("json_schema") && obj["response_format"]["type"] == "json_schema" {
                        obj.insert("response_format".into(), json!({"type": "json_object"}));
                    } else {
                        obj.remove("response_format");
                    }
                    changed = true;
                }
                if body.contains("max_completion_tokens") && obj.contains_key("max_tokens") {
                    let v = obj.remove("max_tokens").unwrap();
                    obj.insert("max_completion_tokens".into(), v);
                    changed = true;
                }
                if unsupported && body.contains("temperature") && obj.contains_key("temperature") {
                    obj.remove("temperature");
                    changed = true;
                }
                if !changed {
                    return Err(format!("HTTP {code}: {}", crate::model::short(&body, 300)));
                }
                progress(t("Adapting request parameters to this server…", "İstek parametreleri bu sunucuya uyarlanıyor…").into());
            }
            Err(ureq::Error::Status(code, r)) => return Err(format!("HTTP {code}: {}", crate::model::short(&r.into_string().unwrap_or_default(), 300))),
            Err(e) => return Err(e.to_string()),
        }
    }
    Err(t("The server rejected the request parameters.", "Sunucu istek parametrelerini reddetti.").into())
}

fn chat(messages: Vec<Value>, structured: bool, progress: &dyn Fn(String)) -> Result<String, String> {
    let o = crate::app_state().lock().model_options.clone();
    let mut payload = json!({"messages": messages, "temperature": 0, "max_tokens": o.token_budget(), "stream": false});
    if structured && o.structured_output {
        payload["response_format"] = if o.remote { json!({"type": "json_schema", "json_schema": {"name": "shortcut_plan", "strict": false, "schema": schema()}}) } else { json!({"type": "json_object", "schema": schema()}) };
    }
    #[cfg(feature = "net")]
    {
        if o.remote {
            remote_chat(&o, &mut payload, progress)
        } else {
            local_chat(&mut payload, progress)
        }
    }
    #[cfg(not(feature = "net"))]
    {
        let _ = (&mut payload, progress);
        Err(t("This build has no network support.", "Bu derlemede ağ desteği yok.").into())
    }
}

/// Prompt → validated plan, with correction rounds.
pub fn plan(req: &PlanRequest, progress: &dyn Fn(String)) -> Result<PlanResult, String> {
    if req.prompt.trim().is_empty() {
        return Err(t("Describe what the shortcut should do.", "Kısayolun ne yapacağını yazın.").into());
    }
    if req.prompt.chars().count() > 32_000 {
        return Err(t("Keep the request under 32,000 characters.", "İsteği 32.000 karakterin altında tutun.").into());
    }
    let o = crate::app_state().lock().model_options.clone();
    let include_schema = o.remote || !o.structured_output;
    let mut previous: Option<(String, String)> = None;
    let rounds = req.rounds.min(4);
    for attempt in 0..=rounds {
        let msgs = build_messages(req, previous.as_ref().map(|(a, b)| (a.as_str(), b.as_str())), include_schema);
        let content = chat(msgs, true, progress)?;
        match read_plan(&content, req) {
            Ok(r) if r.binding.is_some() && r.errors.is_empty() => return Ok(r),
            Ok(r) if r.binding.is_none() => return Ok(r), // clarify / unsupported
            Ok(r) => {
                if attempt == rounds {
                    return Ok(r);
                }
                previous = Some((content, r.errors.join(" ")));
            }
            Err(e) => {
                if attempt == rounds {
                    return Err(e);
                }
                previous = Some((content, e));
            }
        }
        progress(ts(format!("Correcting the plan ({}/{rounds})…", attempt + 1), format!("Plan düzeltiliyor ({}/{rounds})…", attempt + 1)));
    }
    Err(t("Could not create a valid plan.", "Geçerli bir plan oluşturulamadı.").into())
}

/// Free-form text transformation for AI steps and the clipboard page.
pub fn complete_text(instruction: &str, input: &str) -> Result<String, String> {
    if instruction.trim().is_empty() {
        return Err(t("The AI instruction is empty.", "AI talimatı boş.").into());
    }
    let msgs = vec![
        json!({"role": "system", "content": "You are a text tool inside Makro+ on Windows. Apply the instruction to the input. Return only the resulting text — no preface, no explanation, no quotes, and no code fences unless the instruction asks for code. Keep the input's language unless told otherwise."}),
        json!({"role": "user", "content": format!("Instruction:\n{instruction}\n\nInput:\n{input}")}),
    ];
    let text = chat(msgs, false, &|_| {})?;
    Ok(strip_reasoning(&text))
}

/// Quick connectivity test for the settings page.
pub fn test_connection(progress: &dyn Fn(String)) -> Result<String, String> {
    let r = chat(vec![json!({"role": "user", "content": "Reply with the single word: ready"})], false, progress)?;
    Ok(crate::model::short(&strip_reasoning(&r), 80))
}

pub fn stop() {
    *SESSION.lock() = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(prompt: &str, apps: Vec<String>) -> PlanRequest {
        PlanRequest { prompt: prompt.into(), trigger: String::new(), scope_app: String::new(), target_app: String::new(), apps, inventories: vec![], share_commands: true, clipboard: None, rounds: 2 }
    }

    #[test]
    fn hints() {
        assert_eq!(trigger_hint("When I press Ctrl+Alt+T, translate"), "Ctrl+Alt+T");
        assert_eq!(trigger_hint("Ctrl+Shift+1 ile seçili metni kopyala"), "Ctrl+Shift+1");
        assert_eq!(trigger_hint("F9'a basınca dosyayı kaydet"), "F9");
        assert_eq!(trigger_hint("Del+5'e basınca YouTube'u aç"), "Del+5");
        assert_eq!(trigger_hint("open youtube"), "");
    }

    #[test]
    fn multi_app_plan() {
        let notepad = "C:\\Windows\\System32\\notepad.exe".to_string();
        let r = req("Ctrl+Shift+1 ile seçimi Not Defteri'ne yapıştır", vec![notepad.clone()]);
        let content = r#"Sure! {"status":"ready","name":"Seçimi Not Defteri'ne","trigger":"","app_id":"global","message":"","notes":[],
          "steps":[{"kind":"copy_selection"},{"kind":"focus_app","app_id":"app0"},{"kind":"key","combo":"Ctrl+End"},{"kind":"paste_text","text":"{newline}{selection}","app_id":"app0"},{"kind":"save"},
                   {"kind":"ai","instruction":"Özetle","text":"{selection}","output":"notify"}]}"#;
        let p = read_plan(content, &r).unwrap();
        assert!(p.errors.is_empty(), "{:?}", p.errors);
        let b = p.binding.unwrap();
        assert_eq!((b.mods, b.key), (6, 49));
        assert_eq!(b.app_path, "");
        let kinds: Vec<&str> = b.steps().iter().map(|s| s.kind.as_str()).collect();
        assert_eq!(kinds, ["Copy selection", "Focus", "Key", "Paste text", "Key", "AI"]);
        assert_eq!(b.steps()[1].app(), notepad);
        assert_eq!(b.steps()[3].app(), notepad);
        assert_eq!((b.steps()[4].mods, b.steps()[4].key), (2, 83));
    }

    #[test]
    fn clarify_and_rejects() {
        let r = req("translate selection", vec![]);
        let p = read_plan(r#"{"status":"clarify","name":"","trigger":"","app_id":"global","message":"Which key?","notes":[],"steps":[]}"#, &r).unwrap();
        assert!(p.binding.is_none());
        assert_eq!(p.errors, vec!["Which key?".to_string()]);
        let bad = read_plan(r#"{"status":"ready","name":"x","trigger":"F8","app_id":"app7","message":"","notes":[],"steps":[{"kind":"save"}]}"#, &r);
        assert!(bad.is_err());
        let repeat = read_plan(r#"{"status":"ready","name":"x","trigger":"Ctrl+S","app_id":"global","message":"","notes":[],"steps":[{"kind":"save"}]}"#, &r);
        assert!(repeat.is_err());
    }

    #[test]
    fn legacy_settings_import_round_trip() {
        let path = crate::storage::legacy_path();
        let Ok(text) = std::fs::read_to_string(&path) else { return };
        let st: crate::model::State = serde_json::from_str(text.trim_start_matches('\u{feff}')).unwrap();
        let original: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).unwrap();
        let again: serde_json::Value = serde_json::to_value(&st).unwrap();
        assert_eq!(st.bindings.len(), original["Bindings"].as_array().unwrap().len());
        for (a, b) in original["Bindings"].as_array().unwrap().iter().zip(again["Bindings"].as_array().unwrap()) {
            for (k, v) in a.as_object().unwrap() {
                assert_eq!(Some(v), b.get(k), "field {k} changed");
            }
        }
    }
}
