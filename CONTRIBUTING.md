# Contributing

Use Windows 10/11 x64 with .NET Framework 4.8. Read the README and offline guide.

```powershell
.\setup-runtime.ps1
.\test.ps1 -OutputDirectory dist\contributor-check
.\scripts\verify-interface.ps1 -OutputDirectory dist\contributor-check -SkipBuild
.\scripts\verify-interface.ps1 -OutputDirectory dist\contributor-check -SkipBuild -DesktopPreview
.\scripts\verify-application-shortcuts.ps1 -OutputDirectory dist\contributor-check -SkipBuild -DesktopPreview
.\scripts\verify-update-notifications.ps1 -OutputDirectory dist\contributor-check -SkipBuild -DesktopPreview
```

The self-check opens controlled native test windows and sends input to those
fixtures. Run it on a desktop you can leave idle until it finishes. It does not
replace user settings. Inspect `test-results.txt`; report any SKIP alongside
PASS/FAIL results. Model tests are optional and require your own compatible
GGUF and, for app-routing tests, the relevant registered applications.

System-action dispatch tests use a fake host; desktop preview tests send WM_CLOSE
only to their own controlled window. Do not test shutdown, restart, lock or closing
all apps on a contributor's desktop. The optional `--system-model-test <model-path>`
checks plans without executing them. `--update-scan-test` performs a real online
Windows Update search, with no download or installation; it is not a CI step.

Release-notification tests use a fake HTTP handler and notification sink. They
cover versions, response limits, cancellation, offline recovery, opt-out and
repeated notices without contacting GitHub or displaying operating-system alerts.

Keep changes focused and describe the user-visible behavior, validation and
limitations. Preserve the preview-before-save flow, input loop prevention,
cancellation, conflict checks and separation between activation scope and target.
Do not add model/provider allowlists or bundle model weights.

Never commit settings exports, API keys, personal logs, downloads, binaries or
screenshots containing private paths. Run `scripts/verify-publication.ps1` after
staging your intended files. See [SECURITY.md](SECURITY.md) for reporting issues.

Contributions are made under the project's GPL-3.0-only license. Keep independent
third-party notices and record any new dependency and its license.

## Makro+ 3 (Rust, ust/)

- Windows: `.\rust\build.ps1 -Release -Package` (GNU or MSVC Rust toolchain). Run `cargo test` in `rust`.
- End-to-end hook test (sends harmless F13/F14/F23 keys): `.\rust\tools\e2e-test.ps1 -Exe <MakroPlus.exe> -Dir <empty folder>`.
- Linux: `cargo build --release` in `rust` on x86_64; packages are produced by `.github/workflows/rust.yml`.
- Public screenshots must use `rust\tools\demo-data.ps1` (synthetic data only).