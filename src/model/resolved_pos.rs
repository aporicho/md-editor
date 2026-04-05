//! src/model/resolved_pos.rs
//!
//! Phase 4.2 — ResolvedPos struct + resolve 算法。
//! Phase 4.3+ 将在本文件续写访问器方法(node/start/end/before/after/marks...)。
#![allow(dead_code)] // TODO: Phase 4.4 访问器完整后移除

use crate::model::errors::RangeError;
use crate::model::node::Node;

/// One level of the resolved-position path.
///
/// Each frame records the parent `node`, the `index` of the child the
/// position descends into at this level, and `start_pos` — the absolute
/// position in the root document where that child's opening tag begins.
#[derive(Clone)]
pub struct PathFrame {
    /// The parent node at this level.
    pub node: Node,
    /// The child index descended into at this level.
    pub index: usize,
    /// Absolute position of that child's opening tag in the root document.
    pub start_pos: usize,
}

/// You can resolve a position to get more information about it.
/// Objects of this class represent such a resolved position.
#[derive(Clone)]
pub struct ResolvedPos {
    /// The position that was resolved.
    pub pos: usize,
    /// Path from root to the parent of the resolved position.
    pub(crate) path: Vec<PathFrame>,
    /// The offset this position has into its parent node.
    pub parent_offset: usize,
    /// The number of levels the parent node is from the root.
    pub depth: usize,
}

impl ResolvedPos {
    /// Construct a resolved position from the computed path. Internal —
    /// normal callers should use [`resolve`] / [`resolve_no_cache`].
    pub(crate) fn new(pos: usize, path: Vec<PathFrame>, parent_offset: usize) -> Self {
        let depth = path.len().saturating_sub(1);
        ResolvedPos { pos, path, parent_offset, depth }
    }

    /// Resolve an optional / negative depth argument against self.depth.
    ///
    /// - `None` → `self.depth`
    /// - `Some(n)` with `n >= 0` → `n as usize`
    /// - `Some(n)` with `n < 0` → `self.depth + n`(wrapping from the bottom)
    pub(crate) fn resolve_depth(&self, val: Option<i32>) -> usize {
        match val {
            None => self.depth,
            Some(v) if v < 0 => {
                let d = self.depth as i32 + v;
                debug_assert!(d >= 0, "resolve_depth: negative depth out of range");
                d.max(0) as usize
            }
            Some(v) => v as usize,
        }
    }

    /// Resolve a position in a document tree without consulting any cache.
    ///
    /// Returns `Err(RangeError)` if `pos` is outside the document range
    /// `[0, doc.content.size]`.
    pub fn resolve_no_cache(doc: &Node, pos: usize) -> Result<ResolvedPos, RangeError> {
        if pos > doc.content.size {
            return Err(RangeError(format!("Position {} out of range", pos)));
        }
        let mut path: Vec<PathFrame> = Vec::new();
        let mut start = 0usize;
        let mut parent_offset = pos;
        let mut node: Node = doc.clone();
        loop {
            let (index, offset) = node.content.find_index(parent_offset)?;
            let rem = parent_offset - offset;
            path.push(PathFrame {
                node: node.clone(),
                index,
                start_pos: start + offset,
            });
            if rem == 0 {
                break;
            }
            let child: Node = node.content.child(index)?.clone();
            if child.is_text() {
                break;
            }
            node = child;
            parent_offset = rem - 1;
            start += offset + 1;
        }
        Ok(ResolvedPos::new(pos, path, parent_offset))
    }

    /// Resolve a position in a document tree.
    ///
    /// Phase 4.2 simply delegates to [`resolve_no_cache`]. TS's
    /// `ResolvedPos.resolveCached` uses a `WeakMap<Node, ResolveCache>`
    /// which requires document-node identity via `Arc` sharing. Rust's
    /// current `Node` is `Clone` by value, so the identity-cache
    /// optimization is deferred to Phase 4.7+ (Decision 32).
    pub fn resolve(doc: &Node, pos: usize) -> Result<ResolvedPos, RangeError> {
        ResolvedPos::resolve_no_cache(doc, pos)
    }
}
