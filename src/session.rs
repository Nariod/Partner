use partner_core::{
    crypto::Credential,
    model::{Document, NodeId},
    storage::FileLock,
};
use std::path::PathBuf;
use zeroize::Zeroizing;

pub struct OpenDoc {
    pub path: PathBuf,
    pub doc: Document,
    pub creds: Vec<Credential>,
    pub dirty: bool,
    pub selected: NodeId,
    pub lock: FileLock,
}

pub enum Screen {
    Welcome,
    Unlock {
        path: PathBuf,
        lock: FileLock,
        pass: Zeroizing<String>,
        key: Zeroizing<String>,
        use_key: bool,
        error: Option<String>,
    },
    Editor,
}
