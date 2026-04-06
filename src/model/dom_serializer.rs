//! src/model/dom_serializer.rs
//!
//! DOMSerializer stub — 所有方法为占位实现，待 cosmic-text 渲染层完成后实现。
//! 参见设计文档决策 1：DOM 相关 API 改为渲染层 stub。

use std::collections::HashMap;
use std::sync::Arc;

use crate::model::fragment::Fragment;
use crate::model::mark::Mark;
use crate::model::node::Node;
use crate::model::render_output::RenderOutput;
use crate::model::schema::Schema;

/// A serializer that can convert ProseMirror nodes to rendered output.
///
/// NOTE: All methods are stubs. Implementation requires cosmic-text integration.
pub struct DOMSerializer {
    /// Serialization functions for node types.
    pub nodes: HashMap<String, Box<dyn Fn(&Node) -> RenderOutput + Send + Sync>>,
    /// Serialization functions for mark types.
    pub marks: HashMap<String, Box<dyn Fn(&Mark, bool) -> RenderOutput + Send + Sync>>,
}

impl DOMSerializer {
    /// Build a serializer from a schema's node and mark specs.
    pub fn from_schema(schema: &Arc<Schema>) -> Self {
        todo!("需要 cosmic-text 集成后实现")
    }

    /// Serialize a node to rendered output.
    pub fn serialize_node(&self, node: &Node) -> RenderOutput {
        todo!("需要 cosmic-text 集成后实现")
    }

    /// Serialize a fragment to rendered output.
    pub fn serialize_fragment(&self, fragment: &Fragment) -> RenderOutput {
        todo!("需要 cosmic-text 集成后实现")
    }
}
