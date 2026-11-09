# Contributing to Partner

## Branch protection (maintainers — one-time setup)

On GitHub, protect `main` (Settings → Branches → Branch protection rules):

- Require a pull request before merging (≥ 1 approval recommended).
- Require status checks to pass: `fmt`, `clippy`, `test (ubuntu-latest)`,
  `test (windows-latest)`, `deny`, `gitleaks`, `codeql`.
- Require branches to be up to date before merging.
- Do not allow force pushes; do not allow deletions.
- Optionally: require signed commits.

Releases are cut from `main` only, by pushing a `v*.*.*` tag, which triggers
`.github/workflows/release.yml` (builds artifacts + `checksums.txt` +
publishes the GitHub Release).

## Local checks (must pass before opening a PR)

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check        # requires: cargo install cargo-deny
```

## Code rules

- Rust, edition 2021, MSRV 1.89 (see `rust-version` in the manifests).
- No new dependencies without justification (smaller attack surface is a
  security feature for this app).
- **Never log, print, or persist note content or credentials.**
- All file writes of sensitive data go through `partner_core::storage`
  (atomic write, `0600`, fsync).
- Crypto comes from the `age` crate only; do not hand-roll cryptography.
- UI strings go through `src/i18n.rs` (English first; French next).
