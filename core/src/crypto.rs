use crate::{framing, model::Document, Error, Result};
use age::secrecy::{ExposeSecret, SecretString};
use std::io::{Read, Write};

#[derive(Clone)]
pub enum Credential {
    Passphrase(SecretString),
    Key(SecretString),
}

impl Credential {
    pub fn passphrase(p: impl Into<String>) -> Self {
        Self::Passphrase(SecretString::from(p.into()))
    }

    pub fn key(k: impl Into<SecretString>) -> Self {
        Self::Key(k.into())
    }

    fn recipient(&self) -> Result<Box<dyn age::Recipient>> {
        match self {
            Credential::Passphrase(p) => Ok(Box::new(age::scrypt::Recipient::new(p.clone()))),
            Credential::Key(k) => {
                let id = parse_x25519_identity(k.expose_secret())?;
                Ok(Box::new(id.to_public()))
            }
        }
    }

    fn identity(&self) -> Result<Box<dyn age::Identity>> {
        match self {
            Credential::Passphrase(p) => Ok(Box::new(age::scrypt::Identity::new(p.clone()))),
            Credential::Key(k) => Ok(Box::new(parse_x25519_identity(k.expose_secret())?)),
        }
    }
}

pub fn parse_x25519_identity(s: &str) -> Result<age::x25519::Identity> {
    s.trim()
        .parse()
        .map_err(|_| Error::Format("invalid age secret key (expected AGE-SECRET-KEY-1...)".into()))
}

pub fn parse_x25519_recipient(s: &str) -> Result<age::x25519::Recipient> {
    s.trim()
        .parse()
        .map_err(|_| Error::Format("invalid age public key (expected age1...)".into()))
}

pub fn generate_keypair() -> (age::x25519::Identity, age::x25519::Recipient) {
    let id = age::x25519::Identity::generate();
    let rec = id.to_public();
    (id, rec)
}

pub fn identity_secret_string(id: &age::x25519::Identity) -> String {
    id.to_string().expose_secret().to_string()
}

pub fn encrypt_document(doc: &Document, creds: &[Credential], armor: bool) -> Result<Vec<u8>> {
    if creds.is_empty() {
        return Err(Error::Format("no recipient".into()));
    }
    let has_pass = creds.iter().any(|c| matches!(c, Credential::Passphrase(_)));
    let has_key = creds.iter().any(|c| matches!(c, Credential::Key(_)));
    if has_pass && has_key {
        return Err(Error::Format(
            "age does not allow mixing a passphrase with key recipients in one file; use either a passphrase or age key(s)".into(),
        ));
    }
    let payload = framing::frame(doc)?;
    let mut recipients: Vec<Box<dyn age::Recipient>> = Vec::with_capacity(creds.len());
    for c in creds {
        recipients.push(c.recipient()?);
    }
    let encryptor = age::Encryptor::with_recipients(recipients.iter().map(|r| r.as_ref()))?;
    let mut out: Vec<u8> = Vec::with_capacity(payload.len() + 256);
    if armor {
        let armored =
            age::armor::ArmoredWriter::wrap_output(&mut out, age::armor::Format::AsciiArmor)?;
        let mut w = encryptor.wrap_output(armored)?;
        w.write_all(&payload)?;
        w.finish()?.finish()?;
    } else {
        let mut w = encryptor.wrap_output(&mut out)?;
        w.write_all(&payload)?;
        w.finish()?;
    }
    Ok(out)
}

pub fn decrypt_document(data: &[u8], creds: &[Credential]) -> Result<Document> {
    let payload = decrypt_payload(data, creds)?;
    framing::unframe(&payload)
}

pub fn decrypt_payload(data: &[u8], creds: &[Credential]) -> Result<Vec<u8>> {
    if data.is_empty() {
        return Err(Error::Format("empty file".into()));
    }
    if creds.is_empty() {
        return Err(Error::Format("no credential provided".into()));
    }
    let reader: Box<dyn std::io::BufRead + '_> = if data.starts_with(b"-----BEGIN AGE") {
        Box::new(age::armor::ArmoredReader::new(data))
    } else {
        Box::new(data)
    };
    let decryptor = age::Decryptor::new_buffered(reader)?;
    let mut identities: Vec<Box<dyn age::Identity>> = Vec::with_capacity(creds.len());
    for c in creds {
        identities.push(c.identity()?);
    }
    let mut reader = decryptor
        .decrypt(identities.iter().map(|i| i.as_ref()))
        .map_err(|e| match e {
            age::DecryptError::NoMatchingKeys => Error::BadCredentials,
            other => Error::Decrypt(other),
        })?;
    let mut out = Vec::new();
    reader
        .read_to_end(&mut out)
        .map_err(|_| Error::BadCredentials)?;
    Ok(out)
}
