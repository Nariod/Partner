use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub type NodeId = u64;

pub const BLOB_URI_SCHEME: &str = "partner://blob/";

pub const WELCOME_BODY: &str = "# Welcome to Partner\n\nPartner stores all your notes for one mission in a **single encrypted file** (`.partner`), using the [age](https://age-encryption.org/v1) format.\n\n- Create notes and sub-notes in the tree on the left (drag & drop to move them)\n- Paste screenshots with the image button: they are embedded in the encrypted file\n- `Ctrl+P` quick switcher, `Ctrl+Shift+P` command palette, `Ctrl+S` save\n- The app locks itself after inactivity; your passphrase is never stored\n\nDelete this note when you are ready.\n";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Document {
    pub meta: Meta,
    pub nodes: BTreeMap<NodeId, Node>,
    pub blobs: BTreeMap<String, Blob>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Meta {
    pub title: String,
    pub root: NodeId,
    pub next_node_id: NodeId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub title: String,
    pub body: String,
    pub collapsed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Blob {
    pub mime: String,
    pub data: Vec<u8>,
}

impl Document {
    pub fn new(title: impl Into<String>) -> Self {
        let mut nodes = BTreeMap::new();
        nodes.insert(
            0,
            Node {
                id: 0,
                parent: None,
                children: Vec::new(),
                title: String::new(),
                body: String::new(),
                collapsed: false,
            },
        );
        let mut doc = Self {
            meta: Meta {
                title: title.into(),
                root: 0,
                next_node_id: 1,
            },
            nodes,
            blobs: BTreeMap::new(),
        };
        let welcome = doc.create_node(0, "Welcome");
        if let Some(n) = doc.nodes.get_mut(&welcome) {
            n.body = WELCOME_BODY.to_string();
        }
        doc
    }

    pub fn create_node(&mut self, parent: NodeId, title: impl Into<String>) -> NodeId {
        let id = self.meta.next_node_id;
        self.meta.next_node_id += 1;
        let node = Node {
            id,
            parent: Some(parent),
            children: Vec::new(),
            title: title.into(),
            body: String::new(),
            collapsed: false,
        };
        self.nodes.insert(id, node);
        if let Some(p) = self.nodes.get_mut(&parent) {
            p.children.push(id);
        }
        id
    }

    pub fn remove_node(&mut self, id: NodeId) {
        if id == self.meta.root {
            return;
        }
        let Some(node) = self.nodes.get(&id) else {
            return;
        };
        let children = node.children.clone();
        let parent = node.parent;
        for c in children {
            self.remove_node(c);
        }
        if let Some(p) = parent {
            if let Some(p) = self.nodes.get_mut(&p) {
                p.children.retain(|&c| c != id);
            }
        }
        self.nodes.remove(&id);
        self.gc_blobs();
    }

    pub fn move_node(&mut self, id: NodeId, new_parent: NodeId, index: usize) -> bool {
        if id == self.meta.root || id == new_parent {
            return false;
        }
        if self.is_ancestor(id, new_parent) {
            return false;
        }
        let Some(node) = self.nodes.get(&id) else {
            return false;
        };
        let old_parent = node.parent;
        if let Some(old) = old_parent {
            if let Some(p) = self.nodes.get_mut(&old) {
                p.children.retain(|&c| c != id);
            }
        }
        if let Some(p) = self.nodes.get_mut(&new_parent) {
            let idx = index.min(p.children.len());
            p.children.insert(idx, id);
        }
        if let Some(n) = self.nodes.get_mut(&id) {
            n.parent = Some(new_parent);
        }
        true
    }

    pub fn is_ancestor(&self, ancestor: NodeId, id: NodeId) -> bool {
        let mut cur = Some(id);
        while let Some(c) = cur {
            if c == ancestor {
                return true;
            }
            cur = self.nodes.get(&c).and_then(|n| n.parent);
        }
        false
    }

    pub fn set_title(&mut self, id: NodeId, title: impl Into<String>) {
        if let Some(n) = self.nodes.get_mut(&id) {
            n.title = title.into();
        }
    }

    pub fn set_body(&mut self, id: NodeId, body: impl Into<String>) {
        if let Some(n) = self.nodes.get_mut(&id) {
            n.body = body.into();
        }
    }

    pub fn set_collapsed(&mut self, id: NodeId, collapsed: bool) {
        if let Some(n) = self.nodes.get_mut(&id) {
            n.collapsed = collapsed;
        }
    }

    pub fn add_blob(&mut self, mime: impl Into<String>, data: Vec<u8>) -> String {
        let id = random_blob_id();
        self.blobs.insert(
            id.clone(),
            Blob {
                mime: mime.into(),
                data,
            },
        );
        id
    }

    pub fn referenced_blobs(&self) -> BTreeSet<String> {
        let mut set = BTreeSet::new();
        for n in self.nodes.values() {
            let mut rest = n.body.as_str();
            while let Some(pos) = rest.find(BLOB_URI_SCHEME) {
                let after = &rest[pos + BLOB_URI_SCHEME.len()..];
                let id: String = after
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                    .collect();
                if !id.is_empty() {
                    set.insert(id);
                }
                rest = after;
            }
        }
        set
    }

    pub fn gc_blobs(&mut self) {
        let keep = self.referenced_blobs();
        self.blobs.retain(|k, _| keep.contains(k));
    }

    pub fn search(&self, query: &str) -> Vec<NodeId> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return Vec::new();
        }
        self.nodes
            .values()
            .filter(|n| {
                n.id != self.meta.root
                    && (n.title.to_lowercase().contains(&q) || n.body.to_lowercase().contains(&q))
            })
            .map(|n| n.id)
            .collect()
    }

    pub fn all_node_ids(&self) -> Vec<NodeId> {
        self.nodes
            .keys()
            .copied()
            .filter(|&id| id != self.meta.root)
            .collect()
    }

    pub fn word_count(&self, id: NodeId) -> usize {
        self.nodes
            .get(&id)
            .map(|n| n.body.split_whitespace().count())
            .unwrap_or(0)
    }
}

pub fn random_blob_id() -> String {
    let mut b = [0u8; 16];
    getrandom::fill(&mut b).expect("CSPRNG failure");
    let mut s = String::with_capacity(32);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}
