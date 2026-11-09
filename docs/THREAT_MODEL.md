# Threat model

## Assets

1. **Mission notes** inside a `.partner` file (findings, screenshots,
   command outputs, client data) — confidentiality and integrity are
   critical.
2. **The credential** (passphrase or age secret key) — compromise of the
   file *plus* credential yields the plaintext.
3. **The app binary and its updates** — a tampered update would be a total
   compromise of every user's notes.

## Threats and mitigations

| Threat | Mitigation |
|---|---|
| Stolen laptop / copied `.partner` file / cloud-sync folder leak | age encryption at rest (scrypt or X25519 recipients); AEAD detects any tampering |
| File left behind in Downloads/tmp, backups | backups (`.bakN`) are encrypted with the same credentials; permissions `0600`; no plaintext temp files (atomic write: tmp + fsync + rename) |
| Corruption / crash during save | atomic rename; rotating encrypted backups (default 5) |
| Two instances editing the same file | exclusive lock file (flock/LockFileEx) |
| Weak passphrase | scrypt (memory-hard, adaptive work factor chosen by the age implementation); Partner never stores the passphrase; auto-lock clears plaintext from memory after inactivity |
| Memory disclosure while the file is open | plaintext lives in RAM only while unlocked (required to edit); auto-lock drops it; `SecretString` zeroizes credentials on drop/lock; mlock is a possible v2 hardening |
| Clipboard screenshots | pasted images go to RAM then into the encrypted document as blobs; never written in clear |
| Tampered update (Windows/native builds) | update fetched over HTTPS from GitHub Releases API, **SHA-256 verified against the published `checksums.txt`** before install; Flatpak builds have **no network permission** and update via the system package manager; code-signing of installers is planned (v2) |
| Malicious dependency / vulnerable crate | CI runs `cargo-deny` (advisories + license policy) and gitleaks on every PR; `Cargo.lock` committed |
| Supply-chain on CI | all workflows pin `permissions: contents: read` (release: `contents: write`), use GitHub-hosted runners, no third-party actions except well-known ones (dtolnay/rust-toolchain, gitleaks, codeql, softprops/action-gh-release) |

## Out of scope (v1)

- Cold-boot / DMA attacks on RAM.
- Keyloggers / compromised OS.
- Post-quantum confidentiality (age `mlkem768x25519` recipients exist and can
  be enabled in v2).
- Metadata hiding beyond what age provides (filename, mtime and approximate
  size of the ciphertext are visible).
- Denial of service (e.g. filling the disk).

## Reporting

See [SECURITY.md](../SECURITY.md). Do not open public issues for
vulnerabilities.
