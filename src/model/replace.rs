//! src/model/replace.rs
//!
//! Phase 4.1 — Slice struct + 模块级私有辅助函数（remove_range、insert_into）。
//! Phase 4.5+ 将在本文件续写 replace 算法。
#![allow(dead_code)] // TODO: Phase 4.7 replace 算法完整后移除

use std::sync::OnceLock;

use crate::model::fragment::Fragment;

static EMPTY_SLICE: OnceLock<Slice> = OnceLock::new();

/// A slice represents a piece cut out of a larger document. It
/// stores not only a fragment, but also the depth up to which nodes on
/// both side are 'open' (cut through).
#[derive(Clone)]
pub struct Slice {
    /// The slice's content.
    pub content: Fragment,
    /// The open depth at the start of the fragment.
    pub open_start: usize,
    /// The open depth at the end.
    pub open_end: usize,
}

impl Slice {
    /// Create a slice. When specifying a non-zero open depth, the caller
    /// must ensure that the fragment has nodes at least that deep on the
    /// appropriate side.
    pub fn new(content: Fragment, open_start: usize, open_end: usize) -> Self {
        Slice { content, open_start, open_end }
    }

    /// The empty slice.
    pub fn empty() -> Self {
        EMPTY_SLICE
            .get_or_init(|| Slice {
                content: Fragment::empty(),
                open_start: 0,
                open_end: 0,
            })
            .clone()
    }

    /// The size this slice would add when inserted into a document.
    pub fn size(&self) -> usize {
        self.content.size - self.open_start - self.open_end
    }

    /// Tests whether this slice is equal to another slice.
    pub fn eq(&self, other: &Slice) -> bool {
        self.content.eq(&other.content)
            && self.open_start == other.open_start
            && self.open_end == other.open_end
    }

    /// Return a debugging string like `<paragraph("hi")>(0,1)`.
    pub fn to_string_repr(&self) -> String {
        format!("{}({},{})", self.content.to_string_repr(), self.open_start, self.open_end)
    }
}
