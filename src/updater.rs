use partner_core::update::{self, Release, LATEST_RELEASE_API, USER_AGENT};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::Duration;

pub enum UpdateMsg {
    NoUpdate,
    Available(Release),
    Downloaded {
        version: String,
        path: PathBuf,
        sha256: String,
    },
    Error(String),
}

pub enum UpdateState {
    Idle,
    NoUpdate,
    Available(Release),
    Downloaded {
        version: String,
        path: PathBuf,
        sha256: String,
    },
    Error(String),
}

pub fn is_flatpak() -> bool {
    Path::new("/.flatpak-info").exists() || std::env::var_os("FLATPAK_ID").is_some()
}

pub fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .user_agent(USER_AGENT)
        .build()
        .new_agent()
}

pub fn asset_suffix() -> &'static str {
    if cfg!(windows) {
        ".msi"
    } else {
        ".tar.gz"
    }
}

pub fn check_for_update(current_version: &str, tx: Sender<UpdateMsg>) {
    let msg = fetch_latest(current_version);
    let _ = tx.send(msg);
}

fn fetch_latest(current_version: &str) -> UpdateMsg {
    let run = || -> Result<UpdateMsg, String> {
        let agent = agent();
        let mut resp = agent
            .get(LATEST_RELEASE_API)
            .header("Accept", "application/vnd.github+json")
            .call()
            .map_err(|e| e.to_string())?;
        let text = resp
            .body_mut()
            .read_to_string()
            .map_err(|e| e.to_string())?;
        let release = update::parse_release(&text).map_err(|e| e.to_string())?;
        if release.prerelease || !update::is_newer(&release.tag_name, current_version) {
            Ok(UpdateMsg::NoUpdate)
        } else {
            Ok(UpdateMsg::Available(release))
        }
    };
    match run() {
        Ok(m) => m,
        Err(e) => UpdateMsg::Error(e),
    }
}

pub fn download_update(release: &Release, tx: Sender<UpdateMsg>) {
    let version = release.tag_name.clone();
    let msg = (|| -> Result<UpdateMsg, String> {
        let suffix = asset_suffix();
        let asset = update::find_asset(release, suffix)
            .ok_or_else(|| format!("no asset ending with {suffix} in release {version}"))?;
        let asset_name = asset.name.clone();
        let asset_url = asset.browser_download_url.clone();
        let expected = expected_checksum(release, &asset_name)?;
        let agent = agent();
        let mut resp = agent.get(&asset_url).call().map_err(|e| e.to_string())?;
        let bytes = resp.body_mut().read_to_vec().map_err(|e| e.to_string())?;
        if !update::verify_sha256(&bytes, &expected) {
            return Err("SHA-256 mismatch: refusing to install unverified update".into());
        }
        let dir = std::env::temp_dir().join("partner-update");
        let dest = dir.join(&asset_name);
        partner_core::storage::write_private_file(&dest, &bytes).map_err(|e| e.to_string())?;
        Ok(UpdateMsg::Downloaded {
            version,
            path: dest,
            sha256: expected,
        })
    })();
    let _ = tx.send(match msg {
        Ok(m) => m,
        Err(e) => UpdateMsg::Error(e),
    });
}

fn expected_checksum(release: &Release, asset_name: &str) -> Result<String, String> {
    if let Some(cs) = update::find_asset(release, "checksums.txt") {
        let agent = agent();
        let mut resp = agent
            .get(&cs.browser_download_url)
            .call()
            .map_err(|e| e.to_string())?;
        let text = resp
            .body_mut()
            .read_to_string()
            .map_err(|e| e.to_string())?;
        let map = update::parse_checksums(&text);
        if let Some(h) = map.get(asset_name) {
            return Ok(h.clone());
        }
        return Err(format!("no checksum entry for {asset_name}"));
    }
    if let Some(sha) = update::find_asset(release, &format!("{asset_name}.sha256")) {
        let agent = agent();
        let mut resp = agent
            .get(&sha.browser_download_url)
            .call()
            .map_err(|e| e.to_string())?;
        let text = resp
            .body_mut()
            .read_to_string()
            .map_err(|e| e.to_string())?;
        return Ok(text);
    }
    Err("release publishes no checksums; refusing to download".into())
}

pub fn verify_file_sha256(path: &Path, expected: &str) -> bool {
    match std::fs::read(path) {
        Ok(bytes) => update::verify_sha256(&bytes, expected),
        Err(_) => false,
    }
}

#[cfg(windows)]
pub fn install_and_restart(path: &Path) -> std::io::Result<()> {
    std::process::Command::new("msiexec")
        .arg("/i")
        .arg(path)
        .spawn()?;
    Ok(())
}

#[cfg(not(windows))]
pub fn install_and_restart(_path: &Path) -> std::io::Result<()> {
    Ok(())
}
