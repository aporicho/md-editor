// src/model/node.rs
#![allow(dead_code)] // TODO: Phase 3.2 完整实现后移除
use std::sync::Arc;
use serde_json::Value;

use crate::model::schema::{Attrs, NodeType, MarkType};
use crate::model::mark::Mark;
use crate::model::fragment::Fragment;
use crate::model::errors::RangeError;
use crate::model::compare_deep::compare_deep;

/// This class represents a node in the tree that makes up a ProseMirror document.
///
/// Nodes are persistent data structures. Instead of changing them, you
/// create new ones with the content you want.
///
/// **Do not** directly mutate the properties of a `Node` object.
#[derive(Clone)]
pub struct Node {
    /// The type of node that this is.
    pub node_type: Arc<NodeType>,
    /// An object mapping attribute names to values.
    pub attrs: Attrs,
    /// A container holding the node's children.
    pub content: Fragment,
    /// The marks applied to this node.
    pub marks: Vec<Mark>,
    /// For text nodes, this contains the node's text content.
    pub text: Option<String>,
}

impl Node {
    /// Construct a non-text node.
    pub fn new(
        node_type: Arc<NodeType>,
        attrs: Attrs,
        content: Fragment,
        marks: Vec<Mark>,
    ) -> Self {
        Node { node_type, attrs, content, marks, text: None }
    }

    /// Construct a text node. Panics if text is empty.
    pub fn text_node(
        node_type: Arc<NodeType>,
        attrs: Attrs,
        text: String,
        marks: Vec<Mark>,
    ) -> Self {
        assert!(!text.is_empty(), "Empty text nodes are not allowed");
        Node { node_type, attrs, content: Fragment::empty(), marks, text: Some(text) }
    }

    /// The size of this node.
    ///
    /// For text nodes: character count. For leaf nodes: 1. For others: 2 + content.size.
    pub fn node_size(&self) -> usize {
        if let Some(ref t) = self.text {
            t.chars().count()
        } else if self.node_type.is_leaf() {
            1
        } else {
            2 + self.content.size
        }
    }

    /// The number of children that the node has.
    pub fn child_count(&self) -> usize {
        self.content.child_count()
    }

    /// Get the child node at the given index. Raises an error when the index is out of range.
    pub fn child(&self, index: usize) -> Result<&Node, RangeError> {
        self.content.child(index)
    }

    /// Get the child node at the given index, if it exists.
    pub fn maybe_child(&self, index: usize) -> Option<&Node> {
        self.content.maybe_child(index)
    }

    /// Call `f` for every child node, passing the node, its offset, and its index.
    pub fn for_each<F>(&self, f: F)
    where
        F: FnMut(&Node, usize, usize),
    {
        self.content.for_each(f);
    }

    /// Invoke a callback for all descendant nodes recursively overlapping the given two positions.
    pub fn nodes_between<F>(
        &self,
        from: usize,
        to: usize,
        f: &mut F,
        start_pos: usize,
    ) where
        F: FnMut(&Node, usize, Option<&Node>, usize) -> bool,
    {
        self.content.nodes_between(from, to, f, start_pos, Some(self));
    }

    /// Call the given callback for every descendant node.
    pub fn descendants<F>(&self, mut f: F)
    where
        F: FnMut(&Node, usize, Option<&Node>, usize) -> bool,
    {
        self.nodes_between(0, self.content.size, &mut f, 0);
    }

    /// Concatenates all the text nodes found in this fragment and its children.
    pub fn text_content(&self) -> String {
        if self.is_text() {
            return self.text.clone().unwrap_or_default();
        }
        if self.node_type.is_leaf()
            && let Some(ref lf) = self.node_type.spec.leaf_text
        {
            return lf(&Value::Object(
                self.attrs.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            ));
        }
        self.text_between(0, self.content.size, Some(""), None)
    }

    /// Get all text between positions `from` and `to`.
    pub fn text_between(
        &self,
        from: usize,
        to: usize,
        block_separator: Option<&str>,
        leaf_text: Option<&str>,
    ) -> String {
        self.content.text_between(from, to, block_separator, leaf_text)
    }

    /// 与 `text_between` 相同，但叶节点文本由闭包 `leaf_fn` 决定。
    pub fn text_between_fn<F>(
        &self,
        from: usize,
        to: usize,
        block_separator: Option<&str>,
        leaf_fn: F,
    ) -> String
    where
        F: Fn(&Node) -> String,
    {
        self.content.text_between_fn(from, to, block_separator, leaf_fn)
    }

    /// Returns this node's first child, or `None` if there are no children.
    pub fn first_child(&self) -> Option<&Node> {
        self.content.first_child()
    }

    /// Returns this node's last child, or `None` if there are no children.
    pub fn last_child(&self) -> Option<&Node> {
        self.content.last_child()
    }

    /// Test whether two nodes represent the same piece of document.
    pub fn eq(&self, other: &Node) -> bool {
        std::ptr::eq(self as *const _, other as *const _)
            || (self.same_markup(other) && self.content.eq(&other.content)
                && self.text == other.text)
    }

    /// Check pointer equality (same node instance).
    pub fn ptr_eq(&self, other: &Node) -> bool {
        std::ptr::eq(self as *const _, other as *const _)
    }

    /// Compare the markup (type, attributes, and marks) of this node to those of another.
    pub fn same_markup(&self, other: &Node) -> bool {
        self.has_markup(&other.node_type, Some(&other.attrs), Some(&other.marks))
    }

    /// Check whether this node's markup corresponds to the given type, attributes, and marks.
    pub fn has_markup(
        &self,
        node_type: &Arc<NodeType>,
        attrs: Option<&Attrs>,
        marks: Option<&[Mark]>,
    ) -> bool {
        if !Arc::ptr_eq(&self.node_type, node_type) {
            return false;
        }
        let self_attrs_val = Value::Object(
            self.attrs.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        );
        let other_attrs_val = match attrs {
            Some(a) => Value::Object(a.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
            None => match &node_type.default_attrs {
                Some(da) => Value::Object(da.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
                None => Value::Object(Default::default()),
            },
        };
        if !compare_deep(&self_attrs_val, &other_attrs_val) {
            return false;
        }
        Mark::same_set(&self.marks, marks.unwrap_or(Mark::NONE))
    }

    /// Create a new node with the same markup as this node, containing the given content.
    pub fn copy(&self, content: Option<Fragment>) -> Node {
        let content = content.unwrap_or_else(Fragment::empty);
        Node::new(self.node_type.clone(), self.attrs.clone(), content, self.marks.clone())
    }

    /// Create a copy of this node, with the given set of marks instead of the node's own marks.
    pub fn with_marks(&self, marks: Vec<Mark>) -> Node {
        if Mark::same_set(&marks, &self.marks) {
            return self.clone();
        }
        if self.is_text() {
            Node {
                node_type: self.node_type.clone(),
                attrs: self.attrs.clone(),
                content: Fragment::empty(),
                marks,
                text: self.text.clone(),
            }
        } else {
            Node::new(self.node_type.clone(), self.attrs.clone(), self.content.clone(), marks)
        }
    }

    /// Create a copy of this node with only the content between the given positions.
    pub fn cut_inner(&self, from: usize, to: usize) -> Node {
        if from == 0 && to == self.content.size {
            return self.clone();
        }
        self.copy(Some(self.content.cut(from, Some(to))))
    }

    /// For text nodes: create a copy with the given text.
    pub(crate) fn with_text(&self, text: String) -> Node {
        if self.text.as_deref() == Some(text.as_str()) {
            return self.clone();
        }
        Node {
            node_type: self.node_type.clone(),
            attrs: self.attrs.clone(),
            content: Fragment::empty(),
            marks: self.marks.clone(),
            text: Some(text),
        }
    }

    /// True when this is a text node.
    pub fn is_text(&self) -> bool {
        self.node_type.is_text
    }

    /// True when this is a block (non-inline node).
    pub fn is_block(&self) -> bool {
        self.node_type.is_block
    }

    /// True when this is a textblock node, a block node with inline content.
    pub fn is_textblock(&self) -> bool {
        self.node_type.is_textblock()
    }

    /// True when this node allows inline content.
    pub fn inline_content(&self) -> bool {
        self.node_type.inline_content
    }

    /// True when this is an inline node (a text node or one that can appear among text).
    pub fn is_inline(&self) -> bool {
        self.node_type.is_inline()
    }

    /// True when this is a leaf node.
    pub fn is_leaf(&self) -> bool {
        self.node_type.is_leaf()
    }

    /// True when this is an atom.
    pub fn is_atom(&self) -> bool {
        self.node_type.is_atom()
    }

    /// Return a string representation of this node for debugging purposes.
    pub fn to_string_repr(&self) -> String {
        if let Some(ref f) = self.node_type.spec.to_debug_string {
            let attrs_val = Value::Object(
                self.attrs.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            );
            return f(&attrs_val);
        }
        let name = if self.is_text() {
            serde_json::to_string(self.text.as_deref().unwrap_or("")).unwrap_or_default()
        } else if self.content.size > 0 {
            format!("{}({})", self.node_type.name, self.content.to_string_inner())
        } else {
            self.node_type.name.clone()
        };
        wrap_marks(&self.marks, &name)
    }

    /// Return a JSON-serializeable representation of this node.
    pub fn to_json(&self) -> Value {
        let mut obj = serde_json::json!({ "type": self.node_type.name });
        if !self.attrs.is_empty() {
            obj["attrs"] = Value::Object(
                self.attrs.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            );
        }
        if self.content.size > 0 {
            obj["content"] = self.content.to_json().unwrap_or(Value::Null);
        }
        if !self.marks.is_empty() {
            obj["marks"] = Value::Array(self.marks.iter().map(|m| m.to_json()).collect());
        }
        if let Some(ref t) = self.text {
            obj["text"] = Value::String(t.clone());
        }
        obj
    }

    /// Deserialize a node from its JSON representation.
    pub fn from_json(
        schema: &crate::model::schema::Schema,
        json: &Value,
    ) -> Result<Node, RangeError> {
        if json.is_null() {
            return Err(RangeError("Invalid input for Node.fromJSON".to_string()));
        }
        let marks: Vec<Mark> = if let Some(ms) = json.get("marks") {
            let arr = ms.as_array().ok_or_else(|| {
                RangeError("Invalid mark data for Node.fromJSON".to_string())
            })?;
            arr.iter()
                .map(|m| Mark::from_json(schema, m))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| RangeError(e.0))?
        } else {
            vec![]
        };

        let type_name = json["type"]
            .as_str()
            .ok_or_else(|| RangeError("Invalid node type in JSON".to_string()))?;

        if type_name == "text" {
            let text = json["text"]
                .as_str()
                .ok_or_else(|| RangeError("Invalid text node in JSON".to_string()))?;
            let text_type = schema
                .nodes
                .get("text")
                .ok_or_else(|| RangeError("Schema has no text type".to_string()))?;
            return Ok(Node::text_node(
                text_type.clone(),
                Arc::new(Default::default()),
                text.to_string(),
                marks,
            ));
        }

        let node_type = schema.node_type(type_name).map_err(|e| RangeError(e.0))?;
        let content = Fragment::from_json(schema, json.get("content"))?;
        let attrs: Option<std::collections::HashMap<String, Value>> = json
            .get("attrs")
            .and_then(|v| v.as_object())
            .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
        let attrs = crate::model::schema::compute_attrs(&node_type.spec.attrs, attrs.as_ref())
            .map_err(|e| RangeError(e.0))?;
        Ok(Node::new(node_type.clone(), attrs, content, marks))
    }

    /// Test whether a given mark type occurs in this document between the two given positions.
    pub fn range_has_mark(&self, from: usize, to: usize, mark_type: &MarkType) -> bool {
        let mut found = false;
        if to > from {
            self.nodes_between(
                from,
                to,
                &mut |node: &Node, _pos, _parent, _idx| -> bool {
                    if mark_type.is_in_set(&node.marks).is_some() {
                        found = true;
                    }
                    !found
                },
                0,
            );
        }
        found
    }

    /// Find the node directly after the given position.
    pub fn node_at(&self, mut pos: usize) -> Option<&Node> {
        let mut node = self;
        loop {
            let (index, offset) = node.content.find_index(pos).ok()?;
            let child = node.content.maybe_child(index)?;
            if offset == pos || child.is_text() {
                return Some(child);
            }
            pos -= offset + 1;
            node = child;
        }
    }

    /// Find the (direct) child node after the given offset.
    pub fn child_after(&self, pos: usize) -> Result<(Option<&Node>, usize, usize), RangeError> {
        let (index, offset) = self.content.find_index(pos)?;
        Ok((self.content.maybe_child(index), index, offset))
    }

    /// Find the (direct) child node before the given offset.
    pub fn child_before(&self, pos: usize) -> Result<(Option<&Node>, usize, usize), RangeError> {
        if pos == 0 {
            return Ok((None, 0, 0));
        }
        let (index, offset) = self.content.find_index(pos)?;
        if offset < pos {
            return Ok((self.content.maybe_child(index), index, offset));
        }
        if index == 0 {
            return Ok((None, 0, 0));
        }
        let prev = self.content.maybe_child(index - 1);
        let prev_offset = offset - prev.map_or(0, |n| n.node_size());
        Ok((prev, index - 1, prev_offset))
    }

    // ---- Phase 3.4/4 stubs ----

    /// Get the content match state in this node at the given child index.
    pub fn content_match_at(
        &self,
        index: usize,
    ) -> Option<Arc<crate::model::content::ContentMatch>> {
        self.node_type
            .content_match
            .match_fragment(&self.content, 0, index)
    }

    /// Check whether this node and its descendants conform to the schema,
    /// returning an error if they do not.
    ///
    /// Checks content validity, attribute validity, and mark set validity.
    pub fn check(&self) -> Result<(), crate::model::errors::RangeError> {
        todo!("Node::check: implement in Phase 3.4-c")
    }

    /// Test whether replacing the range `from` to `to` (by child index) with
    /// the given fragment (from `start` to `end`) would leave this node's content valid.
    pub fn can_replace(
        &self,
        from: usize,
        to: usize,
        replacement: &crate::model::fragment::Fragment,
        start: usize,
        end: usize,
    ) -> bool {
        todo!("Node::can_replace: implement in Phase 3.4-c")
    }

    /// Test whether replacing the range `from` to `to` (by child index) with
    /// a node of the given type would leave this node's content valid.
    pub fn can_replace_with(
        &self,
        from: usize,
        to: usize,
        type_name: &str,
        marks: &[crate::model::mark::Mark],
    ) -> bool {
        todo!("Node::can_replace_with: implement in Phase 3.4-c")
    }

    /// Test whether the given node's content could be appended to this node.
    pub fn can_append(&self, other: &Node) -> bool {
        todo!("Node::can_append: implement in Phase 3.4-c")
    }

    /// NOTE: stub — requires Slice + ResolvedPos (Phase 4).
    #[allow(unused)]
    pub fn slice_stub(&self) -> ! {
        todo!("Node::slice requires Slice + ResolvedPos — Phase 4")
    }

    /// NOTE: stub — requires Slice + ResolvedPos (Phase 4).
    #[allow(unused)]
    pub fn replace_stub(&self) -> ! {
        todo!("Node::replace requires Slice + ResolvedPos — Phase 4")
    }

    /// NOTE: stub — requires ResolvedPos (Phase 4).
    #[allow(unused)]
    pub fn resolve_stub(&self) -> ! {
        todo!("Node::resolve requires ResolvedPos — Phase 4")
    }
}

fn wrap_marks(marks: &[Mark], s: &str) -> String {
    let mut result = s.to_string();
    for mark in marks.iter().rev() {
        result = format!("{}({})", mark.mark_type.name, result);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use indexmap::indexmap;
    use crate::model::schema::{Schema, SchemaSpec, NodeSpec, MarkSpec};

    fn simple_schema() -> Arc<Schema> {
        Schema::new(SchemaSpec {
            nodes: indexmap! {
                "doc".to_string() => NodeSpec { content: Some("paragraph+".to_string()), ..NodeSpec::default() },
                "paragraph".to_string() => NodeSpec { content: Some("text*".to_string()), ..NodeSpec::default() },
                "text".to_string() => NodeSpec { inline: true, ..NodeSpec::default() },
            },
            marks: indexmap! { "em".to_string() => MarkSpec::default() },
            top_node: None,
        }).unwrap()
    }

    fn empty_attrs() -> Attrs {
        Arc::new(Default::default())
    }

    #[test]
    fn node_text_size() {
        let s = simple_schema();
        let n = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "hello".to_string(), vec![]);
        assert_eq!(n.node_size(), 5);
        assert!(n.is_text());
        assert!(n.is_leaf());
    }

    #[test]
    fn node_text_unicode_size() {
        let s = simple_schema();
        // "你好" = 2 chars, but 6 bytes
        let n = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "你好".to_string(), vec![]);
        assert_eq!(n.node_size(), 2); // char count, not byte count
    }

    #[test]
    fn node_non_leaf_size_is_two_plus_content() {
        let s = simple_schema();
        let text = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "hi".to_string(), vec![]);
        let content = Fragment::from_node(text);
        let para = Node::new(s.nodes["paragraph"].clone(), empty_attrs(), content, vec![]);
        // para size = 2 + content.size = 2 + 2 = 4
        assert_eq!(para.node_size(), 4);
        assert!(!para.is_leaf());
    }

    #[test]
    fn node_eq_same_content() {
        let s = simple_schema();
        let t1 = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "hi".to_string(), vec![]);
        let t2 = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "hi".to_string(), vec![]);
        assert!(t1.eq(&t2));
    }

    #[test]
    fn node_eq_different_text() {
        let s = simple_schema();
        let t1 = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "hi".to_string(), vec![]);
        let t2 = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "bye".to_string(), vec![]);
        assert!(!t1.eq(&t2));
    }

    #[test]
    fn node_same_markup_true() {
        let s = simple_schema();
        let t1 = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "a".to_string(), vec![]);
        let t2 = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "b".to_string(), vec![]);
        assert!(t1.same_markup(&t2));
    }

    #[test]
    fn node_same_markup_false_different_marks() {
        let s = simple_schema();
        let em = s.marks["em"].create(&s.marks["em"], None).unwrap();
        let t1 = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "a".to_string(), vec![]);
        let t2 = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "a".to_string(), vec![em]);
        assert!(!t1.same_markup(&t2));
    }

    #[test]
    fn node_with_marks_changes_marks() {
        let s = simple_schema();
        let em = s.marks["em"].create(&s.marks["em"], None).unwrap();
        let t = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "x".to_string(), vec![]);
        let t2 = t.with_marks(vec![em.clone()]);
        assert_eq!(t2.marks.len(), 1);
        assert_eq!(t2.text.as_deref(), Some("x"));
    }

    #[test]
    fn node_copy_replaces_content() {
        let s = simple_schema();
        let t = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "x".to_string(), vec![]);
        let new_content = Fragment::from_node(t.clone());
        let para = Node::new(s.nodes["paragraph"].clone(), empty_attrs(), Fragment::empty(), vec![]);
        let para2 = para.copy(Some(new_content));
        assert_eq!(para2.content.child_count(), 1);
    }

    #[test]
    fn node_to_json_text() {
        let s = simple_schema();
        let t = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "hello".to_string(), vec![]);
        let j = t.to_json();
        assert_eq!(j["type"], serde_json::json!("text"));
        assert_eq!(j["text"], serde_json::json!("hello"));
    }

    #[test]
    fn node_from_json_text_roundtrip() {
        let s = simple_schema();
        let t = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "hello".to_string(), vec![]);
        let j = t.to_json();
        let restored = Node::from_json(&s, &j).unwrap();
        assert!(t.eq(&restored));
    }

    #[test]
    fn node_from_json_element() {
        let s = simple_schema();
        let text = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "hi".to_string(), vec![]);
        let para = Node::new(s.nodes["paragraph"].clone(), empty_attrs(), Fragment::from_node(text), vec![]);
        let j = para.to_json();
        let restored = Node::from_json(&s, &j).unwrap();
        assert!(para.eq(&restored));
    }
}
