// src/model/content.rs
//
// ContentMatch：节点内容表达式的 DFA 匹配状态机。
// 对应 TypeScript 的 content.ts。
//
// 注意：以下方法为占位实现，依赖 Fragment，将在 Phase 3 补全：
//   - match_fragment(&self, frag: &Fragment, start: usize, end: usize) -> Option<Arc<ContentMatch>>
//   - fill_before(&self, after: &Fragment, to_end: bool, start_index: usize) -> Option<Fragment>
//   - find_wrapping(&self, target_type_name: &str) -> Option<Vec<String>>
//
#[allow(unused_imports)]
use std::sync::Arc;
#[allow(unused_imports)]
use indexmap::IndexMap;

#[allow(unused_imports)]
use crate::model::errors::{RangeError, SyntaxError};

/// Represents a match state of a node type's content expression.
///
/// Instances of this class represent a match state of a node type's
/// content expression, and can be used to find out whether further
/// content matches here, and whether a given position is a valid end
/// of the node.
pub struct ContentMatch {
    /// True when this match state represents a valid end of the node.
    pub valid_end: bool,
    /// True when this match position expects inline content.
    /// Computed at parse time from the first edge's node type's is_inline flag.
    pub inline_content: bool,
    /// Outgoing edges: (type_name, next_state).
    edges: Vec<(String, Arc<ContentMatch>)>,
    /// Cache for find_wrapping results.
    wrap_cache: std::cell::RefCell<Vec<WrapCacheEntry>>,
}

#[allow(dead_code)]
enum WrapCacheEntry {
    Query(String),
    Result(Option<Vec<String>>),
}

impl ContentMatch {
    /// The empty content match (valid end, no edges). Used for leaf nodes.
    pub fn empty() -> Arc<ContentMatch> {
        Arc::new(ContentMatch {
            valid_end: true,
            inline_content: false,
            edges: vec![],
            wrap_cache: std::cell::RefCell::new(vec![]),
        })
    }
}
