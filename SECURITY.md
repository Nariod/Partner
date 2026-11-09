# Security policy

## Reporting a vulnerability

Please report vulnerabilities **privately** via GitHub's private vulnerability
reporting:

- <https://github.com/Nariod/Partner/security/advisories/new>

or by email to the maintainer if listed in the repository profile.

Do **not** open a public issue, PR, or discussion for a security
vulnerability.

You can expect an acknowledgement within 72 hours. We will coordinate a fix
and a release, and credit you in the release notes unless you prefer to stay
anonymous.

## Scope

In scope: the Partner application, the `.partner` file format (see
`docs/FORMAT.md`), the release/update pipeline, and the CI configuration.

Out of scope: the age format itself (report to
<https://github.com/FiloSottile/age>), the Rust standard library, and
vulnerabilities in third-party dependencies that are not exploitable through
Partner's usage (report those upstream; our CI tracks advisories via
`cargo-deny`).

## Supported versions

Only the latest release (`v*.*.*` tag on `main`) receives security fixes.

## Update channel integrity

- Releases are published on GitHub Releases with a `checksums.txt`
  (SHA-256) covering every artifact.
- The in-app updater (non-Flatpak builds) verifies the SHA-256 of the
  downloaded installer against `checksums.txt` before running it.
- Flatpak builds request **no network permission**; updates come from the
  system/Flathub.
- Code signing of Windows installers is planned; until then, verify the
  SHA-256 yourself: `certutil -hashfile partner.msi SHA256` and compare with
  `checksums.txt`.
