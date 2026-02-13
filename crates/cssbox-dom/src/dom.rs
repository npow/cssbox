//! DOM tree types for the layout engine.

use std::collections::HashMap;

/// A unique identifier for a DOM node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DomNodeId(pub usize);

/// The type of a DOM node.
#[derive(Debug, Clone)]
pub enum DomNodeKind {
    /// Document root.
    Document,
    /// An element with a tag name and attributes.
    Element {
        tag: String,
        attributes: HashMap<String, String>,
        namespace: String,
    },
    /// A text node.
    Text(String),
    /// A comment node (ignored for layout).
    Comment(String),
}

/// A single DOM node.
#[derive(Debug, Clone)]
pub struct DomNode {
    pub id: DomNodeId,
    pub kind: DomNodeKind,
    pub children: Vec<DomNodeId>,
    pub parent: Option<DomNodeId>,
}

impl DomNode {
    pub fn tag_name(&self) -> Option<&str> {
        match &self.kind {
            DomNodeKind::Element { tag, .. } => Some(tag),
            _ => None,
        }
    }

    pub fn get_attribute(&self, name: &str) -> Option<&str> {
        match &self.kind {
            DomNodeKind::Element { attributes, .. } => attributes.get(name).map(|s| s.as_str()),
            _ => None,
        }
    }

    pub fn text_content(&self) -> Option<&str> {
        match &self.kind {
            DomNodeKind::Text(text) => Some(text),
            _ => None,
        }
    }

    pub fn is_element(&self) -> bool {
        matches!(self.kind, DomNodeKind::Element { .. })
    }
}

/// Arena-allocated DOM tree.
#[derive(Debug, Clone)]
pub struct DomTree {
    nodes: Vec<DomNode>,
    root: DomNodeId,
}

impl DomTree {
    pub fn new() -> Self {
        let root_node = DomNode {
            id: DomNodeId(0),
            kind: DomNodeKind::Document,
            children: Vec::new(),
            parent: None,
        };
        Self {
            nodes: vec![root_node],
            root: DomNodeId(0),
        }
    }

    pub fn root(&self) -> DomNodeId {
        self.root
    }

    pub fn add_element(
        &mut self,
        parent: DomNodeId,
        tag: &str,
        attributes: HashMap<String, String>,
    ) -> DomNodeId {
        let id = DomNodeId(self.nodes.len());
        self.nodes.push(DomNode {
            id,
            kind: DomNodeKind::Element {
                tag: tag.to_string(),
                attributes,
                namespace: "http://www.w3.org/1999/xhtml".to_string(),
            },
            children: Vec::new(),
            parent: Some(parent),
        });
        self.nodes[parent.0].children.push(id);
        id
    }

    pub fn add_text(&mut self, parent: DomNodeId, text: &str) -> DomNodeId {
        let id = DomNodeId(self.nodes.len());
        self.nodes.push(DomNode {
            id,
            kind: DomNodeKind::Text(text.to_string()),
            children: Vec::new(),
            parent: Some(parent),
        });
        self.nodes[parent.0].children.push(id);
        id
    }

    pub fn node(&self, id: DomNodeId) -> &DomNode {
        &self.nodes[id.0]
    }

    pub fn children(&self, id: DomNodeId) -> &[DomNodeId] {
        &self.nodes[id.0].children
    }

    pub fn parent(&self, id: DomNodeId) -> Option<DomNodeId> {
        self.nodes[id.0].parent
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Find the <body> element.
    pub fn find_body(&self) -> Option<DomNodeId> {
        self.find_element_by_tag("body")
    }

    /// Find the first element with a given tag name (depth-first).
    pub fn find_element_by_tag(&self, tag: &str) -> Option<DomNodeId> {
        self.find_element_recursive(self.root, tag)
    }

    fn find_element_recursive(&self, node: DomNodeId, tag: &str) -> Option<DomNodeId> {
        let n = &self.nodes[node.0];
        if let Some(t) = n.tag_name() {
            if t.eq_ignore_ascii_case(tag) {
                return Some(node);
            }
        }
        for &child in &n.children {
            if let Some(found) = self.find_element_recursive(child, tag) {
                return Some(found);
            }
        }
        None
    }

    /// Find an element by its `id` attribute.
    pub fn find_element_by_id(&self, id: &str) -> Option<DomNodeId> {
        self.find_by_attr_recursive(self.root, "id", id)
    }

    fn find_by_attr_recursive(
        &self,
        node: DomNodeId,
        attr: &str,
        value: &str,
    ) -> Option<DomNodeId> {
        let n = &self.nodes[node.0];
        if let Some(v) = n.get_attribute(attr) {
            if v == value {
                return Some(node);
            }
        }
        for &child in &n.children {
            if let Some(found) = self.find_by_attr_recursive(child, attr, value) {
                return Some(found);
            }
        }
        None
    }

    /// Iterate over all nodes in depth-first order.
    pub fn iter_dfs(&self) -> DfsIter<'_> {
        DfsIter {
            tree: self,
            stack: vec![self.root],
        }
    }
}

impl Default for DomTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Depth-first iterator over DOM nodes.
pub struct DfsIter<'a> {
    tree: &'a DomTree,
    stack: Vec<DomNodeId>,
}

impl<'a> Iterator for DfsIter<'a> {
    type Item = DomNodeId;

    fn next(&mut self) -> Option<DomNodeId> {
        let id = self.stack.pop()?;
        let node = self.tree.node(id);
        // Push children in reverse order so left children are visited first
        for child in node.children.iter().rev() {
            self.stack.push(*child);
        }
        Some(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dom_tree_build() {
        let mut tree = DomTree::new();
        let root = tree.root();
        let body = tree.add_element(root, "body", HashMap::new());
        let div = tree.add_element(body, "div", HashMap::new());
        tree.add_text(div, "Hello");

        assert_eq!(tree.len(), 4);
        assert_eq!(tree.children(root).len(), 1);
        assert_eq!(tree.find_body(), Some(body));
    }

    #[test]
    fn test_find_by_id() {
        let mut tree = DomTree::new();
        let root = tree.root();
        let mut attrs = HashMap::new();
        attrs.insert("id".to_string(), "target".to_string());
        let el = tree.add_element(root, "div", attrs);

        assert_eq!(tree.find_element_by_id("target"), Some(el));
        assert_eq!(tree.find_element_by_id("missing"), None);
    }
}
