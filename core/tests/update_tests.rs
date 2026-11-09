use partner_core::update::{
    find_asset, is_newer, parse_checksums, parse_release, sha256_hex, verify_sha256, version_cmp,
};
use std::cmp::Ordering;

const SAMPLE_JSON: &str = r#"{
  "tag_name": "v0.2.0",
  "name": "Partner v0.2.0",
  "prerelease": false,
  "assets": [
    {"name": "partner-0.2.0-x86_64.msi", "browser_download_url": "https://github.com/Nariod/Partner/releases/download/v0.2.0/partner-0.2.0-x86_64.msi"},
    {"name": "checksums.txt", "browser_download_url": "https://github.com/Nariod/Partner/releases/download/v0.2.0/checksums.txt"}
  ]
}"#;

#[test]
fn parses_github_release() {
    let rel = parse_release(SAMPLE_JSON).unwrap();
    assert_eq!(rel.tag_name, "v0.2.0");
    assert!(!rel.prerelease);
    assert_eq!(rel.assets.len(), 2);
    let msi = find_asset(&rel, ".msi").unwrap();
    assert!(msi
        .browser_download_url
        .contains("partner-0.2.0-x86_64.msi"));
}

#[test]
fn version_comparison() {
    assert_eq!(version_cmp("v0.2.0", "0.1.9"), Ordering::Greater);
    assert_eq!(version_cmp("v1.0.0", "v1.0.0"), Ordering::Equal);
    assert_eq!(version_cmp("0.9.9", "v1.0.0"), Ordering::Less);
    assert!(is_newer("v0.2.0", "0.1.0"));
    assert!(!is_newer("v0.1.0", "0.1.0"));
    assert!(!is_newer("v0.1.0-rc1", "0.1.0"));
}

#[test]
fn sha256_known_vector() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert!(verify_sha256(
        b"abc",
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad  partner.msi"
    ));
    assert!(!verify_sha256(b"abc", "00"));
    assert!(!verify_sha256(b"abc", ""));
}

#[test]
fn checksums_file_parsing() {
    let text = "aaa111  partner.msi\nbbb222 *partner.zip\n\n";
    let map = parse_checksums(text);
    assert_eq!(map.get("partner.msi").unwrap(), "aaa111");
    assert_eq!(map.get("partner.zip").unwrap(), "bbb222");
}
