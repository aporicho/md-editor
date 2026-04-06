//! src/model/dom_parser.rs
//!
//! DOMParser stub — 所有方法为占位实现，待 cosmic-text 渲染层完成后实现。
//! 参见设计文档决策 1：DOM 相关 API 改为渲染层 stub。

use std::sync::Arc;

use crate::model::content::ContentMatch;
use crate::model::node::Node;
use crate::model::render_output::RenderOutput;
use crate::model::replace::Slice;
use crate::model::resolved_pos::ResolvedPos;
use crate::model::schema::Schema;

/// Options for parsing DOM-like content.
pub struct ParseOptions {
    pub preserve_whitespace: Option<bool>,
    pub find_positions: Option<Vec<usize>>,
    pub from: Option<usize>,
    pub to: Option<usize>,
    pub top_node: Option<Node>,
    pub top_match: Option<Arc<ContentMatch>>,
    pub context: Option<ResolvedPos>,
}

/// A parser that can convert rendered output back into ProseMirror nodes.
///
/// NOTE: All methods are stubs. Implementation requires cosmic-text integration.
pub struct DOMParser {
    schema: Arc<Schema>,
}

impl DOMParser {
    /// Create a parser for the given schema.
    pub fn from_schema(schema: &Arc<Schema>) -> Self {
        todo!("需要 cosmic-text 集成后实现")
    }

    /// Parse rendered content into a ProseMirror document.
    pub fn parse(&self, input: &RenderOutput, options: Option<ParseOptions>) -> Node {
        todo!("需要 cosmic-text 集成后实现")
    }

    /// Parse rendered content into a slice.
    pub fn parse_slice(&self, input: &RenderOutput, options: Option<ParseOptions>) -> Slice {
        todo!("需要 cosmic-text 集成后实现")
    }
}
