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

    /// Match a node type by name, returning the next match state if successful.
    ///
    /// Match a node type, returning a match after that node if successful.
    pub fn match_type(&self, type_name: &str) -> Option<&Arc<ContentMatch>> {
        self.edges
            .iter()
            .find(|(name, _)| name == type_name)
            .map(|(_, next)| next)
    }

    /// The number of outgoing edges this node has in the finite automaton
    /// that describes the content expression.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Get the nth outgoing edge from this node in the finite automaton
    /// that describes the content expression.
    ///
    /// Returns `(type_name, next_state)`, or `Err(RangeError)` if n is out of range.
    pub fn edge(&self, n: usize) -> Result<(&str, &Arc<ContentMatch>), RangeError> {
        self.edges
            .get(n)
            .map(|(name, next)| (name.as_str(), next))
            .ok_or_else(|| RangeError(format!("There's no {}th edge in this content match", n)))
    }

    /// Returns true if this match and `other` share at least one outgoing type name.
    ///
    /// Indicates whether this node allows some of the same content as the given match.
    pub fn compatible(&self, other: &ContentMatch) -> bool {
        for (name, _) in &self.edges {
            for (other_name, _) in &other.edges {
                if name == other_name {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state(valid_end: bool, edges: Vec<(&str, Arc<ContentMatch>)>) -> Arc<ContentMatch> {
        Arc::new(ContentMatch {
            valid_end,
            inline_content: false,
            edges: edges.into_iter().map(|(n, s)| (n.to_string(), s)).collect(),
            wrap_cache: std::cell::RefCell::new(vec![]),
        })
    }

    // state_a --"p"--> state_b (valid_end=true)
    fn two_state_dfa() -> Arc<ContentMatch> {
        let state_b = make_state(true, vec![]);
        make_state(false, vec![("p", state_b)])
    }

    #[test]
    fn match_type_hit() {
        let m = two_state_dfa();
        let next = m.match_type("p");
        assert!(next.is_some());
        assert!(next.unwrap().valid_end);
    }

    #[test]
    fn match_type_miss() {
        let m = two_state_dfa();
        assert!(m.match_type("h1").is_none());
    }

    #[test]
    fn empty_valid_end() {
        let m = ContentMatch::empty();
        assert!(m.valid_end);
        assert_eq!(m.edge_count(), 0);
    }

    #[test]
    fn edge_count_and_edge() {
        let state_b = make_state(true, vec![]);
        let state_c = make_state(false, vec![]);
        let m = make_state(false, vec![("p", state_b), ("h1", state_c)]);
        assert_eq!(m.edge_count(), 2);
        let (name, _) = m.edge(0).unwrap();
        assert_eq!(name, "p");
        let (name, _) = m.edge(1).unwrap();
        assert_eq!(name, "h1");
    }

    #[test]
    fn edge_out_of_range() {
        let m = ContentMatch::empty();
        assert!(m.edge(0).is_err());
    }

    #[test]
    fn compatible_same_type() {
        let state_b = make_state(true, vec![]);
        let m1 = make_state(false, vec![("p", state_b.clone())]);
        let m2 = make_state(false, vec![("p", state_b.clone())]);
        assert!(m1.compatible(&m2));
    }

    #[test]
    fn compatible_different_types() {
        let sb = make_state(true, vec![]);
        let sc = make_state(true, vec![]);
        let m1 = make_state(false, vec![("p", sb)]);
        let m2 = make_state(false, vec![("h1", sc)]);
        assert!(!m1.compatible(&m2));
    }
}
