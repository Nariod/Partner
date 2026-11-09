# The `.partner` file format

A Partner file is a **single file per mission**, encrypted with the
[age v1 format](https://age-encryption.org/v1) (implemented by the Rust
[`age`](https://docs.rs/age) crate). Partner never invents its own
cryptography: the outer layer is exactly the age format, so any
`age`/`rage`-compatible tool can decrypt a `.partner` file if you know the
credential.

## Outer layer: age

- Binary age file (default) or ASCII-armored (`-----BEGIN AGE ENCRYPTED
  FILE-----`), depending on the setting at creation time. Both are accepted
  on open.
- Recipients:
  - **passphrase** (`scrypt` stanza), or
  - one or more **X25519 public keys** (`age1…`). Multiple keys are allowed;
    a passphrase *cannot* be mixed with key recipients in the same file (age
    restriction), and Partner enforces this with a clear error.
- Integrity and authenticity of the whole file come from age itself (HMAC
  over the header + ChaCha20-Poly1305 STREAM over the payload). Any
  modification of the file makes decryption fail.

## Inner layer: Partner payload

The plaintext of the age file is the Partner payload:

```
+-------------------+-------------------+--------------------------------+
| magic: "PARTNER"  | version: u8 = 1   | zstd(level 6) compressed data  |
| 7 bytes           | 1 byte            |                                |
+-------------------+-------------------+--------------------------------+
```

The decompressed data is a [postcard](https://docs.rs/postcard) (serde) serialization of the `Document` structure:

```rust
Document {
    meta:  { title: String, root: u64, next_node_id: u64 },
    nodes: BTreeMap<u64, Node>,   // Node { id, parent, children, title, body, collapsed }
    blobs: BTreeMap<String, Blob> // Blob { mime, data } — embedded images etc.
}
```

- `body` is Markdown. Embedded images are referenced as
  `![alt](partner://blob/<32-hex-chars>)` and stored in `blobs`.
- The payload format is versioned (`version` byte). Unknown versions are
  rejected; the magic guards against opening non-Partner age files by
  accident.

## Sibling files created next to a `.partner` file

- `<file>.partner.lock` — lock file (flock / LockFileEx) preventing two
  Partner instances from editing the same file. Permissions `0600`.
- `<file>.partner.bak1` … `.bak5` — rotating encrypted backups (same age
  credentials as the main file), created on every save before the atomic
  replace. Permissions `0600`.
- `<file>.partner.tmp` — transient file during atomic save (write + fsync +
  rename). Never contains plaintext.

## What the format deliberately does NOT do

- No plaintext is ever written to disk (only age ciphertext).
- No git-style external versioning: use the built-in encrypted backups.
- File size leaks the approximate size of the (compressed) plaintext; age
  header metadata (recipient types) is visible. This is inherent to age and
  documented in the threat model.
