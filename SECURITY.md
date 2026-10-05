# Security Policy

## Supported Versions

Only the latest release on the `main` branch is supported with security updates.

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in VoiceGG, please **do not** open a public issue.

Instead, please submit a vulnerability report via GitHub Private Vulnerability Reporting under the **Security** tab of the repository.

### What to include
- Description of the vulnerability.
- Steps to reproduce or proof-of-concept.
- Affected components (daemon, IPC, DSP, GUI).
- Potential impact.

We will acknowledge receipt within 48 hours and work with you to release a patched version promptly.

## Security Architecture Principles

- **No Root Privileges**: VoiceGG runs strictly in the user session (`systemd --user`). It never modifies `/etc`, `/usr`, or system-wide audio files.
- **Local-Only IPC**: IPC communication uses a Unix domain socket located at `$XDG_RUNTIME_DIR/voicegg/daemon.sock` with file mode `0600` and `SO_PEERCRED` validation.
- **Privacy by Default**: No telemetry, no usage tracking, no remote logging, and no persistent device identifiers stored.
- **Strict Parsing**: Preset files and configuration files are schema-validated, strictly bounded in memory, and reject unexpected executable constructs.
