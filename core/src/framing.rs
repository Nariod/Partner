use crate::{model::Document, Error, Result};

pub const MAGIC: &[u8; 7] = b"PARTNER";
pub const FORMAT_VERSION: u8 = 1;

const ZSTD_LEVEL: i32 = 6;

pub fn frame(doc: &Document) -> Result<Vec<u8>> {
    let plain = postcard::to_allocvec(doc)?;
    let mut out = Vec::with_capacity(plain.len() / 2 + 16);
    out.extend_from_slice(MAGIC);
    out.push(FORMAT_VERSION);
    out.extend_from_slice(&zstd::encode_all(&plain[..], ZSTD_LEVEL)?);
    Ok(out)
}

pub fn unframe(bytes: &[u8]) -> Result<Document> {
    if bytes.len() < 8 || &bytes[..7] != MAGIC {
        return Err(Error::Format("not a Partner payload (bad magic)".into()));
    }
    let version = bytes[7];
    if version != FORMAT_VERSION {
        return Err(Error::Format(format!(
            "unsupported payload version {version}"
        )));
    }
    let plain = zstd::decode_all(&bytes[8..])?;
    let doc: Document = postcard::from_bytes(&plain)?;
    Ok(doc)
}
