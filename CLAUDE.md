# CLAUDE.md — VoiceGG

Full spec: see `PLAN.md`. Build phase by phase (PLAN.md §10). Don't skip ahead.

## Hard rules
- **Public repo**: never write personal info (usernames, `/home/<user>` paths, hostnames, emails, IPs, tokens, serials) into code, commits, logs, tests, or docs. Use XDG dirs via the `dirs` crate.
- No telemetry. No network calls at runtime (optional update check is opt-in, daemon only).
- Never use root, never touch `/etc` or `/usr`. User-level PipeWire only.
- RT audio thread: no allocation, locks, logging, syscalls, or blocking. Enable FTZ/DAZ.
- `#![forbid(unsafe_code)]` everywhere except the FFI crate; every `unsafe` needs `// SAFETY:`.
- Never use SteelSeries/Sonar names, logos, or assets in the app.
- A phase is done only when: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo deny check` and frontend tests all pass, plus a self-review with the `code-review-rubric` skill.

## Skills
Load the matching skill before working (see PLAN.md §0.1): sharp-edges, secure-coding, systematic-debugging, frontend-design, ui-styling, design-tokens, accessibility, web-performance, webapp-testing, static-analysis, code-review-rubric, ask-questions-if-underspecified.

## Stack
Rust workspace (pipewire-rs, nnnoiseless, tokio, zbus, serde) + Tauri 2 + Svelte 5 + TypeScript. Target: Arch-based distros, PipeWire + WirePlumber 0.5+, Wayland/Hyprland first.
