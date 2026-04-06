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

    /// The parent node that the position points into.
    pub fn parent(&self) -> &Node {
        &self.path[self.depth].node
    }

    /// The root node in which the position was resolved.
    pub fn doc(&self) -> &Node {
        &self.path[0].node
    }

    /// The ancestor node at the given depth.
    pub fn node(&self, depth: Option<i32>) -> &Node {
        &self.path[self.resolve_depth(depth)].node
    }

    /// The index into the ancestor at the given depth.
    pub fn index(&self, depth: Option<i32>) -> usize {
        self.path[self.resolve_depth(depth)].index
    }

    /// The index pointing after this position into the ancestor at the
    /// given depth. Adds 1 when the position is inside a text node or
    /// when `depth` is higher (closer to root) than `self.depth`.
    pub fn index_after(&self, depth: Option<i32>) -> usize {
        let depth = self.resolve_depth(depth);
        // "at text start" = pos 恰好落在 last frame 的 start_pos(text_offset == 0)
        let at_text_start = self.pos == self.path[self.depth].start_pos;
        let bump = if depth == self.depth && at_text_start { 0 } else { 1 };
        self.path[depth].index + bump
    }

    /// The (absolute) position at the start of the node at the given depth.
    pub fn start(&self, depth: Option<i32>) -> usize {
        let depth = self.resolve_depth(depth);
        if depth == 0 { 0 } else { self.path[depth - 1].start_pos + 1 }
    }

    /// The (absolute) position at the end of the node at the given depth.
    pub fn end(&self, depth: Option<i32>) -> usize {
        let depth = self.resolve_depth(depth);
        // start(depth) + node(depth).content.size
        let start = if depth == 0 { 0 } else { self.path[depth - 1].start_pos + 1 };
        start + self.path[depth].node.content.size
    }

    /// The (absolute) position directly before the wrapping node at the given depth.
    ///
    /// When `depth == self.depth + 1`, returns `self.pos` (the original position).
    /// Returns `Err(RangeError)` at depth 0 (no position before the root).
    pub fn before(&self, depth: Option<i32>) -> Result<usize, RangeError> {
        let depth = self.resolve_depth(depth);
        if depth == 0 {
            return Err(RangeError("There is no position before the top-level node".to_string()));
        }
        if depth == self.depth + 1 {
            return Ok(self.pos);
        }
        Ok(self.path[depth - 1].start_pos)
    }

    /// The (absolute) position directly after the wrapping node at the given depth.
    pub fn after(&self, depth: Option<i32>) -> Result<usize, RangeError> {
        let depth = self.resolve_depth(depth);
        if depth == 0 {
            return Err(RangeError("There is no position after the top-level node".to_string()));
        }
        if depth == self.depth + 1 {
            return Ok(self.pos);
        }
        Ok(self.path[depth - 1].start_pos + self.path[depth].node.node_size())
    }

    /// When this position points into a text node, this returns the distance
    /// between the position and the start of the text node. Will be zero for
    /// positions that point between nodes.
    pub fn text_offset(&self) -> usize {
        self.pos - self.path[self.depth].start_pos
    }

    /// Get the position at the given index in the parent node at the given depth.
    pub fn pos_at_index(&self, index: usize, depth: Option<i32>) -> usize {
        let depth = self.resolve_depth(depth);
        let node = &self.path[depth].node;
        let mut pos = if depth == 0 { 0 } else { self.path[depth - 1].start_pos + 1 };
        for i in 0..index {
            if let Ok(child) = node.child(i) {
                pos += child.node_size();
            }
        }
        pos
    }

    /// Get the node directly after the position, if any. If the position
    /// points into a text node, only the part of that node after the
    /// position is returned.
    pub fn node_after(&self) -> Option<Node> {
        let parent = &self.path[self.depth].node;
        let index = self.path[self.depth].index;
        if index == parent.child_count() {
            return None;
        }
        let d_off = self.text_offset();
        let child = parent.child(index).ok()?;
        if d_off > 0 {
            Some(child.cut(d_off, None))
        } else {
            Some(child.clone())
        }
    }

    /// Get the node directly before the position, if any. If the position
    /// points into a text node, only the part of that node before the
    /// position is returned.
    pub fn node_before(&self) -> Option<Node> {
        let parent = &self.path[self.depth].node;
        let index = self.path[self.depth].index;
        let d_off = self.text_offset();
        if d_off > 0 {
            return parent.child(index).ok().map(|c| c.cut(0, Some(d_off)));
        }
        if index == 0 {
            return None;
        }
        parent.child(index - 1).ok().cloned()
    }

    /// Query whether the given position shares the same parent node.
    pub fn same_parent(&self, other: &ResolvedPos) -> bool {
        self.pos - self.parent_offset == other.pos - other.parent_offset
    }

    /// Return the greater of this and the given position.
    pub fn max<'a>(&'a self, other: &'a ResolvedPos) -> &'a ResolvedPos {
        if other.pos > self.pos { other } else { self }
    }

    /// Return the smaller of this and the given position.
    pub fn min<'a>(&'a self, other: &'a ResolvedPos) -> &'a ResolvedPos {
        if other.pos < self.pos { other } else { self }
    }

    /// The depth up to which this position and the given (non-resolved)
    /// position share the same parent nodes.
    pub fn shared_depth(&self, pos: usize) -> usize {
        for depth in (1..=self.depth).rev() {
            let d = depth as i32;
            if self.start(Some(d)) <= pos && self.end(Some(d)) >= pos {
                return depth;
            }
        }
        0
    }
}
