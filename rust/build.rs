// The self-contained MinGW toolchain lacks some import libraries. GNU ld can
// link against a DLL directly, so expose just those DLLs in a private folder.
fn main() {
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("dll-link");
    let _ = std::fs::create_dir_all(&out);
    let system = std::path::PathBuf::from(std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into())).join("System32");
    for dll in ["shlwapi.dll", "dwmapi.dll", "uxtheme.dll"] {
        let _ = std::fs::copy(system.join(dll), out.join(dll));
    }
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rerun-if-changed=build.rs");
}
