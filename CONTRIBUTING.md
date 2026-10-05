# Contributing to VoiceGG

Thank you for your interest in contributing to VoiceGG!

VoiceGG is a free, open-source audio routing and DSP management tool built for Linux and PipeWire.

## Code of Conduct & Development Rules

1. **Privacy First**: Never commit personal information, system paths (e.g. `/home/<user>`), hostnames, personal emails, or hardware identifiers. Always use XDG standard directories via the `dirs` crate.
2. **Audio Real-Time Safety**:
   - The real-time audio thread must **never allocate, acquire blocking locks, make syscalls, log, or block**.
   - Always enable FTZ (Flush-To-Zero) and DAZ (Denormals-Are-Zero) on audio threads.
3. **Rust Best Practices**:
   - `#![forbid(unsafe_code)]` is enforced on crates unless interfacing with C/FFI (such as direct PipeWire bindings).
   - Any `unsafe` block must be accompanied by an explicit `// SAFETY:` rationale.
   - Code must pass `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`.
4. **No Trademark Infringement**: Do not use third-party proprietary product names or proprietary assets/logos in code or UI.

## Development Workflow

### Prerequisites
- Rust 1.80+ (stable)
- PipeWire development headers (`libpipewire-0.3` / `pkg-config`)
- Node.js & npm (for UI development)

### Running Checks
```bash
# Check formatting
cargo fmt --check

# Run linter
cargo clippy --all-targets -- -D warnings

# Run all tests
cargo test --all-targets
```

## Creating Pull Requests

- Keep changes focused and atomic.
- Provide descriptive commit messages.
- Ensure automated CI workflows pass.
