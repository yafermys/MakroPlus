//! In-app automation on Linux. UI Automation does not exist here; AT-SPI support
//! is planned. Steps that need it report a clear error instead of misfiring.
use crate::i18n::t;
use crate::model::AppInventory;
use crate::win::Hwnd;

fn unsupported() -> String {
    t("Reading and clicking controls inside apps is not available on Linux yet. Use keys, menus' keyboard shortcuts or text steps.", "Uygulama içindeki denetimleri okuma ve tıklama Linux'ta henüz yok. Tuş, menü kısayolu veya metin adımları kullanın.").into()
}

pub fn com_init() {}

pub fn activate_package(_id: &str, _args: &str) -> Result<(), String> {
    Err(unsupported())
}

pub fn menu_command(_w: Hwnd, _path: &str) -> Result<(), String> {
    Err(unsupported())
}

pub fn click_element(_w: Hwnd, _id: &str, _name: &str, _control: i32) -> Result<(), String> {
    Err(unsupported())
}

pub fn set_field(_w: Hwnd, _id: &str, _name: &str, _value: &str) -> Result<(), String> {
    Err(unsupported())
}

pub fn read_field(_w: Hwnd, _id: &str, _name: &str) -> Result<String, String> {
    Err(unsupported())
}

pub fn scan_app(_app: &str) -> Result<AppInventory, String> {
    Err(unsupported())
}
