use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::HashMap;

pub const REPO_OWNER: &str = "Nariod";
pub const REPO_NAME: &str = "Partner";
pub const LATEST_RELEASE_API: &str = "https://api.github.com/repos/Nariod/Partner/releases/latest";
pub const RELEASES_PAGE: &str = "https://github.com/Nariod/Partner/releases";
pub const USER_AGENT: &str = "partner-updater";

#[derive(Debug, Clone, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Release {
    pub tag_name: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub assets: Vec<ReleaseAsset>,
}

pub fn parse_release(json: &str) -> serde_json::Result<Release> {
    serde_json::from_str(json)
}

pub fn parse_version(v: &str) -> [u64; 4] {
    let v = v.trim().trim_start_matches(['v', 'V']);
    let mut parts = [0u64; 4];
    for (i, p) in v.split('.').take(4).enumerate() {
        let digits: String = p.chars().take_while(|c| c.is_ascii_digit()).collect();
        parts[i] = digits.parse().unwrap_or(0);
    }
    parts
}

pub fn version_cmp(a: &str, b: &str) -> Ordering {
    parse_version(a).cmp(&parse_version(b))
}

pub fn is_newer(latest: &str, current: &str) -> bool {
    version_cmp(latest, current) == Ordering::Greater
}

pub fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    let digest = h.finalize();
    let mut s = String::with_capacity(64);
    for b in digest {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

pub fn verify_sha256(data: &[u8], expected: &str) -> bool {
    let expected = expected.trim().to_lowercase();
    let expected = expected.split_whitespace().next().unwrap_or("");
    !expected.is_empty() && sha256_hex(data) == expected
}

pub fn find_asset<'a>(release: &'a Release, suffix: &str) -> Option<&'a ReleaseAsset> {
    release.assets.iter().find(|a| a.name.ends_with(suffix))
}

pub fn parse_checksums(text: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut it = line.split_whitespace();
        if let (Some(hash), Some(name)) = (it.next(), it.next()) {
            map.insert(
                name.trim_start_matches('*').to_string(),
                hash.to_lowercase(),
            );
        }
    }
    map
}
