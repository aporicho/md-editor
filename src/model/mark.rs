// src/model/mark.rs
//
// Mark：附加在节点上的样式信息（如加粗、链接等）。
// 对应 TypeScript 的 mark.ts。
//
#![allow(dead_code)] // TODO: Phase 3.2+ 中 Fragment/Node 使用后移除
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

    /// Given a set of marks, create a new set which contains this one as
    /// well, in the right position. If this mark is already in the set,
    /// the set itself is returned. If any marks that are set to be
    /// exclusive with this mark are present, those are replaced by this one.
    pub fn add_to_set(&self, set: &[Mark]) -> Vec<Mark> {
        let mut copy: Option<Vec<Mark>> = None;
        let mut placed = false;

        for (i, other) in set.iter().enumerate() {
            if self.eq_mark(other) {
                return set.to_vec();
            }
            if self.mark_type.excludes(other.mark_type.as_ref()) {
                if copy.is_none() {
                    copy = Some(set[..i].to_vec());
                }
                // don't push `other` (it's excluded by self)
            } else if other.mark_type.excludes(self.mark_type.as_ref()) {
                return set.to_vec(); // self cannot be added
            } else {
                if !placed && other.mark_type.rank > self.mark_type.rank {
                    if copy.is_none() {
                        copy = Some(set[..i].to_vec());
                    }
                    copy.as_mut().unwrap().push(self.clone());
                    placed = true;
                }
                if let Some(ref mut c) = copy {
                    c.push(other.clone());
                }
            }
        }

        let mut result = copy.unwrap_or_else(|| set.to_vec());
        if !placed {
            result.push(self.clone());
        }
        result
    }

    /// Remove this mark from the given set, returning a new set. If this
    /// mark is not in the set, the set itself is returned.
    pub fn remove_from_set(&self, set: &[Mark]) -> Vec<Mark> {
        for (i, m) in set.iter().enumerate() {
            if self.eq_mark(m) {
                let mut result = set[..i].to_vec();
                result.extend_from_slice(&set[i + 1..]);
                return result;
            }
        }
        set.to_vec()
    }

    /// Test whether this mark is in the given set of marks.
    pub fn is_in_set(&self, set: &[Mark]) -> bool {
        set.iter().any(|m| self.eq_mark(m))
    }

    /// Convert this mark to a JSON-serializeable representation.
    pub fn to_json(&self) -> serde_json::Value {
        let mut obj = serde_json::json!({ "type": self.mark_type.name });
        if !self.attrs.is_empty() {
            let attrs_map: serde_json::Map<String, serde_json::Value> = self
                .attrs
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            obj["attrs"] = serde_json::Value::Object(attrs_map);
        }
        obj
    }

    /// Deserialize a mark from JSON.
    pub fn from_json(
        schema: &crate::model::schema::Schema,
        json: &serde_json::Value,
    ) -> Result<Self, crate::model::errors::RangeError> {
        use crate::model::errors::RangeError;
        let type_name = json
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RangeError("Invalid input for Mark.fromJSON".to_string()))?;
        let mark_type = schema.marks.get(type_name).ok_or_else(|| {
            RangeError(format!("There is no mark type {} in this schema", type_name))
        })?;
        let attrs: Option<std::collections::HashMap<String, serde_json::Value>> = json
            .get("attrs")
            .and_then(|v| v.as_object())
            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
        mark_type
            .create(mark_type, attrs.as_ref())
            .map_err(|e| RangeError(e.0))
    }

    /// Create a properly sorted mark set from `None`, a single mark, or an
    /// unsorted slice of marks.
    pub fn set_from(marks: Option<&[Mark]>) -> Vec<Mark> {
        match marks {
            None | Some([]) => vec![],
            Some(marks) => {
                let mut copy = marks.to_vec();
                copy.sort_by_key(|m| m.mark_type.rank);
                copy
            }
        }
    }
}

impl PartialEq for Mark {
    fn eq(&self, other: &Self) -> bool {
        self.eq_mark(other)
    }
}

impl Eq for Mark {}
