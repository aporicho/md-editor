// src/model/node.rs
#![allow(dead_code)]
use std::sync::Arc;
use crate::model::schema::{Attrs, NodeType};
use crate::model::mark::Mark;
use crate::model::fragment::Fragment;

/// This class represents a node in the tree that makes up a ProseMirror document.
///
/// Do not directly mutate the properties of a Node object.
#[derive(Clone)]
pub struct Node {
    /// The type of node that this is.
    pub node_type: Arc<NodeType>,
    /// An object mapping attribute names to values.
    pub attrs: Attrs,
    /// A container holding the node's children.
    pub content: Fragment,
    /// The marks applied to this node.
    pub marks: Vec<Mark>,
    /// For text nodes, this contains the node's text content.
    pub text: Option<String>,
}

impl Node {
    /// Construct a non-text node.
    pub fn new(
        node_type: Arc<NodeType>,
        attrs: Attrs,
        content: Fragment,
        marks: Vec<Mark>,
    ) -> Self {
        Node { node_type, attrs, content, marks, text: None }
    }

    /// Construct a text node. Panics if text is empty.
    pub fn text_node(
        node_type: Arc<NodeType>,
        attrs: Attrs,
        text: String,
        marks: Vec<Mark>,
    ) -> Self {
        assert!(!text.is_empty(), "Empty text nodes are not allowed");
        Node { node_type, attrs, content: Fragment::empty(), marks, text: Some(text) }
    }

    /// The size of this node.
    /// Text nodes: char count of text. Leaf nodes: 1. Others: 2 + content.size.
    pub fn node_size(&self) -> usize {
        if let Some(ref t) = self.text {
            t.chars().count()
        } else if self.node_type.is_leaf() {
            1
        } else {
            2 + self.content.size
        }
    }

    /// True when this is a text node.
    pub fn is_text(&self) -> bool {
        self.node_type.is_text
    }
}
