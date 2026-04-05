// src/model/mark.rs
//
// Mark：附加在节点上的样式信息（如加粗、链接等）。
// 对应 TypeScript 的 mark.ts。
//
use std::sync::Arc;

use crate::model::schema::{Attrs, MarkType};

/// A mark is a piece of information that can be attached to a node,
/// such as it being emphasized, in code font, or a link. It has a
/// type and optionally a set of attributes that provide further
/// information (such as the target of the link). Marks are created
/// through a `Schema`, which controls which types exist and which
/// attributes they have.
#[derive(Clone)]
pub struct Mark {
    /// The type of this mark.
    pub mark_type: Arc<MarkType>,
    /// The attributes associated with this mark.
    pub attrs: Attrs,
}

impl Mark {
    /// The empty set of marks.
    pub const NONE: &'static [Self] = &[];

    /// Test whether this mark has the same type and attributes as another mark.
    ///
    /// Test whether this mark has the same type and attributes as
    /// another mark.
    pub fn eq_mark(&self, other: &Mark) -> bool {
        std::ptr::eq(self as *const _, other as *const _)
            || (Arc::ptr_eq(&self.mark_type, &other.mark_type)
                && *self.attrs == *other.attrs)
    }

    /// Test whether two sets of marks are identical.
    ///
    /// Test whether two sets of marks are identical.
    pub fn same_set(a: &[Mark], b: &[Mark]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        a.iter().zip(b.iter()).all(|(x, y)| x.eq_mark(y))
    }
}

impl PartialEq for Mark {
    fn eq(&self, other: &Self) -> bool {
        self.eq_mark(other)
    }
}

impl Eq for Mark {}
