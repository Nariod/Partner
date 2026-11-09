# Partner

**Encrypted single-file hierarchical notes, built for security auditors.**

Partner takes the best of CherryTree (one encrypted file per mission) and
Obsidian (fast, pleasant note-taking): every mission lives in a single
`.partner` file, encrypted with the [age](https://age-encryption.org/v1)
format, so the file can travel anywhere (USB key, cloud sync, email) and
stay unreadable without the passphrase or key.

- Linux (native binary or **Flatpak**) and Windows (MSI).
- Pure Rust, egui UI, no telemetry, no accounts, no cloud.
- Flatpak builds request **no network permission**; file access goes through
  the xdg-document portal.

## Features (v1)

- One `.partner` file per mission: hierarchical note tree (drag & drop,
  rename, search), Markdown editor with live preview split view.
- Screenshots pasted from the clipboard are embedded **inside the encrypted
  file** (`partner://blob/…`).
- Encryption: age v1 — passphrase (scrypt) **or** X25519 key(s) (age1…).
  Interoperable with the `age` / `rage` CLIs.
- Auto-lock after inactivity (plaintext wiped from memory, passphrase never
  stored), atomic saves, 5 rotating encrypted backups, `0600` permissions,
  single-instance file lock.
- Shortcuts: `Ctrl+P` quick switcher, `Ctrl+Shift+P` command palette,
  `Ctrl+S` save, `Ctrl+L` lock, `Ctrl+E` toggle preview.
- In-app updater for native builds (checks GitHub Releases, verifies
  SHA-256 against `checksums.txt` before install). Flatpak builds update via
  the system package manager and have no network access.

## Security

- File format and threat model: [`docs/FORMAT.md`](docs/FORMAT.md),
  [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md).
- Report vulnerabilities privately: see [`SECURITY.md`](SECURITY.md).
- CI on every PR: fmt, clippy `-D warnings`, tests (Linux + Windows),
  `cargo-deny` (advisories/licenses), gitleaks (secrets), CodeQL (Rust).
  `main` is protected; releases are built by CI on `v*.*.*` tags with
  published SHA-256 checksums. See [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Build

Requires Rust ≥ 1.89.

```sh
cargo build --release        # native binary: target/release/partner
cargo test --workspace       # tests (core crypto/storage/model/update)
```

### Flatpak (Linux)

```sh
flatpak-builder --force-clean build-dir packaging/io.github.Nariod.Partner.yml
flatpak build-bundle repo partner.flatpak io.github.Nariod.Partner
```

The manifest uses the GNOME 47 runtime/SDK (ships cargo/rustc) and requests
only wayland/x11/ipc/dri + the document portal — no network, no home access.

### Windows

```sh
cargo build --release        # target\release\partner.exe
```

The release workflow also builds an MSI (WiX) — see
`packaging/windows/partner.wxs`.

## Usage

1. **File → New encrypted file**: pick a location (`mission.partner`), set a
   strong passphrase (or generate an age keypair from **Help → About** and
   use the secret key at unlock time).
2. Create notes in the left tree, write Markdown, paste screenshots (🖼 or
   **Edit → Paste image**).
3. `Ctrl+S` saves (also auto-saves every 30 s); the app auto-locks after 10
   min idle (configurable in Settings).
4. The `.partner` file can be opened with `rage -d -i key.txt mission.partner`
   too — the format is standard age around a documented payload.

## Roadmap

- v2: French UI, age PQ recipients (mlkem768x25519), code signing of
  installers, in-app encrypted history/snapshots, PDF/HTML export (with a
  cleartext warning), graph view, backlinks, templates.
- The `.partner` container stays age; only the inner payload version may
  evolve (versioned, see `docs/FORMAT.md`).

## License

AGPL-3.0 — see [LICENSE](LICENSE).
