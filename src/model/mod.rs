mod comparedeep;
mod tests_ts;
mod schema;
mod mark;
mod content;
mod node;
mod fragment;
mod diff;
mod resolvedpos;
mod replace;

use std::collections::BTreeMap;

/// 属性值，对应 ProseMirror 中 Attrs 值的 `any` 类型。
#[derive(Debug, Clone, PartialEq)]
pub enum AttrValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
}

/// 节点或标记的属性集合。
pub type Attrs = BTreeMap<String, AttrValue>;

pub use schema::{Schema, SchemaSpec, NodeSpec, MarkSpec, SchemaError};
