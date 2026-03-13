# TUI Roadmap

This roadmap splits visual/TUI work into two tracks so wrapper ergonomics can improve quickly while deeper rendering features can be proposed upstream.

## Track A: Wrapper-level improvements (`windows-mtr`)

Focus: Improvements that can ship in this repository without waiting on upstream Trippy feature changes.

- Add curated `--profile` presets (`pretty`, `minimal`, `ascii-safe`) that expand to known-good `--trippy-flags` bundles.
- Publish profile usage guidance in `USAGE.md` and `README.md` with copy/paste commands.
- Add a launch profile for Windows Terminal users so elevated sessions can open with profile defaults quickly.
- Document profile trade-offs (refresh rate, address mode, and columns) for `cmd.exe`, PowerShell, and Windows Terminal.
- Keep compatibility rules explicit: wrapper presets should remain additive and overrideable via `--trippy-flags`.

## Track B: Upstream Trippy feature requests/PRs

Focus: Features requiring upstream Trippy changes (new widgets/charts/columns).

- Request packet-loss spike mini-chart widgets to improve at-a-glance incident detection.
- Propose additional TUI columns for burst loss visibility and rolling percentile latency.
- Propose compact terminal-layout widgets optimized for classic Windows console dimensions.
- Upstream Unicode/ASCII rendering mode toggles to improve `cmd.exe` readability parity.
- Contribute PRs that expose these features as stable Trippy CLI flags that `windows-mtr` can consume.

## Success metrics

- **Readability in default `cmd.exe`:** improved operator score (1–5) for profile defaults versus baseline.
- **Time-to-diagnose packet loss spike:** lower median time to identify the affected hop during a controlled scenario.
- **User preference feedback:** profile preference split and qualitative feedback from operators after trialing all profiles.
