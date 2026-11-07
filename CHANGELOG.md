# Changelog

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
