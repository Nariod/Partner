use partner_core::model::{Document, BLOB_URI_SCHEME};

#[test]
fn new_document_has_welcome_node() {
    let doc = Document::new("Mission X");
    assert_eq!(doc.meta.title, "Mission X");
    let ids = doc.all_node_ids();
    assert_eq!(ids.len(), 1);
    assert!(doc.nodes[&ids[0]].body.contains("Welcome"));
}

#[test]
fn create_move_remove_tree_ops() {
    let mut doc = Document::new("t");
    let a = doc.create_node(0, "A");
    let b = doc.create_node(a, "B");
    let c = doc.create_node(0, "C");
    assert!(doc.move_node(c, a, 0));
    assert_eq!(doc.nodes[&a].children, vec![c, b]);
    assert_eq!(doc.nodes[&c].parent, Some(a));
    assert!(!doc.move_node(a, b, 0));
    doc.remove_node(a);
    assert!(!doc.nodes.contains_key(&a));
    assert!(!doc.nodes.contains_key(&b));
    assert!(!doc.nodes.contains_key(&c));
    assert_eq!(doc.nodes.len(), 2);
}

#[test]
fn search_finds_by_title_and_body() {
    let mut doc = Document::new("t");
    let n = doc.create_node(0, "nmap scan");
    doc.set_body(n, "found 445/tcp open smb");
    let hits = doc.search("SMB");
    assert_eq!(hits, vec![n]);
    assert!(doc.search("nothing-here").is_empty());
    assert!(doc.search("   ").is_empty());
}

#[test]
fn blob_gc_keeps_only_referenced() {
    let mut doc = Document::new("t");
    let n = doc.create_node(0, "with image");
    let id1 = doc.add_blob("image/png", vec![1, 2, 3]);
    let id2 = doc.add_blob("image/png", vec![4, 5, 6]);
    doc.set_body(n, format!("![a]({BLOB_URI_SCHEME}{id1})"));
    doc.gc_blobs();
    assert!(doc.blobs.contains_key(&id1));
    assert!(!doc.blobs.contains_key(&id2));
}
