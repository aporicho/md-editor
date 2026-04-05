// src/model/fragment.rs
#![allow(dead_code)]
use std::sync::Arc;
use crate::model::node::Node;

/// A fragment represents a node's collection of child nodes.
///
/// Like nodes, fragments are persistent data structures, and you
/// should not mutate them or their content. Rather, you create new
/// instances whenever needed.
#[derive(Clone)]
pub struct Fragment {
    pub(crate) content: Arc<[Node]>,
    /// The total of the size of its content nodes.
    pub size: usize,
}

impl Fragment {
    pub(crate) fn new(content: Vec<Node>, size: usize) -> Self {
        Fragment { content: content.into(), size }
    }

    /// An empty fragment. Use this whenever a node doesn't contain anything.
    pub fn empty() -> Self {
        Fragment { content: Arc::from([] as [Node; 0]), size: 0 }
    }
}
