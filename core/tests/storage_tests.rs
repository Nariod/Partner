use partner_core::storage::{acquire_lock, backup_path, load, save_atomic};
use partner_core::Error;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn tempdir(tag: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!(
        "partner-core-test-{tag}-{}-{n}",
        std::process::id()
    ));
    fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn save_atomic_writes_and_rotates_backups() {
    let dir = tempdir("save");
    let file = dir.join("mission.partner");
    for i in 0..7u8 {
        save_atomic(&file, format!("payload-{i}").as_bytes(), 5).unwrap();
    }
    assert_eq!(load(&file).unwrap(), b"payload-6");
    assert_eq!(load(&backup_path(&file, 1)).unwrap(), b"payload-5");
    assert_eq!(load(&backup_path(&file, 5)).unwrap(), b"payload-1");
    assert!(!backup_path(&file, 6).exists());
    assert!(!dir.join("mission.partner.tmp").exists());
    fs::remove_dir_all(&dir).ok();
}

#[cfg(unix)]
#[test]
fn files_are_private_0600() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempdir("perms");
    let file = dir.join("m.partner");
    save_atomic(&file, b"x", 0).unwrap();
    let mode = fs::metadata(&file).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn second_lock_on_same_file_fails() {
    let dir = tempdir("lock");
    let file = dir.join("m.partner");
    fs::write(&file, b"data").unwrap();
    let lock1 = acquire_lock(&file).unwrap();
    let err = acquire_lock(&file).unwrap_err();
    assert!(matches!(err, Error::Locked));
    drop(lock1);
    let _lock2 = acquire_lock(&file).unwrap();
    fs::remove_dir_all(&dir).ok();
}
