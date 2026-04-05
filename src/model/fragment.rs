// src/model/fragment.rs
#![allow(dead_code)] // TODO: Phase 3.2 完整实现后移除
use std::sync::{Arc, OnceLock};
use serde_json::Value;

use crate::model::node::Node;
use crate::model::errors::RangeError;

static EMPTY_FRAGMENT: OnceLock<Fragment> = OnceLock::new();

/// 将字符偏移转换为字节偏移。如果 char_offset >= char count，返回 str.len()。
fn char_offset_to_byte_offset(s: &str, char_offset: usize) -> usize {
    s.char_indices()
        .nth(char_offset)
        .map(|(byte_idx, _)| byte_idx)
        .unwrap_or(s.len())
}

/// A fragment represents a node's collection of child nodes.
///
/// Like nodes, fragments are persistent data structures, and you
/// should not mutate them or their content. Rather, you create new
/// instances whenever needed. The API tries to make this easy.
#[derive(Clone)]
pub struct Fragment {
    pub(crate) content: Arc<[Node]>,
    /// The size of the fragment, which is the total of the size of its content nodes.
    pub size: usize,
}

impl Fragment {
    /// Internal constructor.
    pub(crate) fn new(content: Vec<Node>, size: usize) -> Self {
        Fragment { content: content.into(), size }
    }

    /// An empty fragment. Use this whenever a node doesn't contain anything.
    pub fn empty() -> Self {
        EMPTY_FRAGMENT
            .get_or_init(|| Fragment { content: Arc::from([] as [Node; 0]), size: 0 })
            .clone()
    }

    /// Build a fragment from an array of nodes.
    /// Ensures that adjacent text nodes with the same marks are joined together.
    pub fn from_array(array: &[Node]) -> Self {
        if array.is_empty() {
            return Self::empty();
        }
        let mut nodes: Vec<Node> = Vec::with_capacity(array.len());
        let mut size = 0usize;
        for node in array {
            if node.is_text()
                && let Some(last) = nodes.last_mut()
                && last.is_text() && last.same_markup(node)
            {
                let combined = format!(
                    "{}{}",
                    last.text.as_deref().unwrap_or(""),
                    node.text.as_deref().unwrap_or("")
                );
                size -= last.node_size();
                last.text = Some(combined);
                size += last.node_size();
                continue;
            }
            size += node.node_size();
            nodes.push(node.clone());
        }
        Self::new(nodes, size)
    }

    /// Create a fragment from a single node.
    pub fn from_node(node: Node) -> Self {
        let size = node.node_size();
        Self::new(vec![node], size)
    }

    /// The number of child nodes in this fragment.
    pub fn child_count(&self) -> usize {
        self.content.len()
    }

    /// Get the child node at the given index.
    /// Raises an error when the index is out of range.
    pub fn child(&self, index: usize) -> Result<&Node, RangeError> {
        self.content.get(index).ok_or_else(|| {
            RangeError(format!("Index {} out of range for {}", index, self.to_string_repr()))
        })
    }

    /// Get the child node at the given index, if it exists.
    pub fn maybe_child(&self, index: usize) -> Option<&Node> {
        self.content.get(index)
    }

    /// The first child of the fragment, or `None` if it is empty.
    pub fn first_child(&self) -> Option<&Node> {
        self.content.first()
    }

    /// The last child of the fragment, or `None` if it is empty.
    pub fn last_child(&self) -> Option<&Node> {
        self.content.last()
    }

    /// Call `f` for every child node, passing the node, its offset, and its index.
    pub fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(&Node, usize, usize),
    {
        let mut p = 0usize;
        for (i, child) in self.content.iter().enumerate() {
            f(child, p, i);
            p += child.node_size();
        }
    }

    /// Invoke a callback for all descendant nodes between the given two positions.
    ///
    /// Doesn't descend into a node when the callback returns `false`.
    pub fn nodes_between<F>(
        &self,
        from: usize,
        to: usize,
        f: &mut F,
        node_start: usize,
        parent: Option<&Node>,
    ) where
        F: FnMut(&Node, usize, Option<&Node>, usize) -> bool,
    {
        let mut pos = 0usize;
        for (i, child) in self.content.iter().enumerate() {
            let end = pos + child.node_size();
            if end > from && f(child, node_start + pos, parent, i) && child.content.size > 0 {
                let start = pos + 1;
                child.content.nodes_between(
                    from.saturating_sub(start),
                    child.content.size.min(to.saturating_sub(start)),
                    f,
                    node_start + start,
                    Some(child),
                );
            }
            pos = end;
            if pos >= to {
                break;
            }
        }
    }

    /// Call the given callback for every descendant node.
    pub fn descendants<F>(&self, mut f: F)
    where
        F: FnMut(&Node, usize, Option<&Node>, usize) -> bool,
    {
        self.nodes_between(0, self.size, &mut f, 0, None);
    }

    /// Extract the text between `from` and `to`.
    pub fn text_between(
        &self,
        from: usize,
        to: usize,
        block_separator: Option<&str>,
        leaf_text: Option<&str>,
    ) -> String {
        let mut text = String::new();
        let mut first = true;
        self.nodes_between(
            from,
            to,
            &mut |node: &Node, pos: usize, _parent: Option<&Node>, _idx: usize| -> bool {
                let node_text = if node.is_text() {
                    let t = node.text.as_deref().unwrap_or("");
                    let char_start = from.saturating_sub(pos);
                    let char_end = (to - pos).min(t.chars().count());
                    let byte_start = char_offset_to_byte_offset(t, char_start);
                    let byte_end = char_offset_to_byte_offset(t, char_end);
                    if byte_end > byte_start { t[byte_start..byte_end].to_string() } else { String::new() }
                } else if node.node_type.is_leaf() {
                    if let Some(lt) = leaf_text {
                        lt.to_string()
                    } else if let Some(ref lf) = node.node_type.spec.leaf_text {
                        lf(&Value::Object(
                            node.attrs.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                        ))
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };

                if node.node_type.is_block
                    && (node.node_type.is_leaf() && !node_text.is_empty()
                        || node.node_type.inline_content)
                    && let Some(sep) = block_separator
                {
                    if first {
                        first = false;
                    } else {
                        text.push_str(sep);
                    }
                }
                text.push_str(&node_text);
                !node.node_type.is_leaf()
            },
            0,
            None,
        );
        text
    }

    /// Create a new fragment containing the combined content of this fragment and the other.
    pub fn append(&self, other: &Fragment) -> Fragment {
        if other.size == 0 {
            return self.clone();
        }
        if self.size == 0 {
            return other.clone();
        }
        let mut content: Vec<Node> = self.content.to_vec();
        let mut i = 0usize;
        // Try to merge last of self with first of other (text nodes with same markup)
        if let Some(first_other) = other.content.first()
            && let Some(last) = content.last_mut()
            && last.is_text() && first_other.is_text() && last.same_markup(first_other)
        {
            let combined = format!(
                "{}{}",
                last.text.as_deref().unwrap_or(""),
                first_other.text.as_deref().unwrap_or("")
            );
            last.text = Some(combined);
            i = 1;
        }
        for node in other.content[i..].iter() {
            content.push(node.clone());
        }
        let new_size = self.size + other.size;
        Fragment::new(content, new_size)
    }

    /// Cut out the sub-fragment between the two given positions.
    pub fn cut(&self, from: usize, to: Option<usize>) -> Fragment {
        let to = to.unwrap_or(self.size);
        if from == 0 && to == self.size {
            return self.clone();
        }
        let mut result: Vec<Node> = vec![];
        let mut size = 0usize;
        if to > from {
            let mut pos = 0usize;
            for child in self.content.iter() {
                let end = pos + child.node_size();
                if end > from {
                    let child = if pos < from || end > to {
                        if child.is_text() {
                            let text = child.text.as_deref().unwrap_or("");
                            let char_start = from.saturating_sub(pos);
                            let char_end = (to - pos).min(text.chars().count());
                            let byte_start = char_offset_to_byte_offset(text, char_start);
                            let byte_end = char_offset_to_byte_offset(text, char_end);
                            child.with_text(text[byte_start..byte_end].to_string())
                        } else {
                            let inner_from = from.saturating_sub(pos + 1);
                            let inner_to = child.content.size.min(to.saturating_sub(pos + 1));
                            child.cut_inner(inner_from, inner_to)
                        }
                    } else {
                        child.clone()
                    };
                    size += child.node_size();
                    result.push(child);
                }
                pos = end;
                if pos >= to {
                    break;
                }
            }
        }
        Fragment::new(result, size)
    }

    /// Cut by child index (not position). @internal
    pub(crate) fn cut_by_index(&self, from: usize, to: usize) -> Fragment {
        if from == to {
            return Fragment::empty();
        }
        if from == 0 && to == self.content.len() {
            return self.clone();
        }
        Fragment::from_array(&self.content[from..to])
    }

    /// Create a new fragment in which the node at the given index is replaced by the given node.
    pub fn replace_child(&self, index: usize, node: Node) -> Fragment {
        let current = &self.content[index];
        if std::ptr::eq(current as *const _, &node as *const _) {
            return self.clone();
        }
        let mut content = self.content.to_vec();
        let size = self.size + node.node_size() - current.node_size();
        content[index] = node;
        Fragment::new(content, size)
    }

    /// Create a new fragment by prepending the given node to this fragment.
    pub fn add_to_start(&self, node: Node) -> Fragment {
        let size = self.size + node.node_size();
        let mut content = vec![node];
        content.extend_from_slice(&self.content);
        Fragment::new(content, size)
    }

    /// Create a new fragment by appending the given node to this fragment.
    pub fn add_to_end(&self, node: Node) -> Fragment {
        let size = self.size + node.node_size();
        let mut content = self.content.to_vec();
        content.push(node);
        Fragment::new(content, size)
    }

    /// Compare this fragment to another one.
    pub fn eq(&self, other: &Fragment) -> bool {
        if self.content.len() != other.content.len() {
            return false;
        }
        self.content.iter().zip(other.content.iter()).all(|(a, b)| a.eq(b))
    }

    /// Find the index and inner offset corresponding to a given relative position. @internal
    /// Returns (index, offset).
    pub(crate) fn find_index(&self, pos: usize) -> Result<(usize, usize), RangeError> {
        if pos == 0 {
            return Ok((0, 0));
        }
        if pos == self.size {
            return Ok((self.content.len(), pos));
        }
        if pos > self.size {
            return Err(RangeError(format!(
                "Position {} outside of fragment ({})",
                pos,
                self.to_string_repr()
            )));
        }
        let mut cur_pos = 0usize;
        for (i, child) in self.content.iter().enumerate() {
            let end = cur_pos + child.node_size();
            if end >= pos {
                if end == pos {
                    return Ok((i + 1, end));
                }
                return Ok((i, cur_pos));
            }
            cur_pos = end;
        }
        Err(RangeError("Fragment.find_index: internal error".to_string()))
    }

    /// Find the first position at which this fragment and another fragment differ.
    pub fn find_diff_start(&self, other: &Fragment, pos: usize) -> Option<usize> {
        crate::model::diff::find_diff_start(self, other, pos)
    }

    /// Find the first position (from the end) at which this fragment and the given fragment differ.
    pub fn find_diff_end(
        &self,
        other: &Fragment,
        pos: usize,
        other_pos: usize,
    ) -> Option<(usize, usize)> {
        crate::model::diff::find_diff_end(self, other, pos, other_pos)
    }

    /// Return a debugging string that describes this fragment.
    pub fn to_string_repr(&self) -> String {
        format!("<{}>", self.to_string_inner())
    }

    /// @internal
    pub(crate) fn to_string_inner(&self) -> String {
        self.content
            .iter()
            .map(|n| n.to_string_repr())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Create a JSON-serializeable representation of this fragment.
    pub fn to_json(&self) -> Option<Value> {
        if self.content.is_empty() {
            return None;
        }
        Some(Value::Array(self.content.iter().map(|n| n.to_json()).collect()))
    }

    /// Deserialize a fragment from its JSON representation.
    pub fn from_json(
        schema: &crate::model::schema::Schema,
        value: Option<&Value>,
    ) -> Result<Fragment, RangeError> {
        match value {
            None | Some(Value::Null) => Ok(Fragment::empty()),
            Some(Value::Array(arr)) => {
                let nodes: Result<Vec<Node>, RangeError> = arr
                    .iter()
                    .map(|v| Node::from_json(schema, v))
                    .collect();
                Ok(Fragment::from_array(&nodes?))
            }
            _ => Err(RangeError("Invalid input for Fragment.fromJSON".to_string())),
        }
    }
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

    fn empty_attrs() -> Arc<std::collections::HashMap<String, serde_json::Value>> {
        Arc::new(Default::default())
    }

    #[test]
    fn fragment_empty_has_size_zero() {
        let f = Fragment::empty();
        assert_eq!(f.size, 0);
        assert_eq!(f.child_count(), 0);
    }

    #[test]
    fn fragment_from_array_single_node() {
        let s = simple_schema();
        let n = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "hello".to_string(), vec![]);
        let f = Fragment::from_array(&[n]);
        assert_eq!(f.child_count(), 1);
        assert_eq!(f.size, 5);
    }

    #[test]
    fn fragment_from_array_merges_adjacent_text() {
        let s = simple_schema();
        let n1 = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "foo".to_string(), vec![]);
        let n2 = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "bar".to_string(), vec![]);
        let f = Fragment::from_array(&[n1, n2]);
        assert_eq!(f.child_count(), 1); // merged
        assert_eq!(f.first_child().unwrap().text.as_deref(), Some("foobar"));
    }

    #[test]
    fn fragment_append_combines_sizes() {
        let s = simple_schema();
        let n1 = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "foo".to_string(), vec![]);
        let n2 = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "bar".to_string(), vec![]);
        let f1 = Fragment::from_node(n1);
        let f2 = Fragment::from_node(n2);
        let f = f1.append(&f2);
        assert_eq!(f.child_count(), 1); // adjacent text merged
        assert_eq!(f.size, 6);
    }

    #[test]
    fn fragment_eq_identical() {
        let s = simple_schema();
        let n = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "x".to_string(), vec![]);
        let f1 = Fragment::from_node(n.clone());
        let f2 = Fragment::from_node(n);
        assert!(f1.eq(&f2));
    }

    #[test]
    fn fragment_cut_by_position() {
        let s = simple_schema();
        let n = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "hello".to_string(), vec![]);
        let f = Fragment::from_node(n);
        let cut = f.cut(1, Some(4));
        assert_eq!(cut.first_child().unwrap().text.as_deref(), Some("ell"));
    }

    #[test]
    fn fragment_cut_unicode_safe() {
        let s = simple_schema();
        let n = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "你好世界".to_string(), vec![]);
        let f = Fragment::from_node(n);
        // 字符偏移：切出 "好世"（字符 1..3）
        let cut = f.cut(1, Some(3));
        assert_eq!(cut.first_child().unwrap().text.as_deref(), Some("好世"));
    }
}
