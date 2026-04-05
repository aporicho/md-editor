//! src/model/replace.rs
//!
//! Phase 4.1 — Slice struct + 模块级私有辅助函数（remove_range、insert_into）。
//! Phase 4.5+ 将在本文件续写 replace 算法。
#![allow(dead_code)] // TODO: Phase 4.7 replace 算法完整后移除

use std::sync::OnceLock;
use serde_json::{Map, Value};

use crate::model::errors::RangeError;
use crate::model::fragment::Fragment;
use crate::model::node::Node;
use crate::model::schema::Schema;

static EMPTY_SLICE: OnceLock<Slice> = OnceLock::new();

fn parse_open_field(value: Option<&Value>) -> Result<usize, RangeError> {
    match value {
        None | Some(Value::Null) => Ok(0),
        Some(v) => v.as_u64()
            .map(|n| n as usize)
            .ok_or_else(|| RangeError("Invalid input for Slice.fromJSON".to_string())),
    }
}

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

    /// Convert a slice to a JSON-serializable representation.
    /// Returns `None` when content is empty.
    pub fn to_json(&self) -> Option<Value> {
        if self.content.size == 0 {
            return None;
        }
        let mut obj = Map::new();
        // Fragment::to_json returns None for empty content, but here content.size > 0
        // so it must return Some; unwrap_or an empty array defensively.
        obj.insert(
            "content".to_string(),
            self.content.to_json().unwrap_or(Value::Array(vec![])),
        );
        if self.open_start > 0 {
            obj.insert("openStart".to_string(), Value::from(self.open_start as u64));
        }
        if self.open_end > 0 {
            obj.insert("openEnd".to_string(), Value::from(self.open_end as u64));
        }
        Some(Value::Object(obj))
    }

    /// Deserialize a slice from its JSON representation.
    /// `None` or `Value::Null` ⇒ empty slice.
    pub fn from_json(schema: &Schema, value: Option<&Value>) -> Result<Slice, RangeError> {
        let Some(v) = value else { return Ok(Slice::empty()); };
        if v.is_null() { return Ok(Slice::empty()); }
        let obj = v.as_object().ok_or_else(|| {
            RangeError("Invalid input for Slice.fromJSON".to_string())
        })?;

        let open_start = parse_open_field(obj.get("openStart"))?;
        let open_end = parse_open_field(obj.get("openEnd"))?;

        let content = Fragment::from_json(schema, obj.get("content"))?;
        Ok(Slice::new(content, open_start, open_end))
    }

    /// Create a slice from a fragment by taking the maximum possible
    /// open value on both sides of the fragment.
    ///
    /// When `open_isolating` is `false`, isolating nodes (those whose spec
    /// has `isolating: true`) are treated as leaves and not descended into.
    pub fn max_open(fragment: Fragment, open_isolating: bool) -> Slice {
        let mut open_start = 0usize;
        let mut n = fragment.first_child();
        while let Some(node) = n {
            if node.is_leaf() { break; }
            if !open_isolating && node.node_type.spec.isolating { break; }
            open_start += 1;
            n = node.first_child();
        }
        let mut open_end = 0usize;
        let mut n = fragment.last_child();
        while let Some(node) = n {
            if node.is_leaf() { break; }
            if !open_isolating && node.node_type.spec.isolating { break; }
            open_end += 1;
            n = node.last_child();
        }
        Slice::new(fragment, open_start, open_end)
    }

    /// @internal — remove a flat range from this slice's content.
    ///
    /// `from` and `to` are slice-relative positions (not including `open_start`);
    /// the implementation shifts them by `open_start` before calling `remove_range`.
    pub fn remove_between(&self, from: usize, to: usize) -> Result<Slice, RangeError> {
        let new_content = remove_range(
            &self.content,
            from + self.open_start,
            to + self.open_start,
        )?;
        Ok(Slice::new(new_content, self.open_start, self.open_end))
    }

    /// @internal — insert `fragment` at slice-relative position `pos`.
    /// Returns `None` when the insertion would violate content constraints
    /// at a nested level.
    pub fn insert_at(&self, pos: usize, fragment: &Fragment) -> Option<Slice> {
        let new_content = insert_into(
            &self.content,
            pos + self.open_start,
            fragment,
            None,
        )?;
        Some(Slice::new(new_content, self.open_start, self.open_end))
    }
}

/// Internal: remove a flat range [from, to) from a fragment.
/// Returns `Err(RangeError)` when the range is not flat (spans multiple
/// non-text children at different depths).
fn remove_range(content: &Fragment, from: usize, to: usize) -> Result<Fragment, RangeError> {
    let (index, offset) = content.find_index(from)?;
    let (index_to, offset_to) = content.find_index(to)?;
    let child_is_text = content.maybe_child(index).map(|n| n.is_text()).unwrap_or(false);

    if offset == from || child_is_text {
        // 平凡删除路径
        if offset_to != to {
            // to 不在子节点边界 ⇒ 必须也是文本节点,否则非平凡
            let child_to_is_text = content.maybe_child(index_to)
                .map(|n| n.is_text())
                .unwrap_or(false);
            if !child_to_is_text {
                return Err(RangeError("Removing non-flat range".to_string()));
            }
        }
        return Ok(content.cut(0, Some(from)).append(&content.cut(to, None)));
    }

    // 非平凡路径:from 在某个非文本子节点内部
    if index != index_to {
        return Err(RangeError("Removing non-flat range".to_string()));
    }
    // 此时 maybe_child(index) 一定是 Some(因为 offset != from 且不在边界)
    let child = content.maybe_child(index)
        .expect("remove_range: child must exist when offset != from");
    let inner = remove_range(&child.content, from - offset - 1, to - offset - 1)?;
    Ok(content.replace_child(index, child.copy(Some(inner))))
}

/// Internal: insert `insert` into `content` at byte position `dist`.
///
/// `parent` is `None` at the top-level call and `Some(child)` when descending
/// into a child node. When `parent` is provided, we check `parent.can_replace`
/// before inserting, returning `None` if the insertion would produce invalid
/// structure.
fn insert_into(
    content: &Fragment,
    dist: usize,
    insert: &Fragment,
    parent: Option<&Node>,
) -> Option<Fragment> {
    let (index, offset) = content.find_index(dist).ok()?;
    let child_is_text = content.maybe_child(index).map(|n| n.is_text()).unwrap_or(false);

    if offset == dist || child_is_text {
        // 边界或文本节点内部 ⇒ 平凡插入
        if let Some(p) = parent
            && !p.can_replace(index, index, insert, 0, insert.child_count())
        {
            return None;
        }
        return Some(
            content.cut(0, Some(dist))
                .append(insert)
                .append(&content.cut(dist, None)),
        );
    }

    // dist 在一个非文本子节点内部 ⇒ 递归下降
    let child = content.maybe_child(index)?;
    let inner = insert_into(&child.content, dist - offset - 1, insert, Some(child))?;
    Some(content.replace_child(index, child.copy(Some(inner))))
}
