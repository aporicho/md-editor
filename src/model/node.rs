// src/model/node.rs
#![allow(dead_code)] // TODO: Phase 3.2 完整实现后移除
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

    // ---- 占位方法：Task 3 替换为完整实现 ----

    pub fn same_markup(&self, _other: &Node) -> bool {
        // placeholder: Task 3 implements real comparison
        Arc::ptr_eq(&self.node_type, &_other.node_type)
            && *self.attrs == *_other.attrs
            && crate::model::mark::Mark::same_set(&self.marks, &_other.marks)
    }

    pub fn with_text(&self, text: String) -> Node {
        // placeholder: Task 3 implements full version
        Node {
            node_type: self.node_type.clone(),
            attrs: self.attrs.clone(),
            content: Fragment::empty(),
            marks: self.marks.clone(),
            text: Some(text),
        }
    }

    pub fn cut_inner(&self, from: usize, to: usize) -> Node {
        // placeholder: Task 3 implements full version
        if from == 0 && to == self.content.size {
            return self.clone();
        }
        Node {
            node_type: self.node_type.clone(),
            attrs: self.attrs.clone(),
            content: self.content.cut(from, Some(to)),
            marks: self.marks.clone(),
            text: None,
        }
    }

    pub fn eq(&self, other: &Node) -> bool {
        // placeholder: Task 3 implements full version
        std::ptr::eq(self as *const _, other as *const _)
            || (self.same_markup(other) && self.content.eq(&other.content)
                && self.text == other.text)
    }

    pub fn ptr_eq(&self, other: &Node) -> bool {
        std::ptr::eq(self as *const _, other as *const _)
    }

    pub fn to_string_repr(&self) -> String {
        // placeholder: Task 3 implements full version
        self.node_type.name.clone()
    }

    pub fn to_json(&self) -> serde_json::Value {
        // placeholder: Task 3 implements full version
        let mut obj = serde_json::json!({ "type": self.node_type.name });
        if let Some(ref t) = self.text {
            obj["text"] = serde_json::Value::String(t.clone());
        }
        obj
    }

    pub fn from_json(
        _schema: &crate::model::schema::Schema,
        _json: &serde_json::Value,
    ) -> Result<Node, crate::model::errors::RangeError> {
        Err(crate::model::errors::RangeError(
            "Node::from_json: placeholder — implement in Task 3".to_string(),
        ))
    }
}
