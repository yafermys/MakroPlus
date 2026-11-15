# Changelog

## 2.8.0 - 2026-11-15

- Notify users about newer stable Makro+ releases with a tray notification and persistent in-app release banner.
- Add English/Turkish update settings, manual checks, opt-out, six-hour success caching, offline backoff and one notice per version across restarts. Fetch only public release metadata; never download or install updates automatically.
- Keep update HTTP and disk writes off the input path, cancel requests on exit, validate release links and bound response size/time. Add fake-network headless and desktop notification checks to CI and packaging.
- Add an Application shortcuts page with app-name search, action/key filters, source filters and optional global/disabled entries.
- Combine detected in-app assignments, native menu shortcuts and personal Makro+ launch/scoped actions without registering external keys again.
- Read Brave's saved native accelerators and extension commands, including custom changes and explicit removals.
- Discover supported editor, utility and game settings, including Antigravity, Notepad++, Sublime Text, JetBrains, VLC, OBS, PowerToys, Minecraft, League of Legends and Counter-Strike 2.
- Keep unsupported applications visible with honest coverage information; label Spotify/Discord defaults as references.
- Preserve source bytes with read-only new providers and block them from existing source-write/restore paths.
- Bound file sizes, traversal and record counts; report damaged sources without losing other results.
- Optimize app indexing and draft conflict checks; cap dense conflict reports with an explicit notice.
- Add parser, safety, native-menu and search/filter checks to the Windows workflow.
- Update the landing page, screenshots, offline guide and provider documentation; record the standing release-documentation rule in AGENTS.md.

## 2.7.0 — 2026-11-15

- Simplify Home and sidebar navigation, and group advanced shortcut options in the editor.
- Add a persistent English/Türkçe interface selector while keeping stored action IDs and user-entered commands unchanged.
- Add 16 ready-to-customize recommendations with keyboard, mouse and touchpad triggers and existing conflict validation.
- Add native Windows actions for lock, desktop/task view, screenshots, clipboard history, symbols, Explorer, Task Manager, sound/network/storage settings and Windows Update.
- Check for available software updates on a worker through Windows Update; no updates are downloaded or installed.
- Confirm shutdown, restart and closure of visible user-session app windows at execution time. Respect save dialogs and permissions; preserve the shell, background services and Makro+.
- Make native system actions available to prompt-generated plans and editable macro steps without requiring a registered or running app.
- Preserve direct execution of saved plans and the input/history responsiveness improvements from 2.6.3.
- Add headless system-action checks to Windows CI and a repeatable desktop preview test script.
- Refresh the English README, screenshots and offline guide for simpler setup and discovery.

## 2.6.3

- Integrate input-responsiveness PR #1 by misutesu-desu, preserving its authorship.
- Move coalesced history disk writes off the UI thread while keeping mutable-state capture on the owning thread.
- Reject stale and duplicate snapshots per destination so delayed history cannot roll back newer settings.
- Preserve dirty history after capture/write failures; ignore obsolete task failures after a newer successful save.
- Observe worker failures independently of the UI dispatcher and remove input hooks before a final synchronous close save.
- Filter unsupported mouse messages before marshaling, skip foreground process queries for unmatched keyboard/pointer input, and cache own-path/extended-key data.
- Add reproducible concurrency, failure, retry, shutdown and hook-routing checks; run the headless subset in Windows CI.

## 2.6.2

- Prepare the first public source release under GPL-3.0-only.
- Add source/distribution exclusions, third-party notices, contributor guidance,
  publication checks and a Windows build workflow.
- Include license and source-building information in the Windows package.
- Align the Windows manifest version with the app version.
- Handle invalid manually entered model paths without crashing or saving settings.
- Replace personal example URLs with synthetic test fixtures.

## 2.6.1

- Add the documented `models` folder, folder opening/refresh and portable model paths.
- Include a complete offline user guide with prompt examples and configuration help.
- Keep registered closed applications available for creating assignments.
- Verify 714 automatic checks and three real model-folder/closed-app scenarios.

## 2.6.0

- Improve compatibility across local models and chat-completions servers.
- Support response blocks/tool plan calls and automatic request-parameter adaptation.
- Validate explicit text, waits, selection and action order before presenting a ready plan.
- Reduce unnecessary trigger and foreground-focus delays.
- Exercise 42 local/API compatibility scenarios across three GGUF models and 20
  existing model regression scenarios on the development computer.

Earlier development builds introduced keyboard/mouse shortcuts, continuous
one-finger touchpad shapes, macros, profiles, app preferences, conflict scanning,
custom themes and editable detected shortcuts. Test results describe their exact
scope; they are not a claim of physical testing on every input device or model.
