pub mod crypto;
pub mod framing;
pub mod model;
pub mod storage;
pub mod update;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("encryption error: {0}")]
    Encrypt(#[from] age::EncryptError),
    #[error("decryption error: {0}")]
    Decrypt(#[from] age::DecryptError),
    #[error("serialization error: {0}")]
    Encode(#[from] bincode::error::EncodeError),
    #[error("deserialization error: {0}")]
    Decode(#[from] bincode::error::DecodeError),
    #[error("invalid Partner file: {0}")]
    Format(String),
    #[error("this file is already open in another Partner instance")]
    Locked,
    #[error("wrong passphrase or key")]
    BadCredentials,
}

pub type Result<T> = std::result::Result<T, Error>;
