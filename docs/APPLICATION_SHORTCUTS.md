# Application shortcut discovery

Open **Application shortcuts** in the sidebar (Türkçe: **Uygulama kısayolları**).
Choose **Scan applications**, search for an application on the left, then inspect
its keys and actions on the right. Registered applications also have a
**Show assigned shortcuts** button.

- Search actions or key combinations within the selected application.
- Filter by app sources, Makro+ assignments or known defaults.
- Include global shortcuts when you want to see assignments affecting every app.
- Enable **Show disabled / unassigned** to inspect removed or inactive entries.
- Select a row for its command, context and source path. **Show source** locates
  a settings file. Existing supported JSONC and Windows shortcut editors remain
  available; other providers show read-only details.

The scan runs in the background. Searching filters the existing results instead
of scanning the disk on each keystroke. Detected app shortcuts are also available
to the existing conflict checker and shortcut guide. Dense conflict sets have an
explicit limit notice instead of unbounded pair processing.

## Readable sources

| Application | What is read |
| --- | --- |
| VS Code, Insiders, Cursor, VSCodium, Antigravity | User/profile `keybindings.json` overrides, sequences, conditions and removal rules |
| Brave | Current `brave.accelerators` native browser assignments, explicit removals and extension commands in Preferences; stored defaults help identify custom changes |
| Chrome, Edge, Vivaldi | Saved Windows extension command assignments in Preferences |
| Notepad++ | Command, macro and plugin entries in `shortcuts.xml` |
| Sublime Text | User `.sublime-keymap` files, including sequences and contexts |
| JetBrains IDEs | Custom keymap XML; inherited defaults are not reconstructed |
| VLC | Explicit `key-*` and `global-key-*` overrides in `vlcrc` |
| OBS Studio | Hotkeys in profile INI files and scene/source JSON |
| PowerToys Keyboard Manager | Key and shortcut remapping profiles, including app-specific targets |
| Minecraft Java | Saved keyboard/mouse bindings in `options.txt` |
| League of Legends | Event assignments in `Config/input.ini` |
| Counter-Strike 2 | Saved `bindings` in user-key VCFG files, including literal scancodes |
| Open Windows applications | Shortcut labels exposed through native Win32 menus; custom-drawn menus may expose none |

Automatic discovery uses known user-settings locations, registered application
paths, installed-app metadata and standard Steam user-config locations. Use
**Add source file** or **Add folder** for supported portable/custom settings
locations. A file must still match a recognized format; this is not a general
parser for every JSON, XML or game configuration file.

## Coverage and safety

**A missing record does not mean the application has no shortcuts.** Coverage
is shown per app. Apps with private/cloud-only settings can remain in the search
list even when their custom bindings cannot be read. Documented Spotify and
Discord defaults are labeled **Known default / Reference only**, separately from
actual detected settings and personal Makro+ assignments.

The scan does not launch applications, press their keys or register their
shortcuts again. New provider readers do not write application settings or keep
full settings snapshots. Only shortcut-related fields are retained. Edits to
read-only sources must be made inside the original app, followed by a rescan.
Existing editable sources still use their original backup and stale-write checks.

Some files contain overrides rather than all built-in defaults. App profiles,
focus conditions, key layouts, permissions and enabled plugin states can affect
what actually runs. Unknown command IDs and physical scancodes remain visible;
they are not guessed into a different action. File sizes, traversal depth and
record counts are bounded, and damaged sources are reported without hiding
other successful sources.

## Verification

```powershell
.\scripts\verify-application-shortcuts.ps1
.\scripts\verify-application-shortcuts.ps1 -DesktopPreview
```

The checks use synthetic files and a controlled hidden native menu. They verify
parsing, original-byte preservation, restricted XML handling, large-file errors,
partial scans, app identities, conflict integration and search/filter UI behavior.
No application's real settings are edited. The optional `--app-shortcut-live`
flag runs discovery against the current computer and writes an ignored local
summary containing counts and app names, without exporting settings snapshots.

## Format references

- [Brave accelerator preferences](https://github.com/brave/brave-core/blob/master/components/commands/browser/accelerator_pref_manager.cc)
- [Brave key-code representation](https://github.com/brave/brave-core/blob/master/components/commands/common/accelerator_parsing.cc)
- [Chromium extension commands](https://github.com/chromium/chromium/blob/main/chrome/browser/extensions/commands/command_service.cc)
- [Notepad++ configuration files](https://npp-user-manual.org/docs/config-files/)
- [Sublime Text key bindings](https://www.sublimetext.com/docs/key_bindings.html)
- [PowerToys Keyboard Manager profile format](https://github.com/microsoft/PowerToys/blob/main/doc/devdocs/modules/keyboardmanager/keyboardmanager.md)
- [Win32 menu labels](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-getmenustringw)
- [Spotify default shortcuts](https://support.spotify.com/jm/article/keyboard-shortcuts/)
- [Discord default shortcuts](https://support.discord.com/hc/en-us/articles/225977308--Windows-Discord-Hotkeys)
