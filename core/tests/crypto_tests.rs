use partner_core::crypto::{decrypt_document, encrypt_document, generate_keypair, Credential};
use partner_core::framing::{self, FORMAT_VERSION, MAGIC};
use partner_core::model::Document;
use partner_core::Error;

fn sample_doc() -> Document {
    let mut doc = Document::new("Mission Alpha");
    let n = doc.create_node(0, "Findings");
    doc.set_body(n, "# Findings\n\n- 10.0.0.1:22 open\n".repeat(50));
    let blob = doc.add_blob("image/png", vec![0xAB; 1024 * 512]);
    doc.set_body(
        n,
        format!("{}\n![shot](partner://blob/{blob})\n", doc.nodes[&n].body),
    );
    doc
}

#[test]
fn x25519_round_trip() {
    let (id, rec) = generate_keypair();
    let doc = sample_doc();
    let key_cred = Credential::key(id.to_string());
    let ct = encrypt_document(&doc, std::slice::from_ref(&key_cred), false).unwrap();
    assert!(ct.starts_with(b"age-encryption.org/v1"));
    let pt = decrypt_document(&ct, &[key_cred]).unwrap();
    assert_eq!(pt.meta.title, doc.meta.title);
    assert_eq!(pt.nodes.len(), doc.nodes.len());
    assert_eq!(pt.blobs.len(), 1);
    let _ = rec;
}

#[test]
fn passphrase_round_trip_and_wrong_passphrase() {
    let doc = sample_doc();
    let good = Credential::passphrase("correct horse battery staple");
    let ct = encrypt_document(&doc, std::slice::from_ref(&good), false).unwrap();
    let pt = decrypt_document(&ct, &[good]).unwrap();
    assert_eq!(pt.meta.title, "Mission Alpha");
    let bad = Credential::passphrase("wrong passphrase");
    let err = decrypt_document(&ct, &[bad]).unwrap_err();
    assert!(matches!(err, Error::BadCredentials | Error::Decrypt(_)));
}

#[test]
fn multi_recipient_two_keys() {
    let (id1, _rec1) = generate_keypair();
    let (id2, _rec2) = generate_keypair();
    let doc = sample_doc();
    let k1 = Credential::key(id1.to_string());
    let k2 = Credential::key(id2.to_string());
    let ct = encrypt_document(&doc, &[k1.clone(), k2.clone()], false).unwrap();
    assert!(decrypt_document(&ct, &[k1]).is_ok());
    assert!(decrypt_document(&ct, &[k2]).is_ok());
}

#[test]
fn mixing_passphrase_and_key_is_rejected() {
    let (id, _rec) = generate_keypair();
    let doc = sample_doc();
    let pass = Credential::passphrase("s3cret mission");
    let key = Credential::key(id.to_string());
    let err = encrypt_document(&doc, &[pass, key], false).unwrap_err();
    assert!(matches!(err, Error::Format(_)));
}

#[test]
fn tampered_ciphertext_fails() {
    let (id, _rec) = generate_keypair();
    let doc = sample_doc();
    let cred = Credential::key(id.to_string());
    let mut ct = encrypt_document(&doc, std::slice::from_ref(&cred), false).unwrap();
    let last = ct.len() - 1;
    ct[last] ^= 0xFF;
    assert!(decrypt_document(&ct, &[cred]).is_err());
}

#[test]
fn armor_round_trip() {
    let (id, _rec) = generate_keypair();
    let doc = sample_doc();
    let cred = Credential::key(id.to_string());
    let ct = encrypt_document(&doc, std::slice::from_ref(&cred), true).unwrap();
    assert!(ct.starts_with(b"-----BEGIN AGE ENCRYPTED FILE-----"));
    let text = String::from_utf8(ct).unwrap();
    let pt = decrypt_document(text.as_bytes(), &[cred]).unwrap();
    assert_eq!(pt.meta.title, "Mission Alpha");
}

#[test]
fn framing_magic_and_version() {
    let doc = sample_doc();
    let framed = framing::frame(&doc).unwrap();
    assert_eq!(&framed[..7], MAGIC);
    assert_eq!(framed[7], FORMAT_VERSION);
    let back = framing::unframe(&framed).unwrap();
    assert_eq!(back.meta.title, "Mission Alpha");
    let mut bad = framed.clone();
    bad[7] = 99;
    assert!(framing::unframe(&bad).is_err());
    let mut bad2 = framed.clone();
    bad2[0] = b'X';
    assert!(framing::unframe(&bad2).is_err());
}

#[test]
fn large_document_round_trip() {
    let (id, _rec) = generate_keypair();
    let mut doc = Document::new("big");
    let n = doc.create_node(0, "big");
    let mut body = String::new();
    for i in 0..20_000 {
        body.push_str(&format!("line {i}: some audit evidence text\n"));
    }
    doc.set_body(n, body);
    doc.add_blob("image/png", vec![7u8; 2 * 1024 * 1024]);
    let cred = Credential::key(id.to_string());
    let ct = encrypt_document(&doc, std::slice::from_ref(&cred), false).unwrap();
    let pt = decrypt_document(&ct, &[cred]).unwrap();
    assert_eq!(pt.nodes[&n].body.len(), doc.nodes[&n].body.len());
}
