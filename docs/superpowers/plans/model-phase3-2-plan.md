# model Phase 3.2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 `Fragment`、`Node`、`diff.rs` 三个核心模块，并补全 `NodeType`/`Schema` 的节点创建方法。

**Architecture:** Fragment（`Arc<[Node]>`，不可变共享切片）与 Node（`text: Option<String>` 区分文本节点）互相依赖，同 crate 内 use 引用，Rust 编译器统一解析，无需特殊处理。diff.rs 提供两个纯函数。依赖 Slice/ResolvedPos 的 Node 方法继续留 `_stub`，依赖 `fill_before` 的 `create_and_fill` 继续留 stub，均在 Phase 3.4/4 补全。

**Tech Stack:** Rust 2024 edition, `std::sync::Arc`, `indexmap`, `serde_json`

**TS 参考：** `reference/prosemirror-model/src/fragment.ts`, `node.ts`, `diff.ts`

---

## 文件结构

```
src/model/fragment.rs       ← 新建：Fragment struct + 所有方法
src/model/node.rs           ← 新建：Node struct + 所有方法（部分 stub）
src/model/diff.rs           ← 新建：find_diff_start, find_diff_end
src/model/tests_diff.rs     ← 新建：diff 函数测试
src/model/mod.rs            ← 修改：声明 4 个新模块
src/model/schema.rs         ← 修改：替换 NodeType::create/create_checked + Schema::node/text/mark
```

---

## 背景

已有（不修改）：
- `Mark { mark_type: Arc<MarkType>, attrs: Attrs }` — 全部方法完整
- `NodeType` — `is_leaf()`, `is_inline()`, `is_atom()`, `allows_mark_type()`, `whitespace()` 等
- `MarkType::create(self: &Arc<Self>, attrs) -> Result<Mark, SchemaError>` 已实现
- `ContentMatch::empty()` — 返回有效的空内容匹配（valid_end=true, no edges）
- `compare_deep(a: &Value, b: &Value) -> bool` — Phase 1
- `compute_attrs(specs, values) -> Result<Attrs, SchemaError>` — Phase 1

需要替换的 stub（`-> !` 签名）：
- `NodeType::create_stub`, `create_checked_stub`（`valid_content_stub`, `allows_marks_stub`, `allowed_marks_stub` 在本阶段也实现）
- `Schema::node_stub`, `text_stub`, `mark_stub`

---

## Task 1: Fragment + Node struct 骨架 + mod.rs

**Files:**
- Create: `src/model/fragment.rs`
- Create: `src/model/node.rs`
- Modify: `src/model/mod.rs`

- [ ] **Step 1: 创建 fragment.rs 骨架**

```rust
// src/model/fragment.rs
#![allow(dead_code)]
use std::sync::Arc;
use crate::model::node::Node;

/// A fragment represents a node's collection of child nodes.
///
/// Like nodes, fragments are persistent data structures, and you
/// should not mutate them or their content. Rather, you create new
/// instances whenever needed.
#[derive(Clone)]
pub struct Fragment {
    pub(crate) content: Arc<[Node]>,
    /// The total of the size of its content nodes.
    pub size: usize,
}

impl Fragment {
    pub(crate) fn new(content: Vec<Node>, size: usize) -> Self {
        Fragment { content: content.into(), size }
    }

    /// An empty fragment. Use this whenever a node doesn't contain anything.
    pub fn empty() -> Self {
        Fragment { content: Arc::from([] as [Node; 0]), size: 0 }
    }
}
```

- [ ] **Step 2: 创建 node.rs 骨架**

```rust
// src/model/node.rs
#![allow(dead_code)]
use std::sync::Arc;
use crate::model::schema::{Attrs, NodeType};
use crate::model::mark::Mark;
use crate::model::fragment::Fragment;

/// This class represents a node in the tree that makes up a ProseMirror document.
///
/// Do not directly mutate the properties of a Node object.
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
    /// Text nodes: byte count of text. Leaf nodes: 1. Others: 2 + content.size.
    pub fn node_size(&self) -> usize {
        if let Some(ref t) = self.text {
            t.len()
        } else if self.node_type.is_leaf() {
            1
        } else {
            2 + self.content.size
        }
    }

    /// True when this is a text node.
    pub fn is_text(&self) -> bool {
        self.node_type.is_text
    }
}
```

- [ ] **Step 3: 更新 mod.rs**

```rust
// src/model/mod.rs（追加）
pub mod fragment;
pub mod node;
pub mod diff;
#[cfg(test)]
mod tests_diff;
```

创建占位 `src/model/diff.rs`：
```rust
// src/model/diff.rs
#![allow(dead_code)]
use crate::model::fragment::Fragment;

pub fn find_diff_start(_a: &Fragment, _b: &Fragment, _pos: usize) -> Option<usize> { todo!() }
pub fn find_diff_end(_a: &Fragment, _b: &Fragment, _pos_a: usize, _pos_b: usize) -> Option<(usize, usize)> { todo!() }
```

创建占位 `src/model/tests_diff.rs`：
```rust
// src/model/tests_diff.rs
```

- [ ] **Step 4: 验证编译**

```bash
cargo check
```

Expected: 无报错（可能有 unused warning，正常）

- [ ] **Step 5: Commit**

```bash
git add src/model/fragment.rs src/model/node.rs src/model/diff.rs src/model/tests_diff.rs src/model/mod.rs
git commit -m "feat(model): add Fragment and Node struct skeletons with mod declarations"
```

---

## Task 2: Fragment 全部方法

**Files:**
- Modify: `src/model/fragment.rs`

**TS 参考：** `reference/prosemirror-model/src/fragment.ts`

- [ ] **Step 1: 将 fragment.rs 替换为完整实现**

```rust
// src/model/fragment.rs
#![allow(dead_code)]
use std::sync::Arc;
use serde_json::Value;

use crate::model::node::Node;
use crate::model::errors::RangeError;

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

    /// An empty fragment.
    pub fn empty() -> Self {
        Fragment { content: Arc::from([] as [Node; 0]), size: 0 }
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
            if let (Some(last), true) = (nodes.last_mut(), node.is_text()) {
                if last.is_text() && last.same_markup(node) {
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

    /// Create a fragment from a Fragment, Node, or slice of nodes.
    /// Returns empty fragment for None.
    pub fn from_option(f: Option<Fragment>) -> Self {
        f.unwrap_or_else(Self::empty)
    }

    /// The number of child nodes in this fragment.
    pub fn child_count(&self) -> usize {
        self.content.len()
    }

    /// Get the child node at the given index.
    /// Raises an error when the index is out of range.
    pub fn child(&self, index: usize) -> Result<&Node, RangeError> {
        self.content.get(index).ok_or_else(|| {
            RangeError(format!("Index {} out of range for {}", index, self.to_string()))
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
    ///
    /// Call `f` for every child node, passing the node, its offset
    /// into this parent node, and its index.
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
    /// Invoke a callback for all descendant nodes between the given two
    /// positions (relative to start of this fragment). Doesn't descend
    /// into a node when the callback returns `false`.
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
                    (child.content.size).min(to.saturating_sub(start)),
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
    ///
    /// Extract the text between `from` and `to`. See the same method on `Node`.
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
                    let start = from.saturating_sub(pos);
                    let end = (to - pos).min(t.len());
                    if end > start { t[start..end].to_string() } else { String::new() }
                } else if node.node_type.is_leaf() {
                    if let Some(lt) = leaf_text {
                        lt.to_string()
                    } else if let Some(ref lf) = node.node_type.spec.leaf_text {
                        lf(&serde_json::to_value(&*node.attrs).unwrap_or(Value::Null))
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };

                if node.node_type.is_block
                    && (node.node_type.is_leaf() && !node_text.is_empty()
                        || node.node_type.inline_content)
                {
                    if let Some(sep) = block_separator {
                        if first {
                            first = false;
                        } else {
                            text.push_str(sep);
                        }
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
    ///
    /// Create a new fragment containing the combined content of this
    /// fragment and the other.
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
        if let (Some(last), Some(first)) = (content.last_mut(), other.content.first()) {
            if last.is_text() && first.is_text() && last.same_markup(first) {
                let combined = format!(
                    "{}{}",
                    last.text.as_deref().unwrap_or(""),
                    first.text.as_deref().unwrap_or("")
                );
                last.text = Some(combined);
                i = 1;
            }
        }
        for node in other.content[i..].iter() {
            content.push(node.clone());
        }
        Fragment::new(content, self.size + other.size)
    }

    /// Cut out the sub-fragment between the two given positions.
    ///
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
                            let start = from.saturating_sub(pos);
                            let end_idx = (to - pos).min(text.len());
                            child.with_text(text[start..end_idx].to_string())
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

    /// Cut by child index (not position).
    ///
    /// @internal
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
    ///
    /// Create a new fragment in which the node at the given index is
    /// replaced by the given node.
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
    ///
    /// Create a new fragment by prepending the given node to this
    /// fragment.
    pub fn add_to_start(&self, node: Node) -> Fragment {
        let size = self.size + node.node_size();
        let mut content = vec![node];
        content.extend_from_slice(&self.content);
        Fragment::new(content, size)
    }

    /// Create a new fragment by appending the given node to this fragment.
    ///
    /// Create a new fragment by appending the given node to this
    /// fragment.
    pub fn add_to_end(&self, node: Node) -> Fragment {
        let size = self.size + node.node_size();
        let mut content = self.content.to_vec();
        content.push(node);
        Fragment::new(content, size)
    }

    /// Compare this fragment to another one.
    ///
    /// Compare this fragment to another one.
    pub fn eq(&self, other: &Fragment) -> bool {
        if self.content.len() != other.content.len() {
            return false;
        }
        self.content.iter().zip(other.content.iter()).all(|(a, b)| a.eq(b))
    }

    /// Find the index and inner offset corresponding to a given relative position.
    ///
    /// @internal Returns (index, offset).
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
                self.to_string()
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
    ///
    /// Find the first position at which this fragment and another
    /// fragment differ, or `None` if they are the same.
    pub fn find_diff_start(&self, other: &Fragment, pos: usize) -> Option<usize> {
        crate::model::diff::find_diff_start(self, other, pos)
    }

    /// Find the first position (from the end) at which this fragment and the given fragment differ.
    ///
    /// Find the first position, searching from the end, at which this
    /// fragment and the given fragment differ, or `None` if they are
    /// the same.
    pub fn find_diff_end(
        &self,
        other: &Fragment,
        pos: usize,
        other_pos: usize,
    ) -> Option<(usize, usize)> {
        crate::model::diff::find_diff_end(self, other, pos, other_pos)
    }

    /// Return a debugging string that describes this fragment.
    pub fn to_string(&self) -> String {
        format!("<{}>", self.to_string_inner())
    }

    /// @internal
    pub(crate) fn to_string_inner(&self) -> String {
        self.content
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Create a JSON-serializeable representation of this fragment.
    ///
    /// Create a JSON-serializeable representation of this fragment.
    pub fn to_json(&self) -> Option<Value> {
        if self.content.is_empty() {
            return None;
        }
        Some(Value::Array(self.content.iter().map(|n| n.to_json()).collect()))
    }

    /// Deserialize a fragment from its JSON representation.
    ///
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
    use serde_json::json;
    use crate::model::schema::{Schema, SchemaSpec, NodeSpec, MarkSpec, AttrSpec};

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

    #[test]
    fn fragment_empty_has_size_zero() {
        let f = Fragment::empty();
        assert_eq!(f.size, 0);
        assert_eq!(f.child_count(), 0);
    }

    #[test]
    fn fragment_from_array_single_node() {
        let s = simple_schema();
        let text_type = s.nodes["text"].clone();
        let attrs = Arc::new(std::collections::HashMap::new());
        let n = Node::text_node(text_type, attrs, "hello".to_string(), vec![]);
        let f = Fragment::from_array(&[n]);
        assert_eq!(f.child_count(), 1);
        assert_eq!(f.size, 5);
    }

    #[test]
    fn fragment_from_array_merges_adjacent_text() {
        let s = simple_schema();
        let text_type = s.nodes["text"].clone();
        let attrs = Arc::new(std::collections::HashMap::new());
        let n1 = Node::text_node(text_type.clone(), attrs.clone(), "foo".to_string(), vec![]);
        let n2 = Node::text_node(text_type.clone(), attrs.clone(), "bar".to_string(), vec![]);
        let f = Fragment::from_array(&[n1, n2]);
        assert_eq!(f.child_count(), 1); // merged
        assert_eq!(f.first_child().unwrap().text.as_deref(), Some("foobar"));
    }

    #[test]
    fn fragment_append_combines_sizes() {
        let s = simple_schema();
        let text_type = s.nodes["text"].clone();
        let attrs = Arc::new(std::collections::HashMap::new());
        let n1 = Node::text_node(text_type.clone(), attrs.clone(), "foo".to_string(), vec![]);
        let n2 = Node::text_node(text_type.clone(), attrs.clone(), "bar".to_string(), vec![]);
        let f1 = Fragment::from_node(n1);
        let f2 = Fragment::from_node(n2);
        let f = f1.append(&f2);
        assert_eq!(f.child_count(), 1); // adjacent text merged
        assert_eq!(f.size, 6);
    }

    #[test]
    fn fragment_eq_identical() {
        let s = simple_schema();
        let text_type = s.nodes["text"].clone();
        let attrs = Arc::new(std::collections::HashMap::new());
        let n = Node::text_node(text_type, attrs, "x".to_string(), vec![]);
        let f1 = Fragment::from_node(n.clone());
        let f2 = Fragment::from_node(n);
        assert!(f1.eq(&f2));
    }

    #[test]
    fn fragment_cut_by_position() {
        let s = simple_schema();
        let text_type = s.nodes["text"].clone();
        let attrs = Arc::new(std::collections::HashMap::new());
        let n = Node::text_node(text_type, attrs, "hello".to_string(), vec![]);
        let f = Fragment::from_node(n);
        let cut = f.cut(1, Some(4));
        assert_eq!(cut.first_child().unwrap().text.as_deref(), Some("ell"));
    }
}
```

- [ ] **Step 2: 验证编译**

```bash
cargo check
```

Expected: 无报错（Node 方法 `same_markup`, `with_text`, `cut_inner`, `eq`, `to_string`, `to_json`, `from_json` 还未定义，会报错）

这是预期的——Task 3 会实现 Node 方法。暂时在 node.rs 中加占位方法：

在 `node.rs` 的 `impl Node` 中追加：
```rust
    pub fn same_markup(&self, other: &Node) -> bool { false }
    pub fn with_text(&self, _t: String) -> Node { self.clone() }
    pub fn cut_inner(&self, _from: usize, _to: usize) -> Node { self.clone() }
    pub fn eq(&self, _other: &Node) -> bool { false }
    pub fn to_string(&self) -> String { self.node_type.name.clone() }
    pub fn to_json(&self) -> Value { Value::Null }
    pub fn from_json(_schema: &crate::model::schema::Schema, _json: &Value) -> Result<Node, crate::model::errors::RangeError> { unimplemented!() }
```

```bash
cargo check
```

Expected: 无报错

- [ ] **Step 3: 运行 Fragment 测试**

```bash
cargo test -p md-editor model::fragment::tests
```

Expected: 部分通过（from_array_merges, append 可能因 same_markup 占位返回 false 而失败），先确认编译通过即可。

- [ ] **Step 4: Commit**

```bash
git add src/model/fragment.rs src/model/node.rs src/model/diff.rs src/model/tests_diff.rs src/model/mod.rs
git commit -m "feat(model): add Fragment with all methods and Node skeleton"
```

---

## Task 3: Node 全部方法（替换占位实现）

**Files:**
- Modify: `src/model/node.rs`

- [ ] **Step 1: 将 node.rs 替换为完整实现**

```rust
// src/model/node.rs
#![allow(dead_code)]
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
    /// The size of this node, as defined by the integer-based indexing
    /// scheme. For text nodes, this is the amount of characters. For other
    /// leaf nodes, it is one. For non-leaf nodes, it is the size of the
    /// content plus two (the start and end token).
    pub fn node_size(&self) -> usize {
        if let Some(ref t) = self.text {
            t.len()
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
    ///
    /// Invoke a callback for all descendant nodes recursively overlapping
    /// the given two positions that are relative to start of this
    /// node's content. The last parameter can be used to specify a
    /// starting position to count from.
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
        if self.node_type.is_leaf() {
            if let Some(ref lf) = self.node_type.spec.leaf_text {
                return lf(&Value::Object(
                    self.attrs.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                ));
            }
        }
        self.text_between(0, self.content.size, Some(""), None)
    }

    /// Get all text between positions `from` and `to`.
    ///
    /// Get all text between positions `from` and `to`. When
    /// `blockSeparator` is given, it will be inserted to separate text
    /// from different block nodes.
    pub fn text_between(
        &self,
        from: usize,
        to: usize,
        block_separator: Option<&str>,
        leaf_text: Option<&str>,
    ) -> String {
        self.content.text_between(from, to, block_separator, leaf_text)
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
    ///
    /// Test whether two nodes represent the same piece of document.
    pub fn eq(&self, other: &Node) -> bool {
        std::ptr::eq(self as *const _, other as *const _)
            || (self.same_markup(other) && self.content.eq(&other.content))
    }

    /// Test whether two nodes are the same piece of document (pointer equality).
    pub fn ptr_eq(&self, other: &Node) -> bool {
        std::ptr::eq(self as *const _, other as *const _)
    }

    /// Compare the markup (type, attributes, and marks) of this node to those of another.
    ///
    /// Compare the markup (type, attributes, and marks) of this node to
    /// those of another. Returns `true` if both have the same markup.
    pub fn same_markup(&self, other: &Node) -> bool {
        self.has_markup(&other.node_type, Some(&other.attrs), Some(&other.marks))
    }

    /// Check whether this node's markup corresponds to the given type, attributes, and marks.
    ///
    /// Check whether this node's markup correspond to the given type,
    /// attributes, and marks.
    pub fn has_markup(
        &self,
        node_type: &Arc<NodeType>,
        attrs: Option<&Attrs>,
        marks: Option<&[Mark]>,
    ) -> bool {
        if !Arc::ptr_eq(&self.node_type, node_type) {
            return false;
        }
        // Compare attrs via compare_deep
        let self_attrs_val = Value::Object(
            self.attrs.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        );
        let other_attrs_val = match attrs {
            Some(a) => Value::Object(a.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
            None => {
                // use node_type.default_attrs or empty
                match &node_type.default_attrs {
                    Some(da) => Value::Object(da.iter().map(|(k,v)| (k.clone(),v.clone())).collect()),
                    None => Value::Object(Default::default()),
                }
            }
        };
        if !compare_deep(&self_attrs_val, &other_attrs_val) {
            return false;
        }
        Mark::same_set(
            &self.marks,
            marks.unwrap_or(Mark::NONE),
        )
    }

    /// Create a new node with the same markup as this node, containing the given content.
    ///
    /// Create a new node with the same markup as this node, containing
    /// the given content (or empty, if no content is given).
    pub fn copy(&self, content: Option<Fragment>) -> Node {
        let content = content.unwrap_or_else(Fragment::empty);
        Node::new(self.node_type.clone(), self.attrs.clone(), content, self.marks.clone())
    }

    /// Create a copy of this node, with the given set of marks instead of the node's own marks.
    ///
    /// Create a copy of this node, with the given set of marks instead
    /// of the node's own marks.
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
    pub fn to_string(&self) -> String {
        if let Some(ref f) = self.node_type.spec.to_debug_string {
            let attrs_val = Value::Object(self.attrs.iter().map(|(k,v)| (k.clone(), v.clone())).collect());
            return f(&attrs_val);
        }
        let mut name = self.node_type.name.clone();
        if self.is_text() {
            name = serde_json::to_string(self.text.as_deref().unwrap_or(""))
                .unwrap_or_default();
        } else if self.content.size > 0 {
            name = format!("{}({})", name, self.content.to_string_inner());
        }
        wrap_marks(&self.marks, &name)
    }

    /// Return a JSON-serializeable representation of this node.
    ///
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
    ///
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

        let type_name = json["type"].as_str()
            .ok_or_else(|| RangeError("Invalid node type in JSON".to_string()))?;

        if type_name == "text" {
            let text = json["text"].as_str()
                .ok_or_else(|| RangeError("Invalid text node in JSON".to_string()))?;
            let text_type = schema.nodes.get("text")
                .ok_or_else(|| RangeError("Schema has no text type".to_string()))?;
            return Ok(Node::text_node(
                text_type.clone(),
                Arc::new(Default::default()),
                text.to_string(),
                marks,
            ));
        }

        let node_type = schema.node_type(type_name)
            .map_err(|e| RangeError(e.0))?;
        let content = Fragment::from_json(schema, json.get("content"))?;
        let attrs: Option<std::collections::HashMap<String, Value>> = json
            .get("attrs")
            .and_then(|v| v.as_object())
            .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
        let attrs = crate::model::schema::compute_attrs(&node_type.spec.attrs, attrs.as_ref())
            .map_err(|e| RangeError(e.0))?;
        Ok(Node::new(node_type.clone(), attrs, content, marks))
    }

    /// Test whether a given mark or mark type occurs in this document between the two given positions.
    ///
    /// Test whether a given mark or mark type occurs in this document
    /// between the two given positions.
    pub fn range_has_mark(&self, from: usize, to: usize, mark_type: &MarkType) -> bool {
        let mut found = false;
        if to > from {
            self.nodes_between(from, to, &mut |node: &Node, _pos, _parent, _idx| -> bool {
                if mark_type.is_in_set(&node.marks).is_some() {
                    found = true;
                }
                !found
            }, 0);
        }
        found
    }

    // ---- Phase 3.4/4 stubs ----

    /// Get the content match in this node at the given index.
    ///
    /// NOTE: stub — requires ContentMatch::match_fragment (Phase 3.4).
    #[allow(unused)]
    pub fn content_match_at_stub(&self) -> ! {
        todo!("Node::content_match_at requires ContentMatch::match_fragment — Phase 3.4")
    }

    /// Test whether replacing the range between `from` and `to` with the given replacement fragment
    /// would leave the node's content valid.
    ///
    /// NOTE: stub — requires ContentMatch::match_fragment (Phase 3.4).
    #[allow(unused)]
    pub fn can_replace_stub(&self) -> ! {
        todo!("Node::can_replace requires match_fragment — Phase 3.4")
    }

    /// Test whether replacing the range `from` to `to` with a node of the given type would
    /// leave the node's content valid.
    ///
    /// NOTE: stub — requires ContentMatch::match_fragment (Phase 3.4).
    #[allow(unused)]
    pub fn can_replace_with_stub(&self) -> ! {
        todo!("Node::can_replace_with requires match_fragment — Phase 3.4")
    }

    /// Test whether the given node's content could be appended to this node.
    ///
    /// NOTE: stub — requires ContentMatch (Phase 3.4).
    #[allow(unused)]
    pub fn can_append_stub(&self) -> ! {
        todo!("Node::can_append requires match_fragment — Phase 3.4")
    }

    /// Check whether this node and its descendants conform to the schema.
    ///
    /// NOTE: stub — requires valid_content (Phase 3.4).
    #[allow(unused)]
    pub fn check_stub(&self) -> ! {
        todo!("Node::check requires valid_content — Phase 3.4")
    }

    /// Cut out the part of the document between the given positions, and return it as a Slice.
    ///
    /// NOTE: stub — requires Slice + ResolvedPos (Phase 4).
    #[allow(unused)]
    pub fn slice_stub(&self) -> ! {
        todo!("Node::slice requires Slice + ResolvedPos — Phase 4")
    }

    /// Replace the part of the document between the given positions with the given slice.
    ///
    /// NOTE: stub — requires Slice + ResolvedPos (Phase 4).
    #[allow(unused)]
    pub fn replace_stub(&self) -> ! {
        todo!("Node::replace requires Slice + ResolvedPos — Phase 4")
    }

    /// Resolve the given position in the document.
    ///
    /// NOTE: stub — requires ResolvedPos (Phase 4).
    #[allow(unused)]
    pub fn resolve_stub(&self) -> ! {
        todo!("Node::resolve requires ResolvedPos — Phase 4")
    }

    /// Find the node directly after the given position.
    ///
    /// NOTE: stub — requires find_index (available) but needs ResolvedPos-like logic.
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

    fn empty_attrs() -> Attrs { Arc::new(Default::default()) }

    #[test]
    fn node_text_size() {
        let s = simple_schema();
        let n = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "hello".to_string(), vec![]);
        assert_eq!(n.node_size(), 5);
        assert!(n.is_text());
        assert!(n.is_leaf());
    }

    #[test]
    fn node_leaf_size_is_one() {
        // image-like node (leaf, non-text) — use paragraph with no content as proxy
        // Actually paragraph has content, use text node with empty marks to represent inline leaf
        // Better: use a schema with an atom node
        let s = simple_schema();
        let text = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "x".to_string(), vec![]);
        // text is a leaf → size 1? No: text node size = text.len() = 1
        // We need is_leaf() for text type
        assert!(s.nodes["text"].is_leaf()); // text has no content expression
        assert_eq!(text.node_size(), 1); // "x" has len=1
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
    fn node_same_markup_true() {
        let s = simple_schema();
        let t1 = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "a".to_string(), vec![]);
        let t2 = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "b".to_string(), vec![]);
        assert!(t1.same_markup(&t2));
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
}
```

- [ ] **Step 2: 运行测试**

```bash
cargo test -p md-editor model::node::tests model::fragment::tests
```

Expected: 全部通过（Fragment 的合并测试现在也会通过，因为 same_markup 已实现）

- [ ] **Step 3: Commit**

```bash
git add src/model/node.rs src/model/fragment.rs
git commit -m "feat(model): complete Node and Fragment methods"
```

---

## Task 4: diff.rs + tests_diff.rs

**Files:**
- Modify: `src/model/diff.rs`
- Modify: `src/model/tests_diff.rs`

- [ ] **Step 1: 实现 diff.rs**

```rust
// src/model/diff.rs
use crate::model::fragment::Fragment;

/// Find the first position at which two fragments differ.
///
/// Find the first position at which fragment `a` and fragment `b`
/// differ, or `None` if they are the same.
pub fn find_diff_start(a: &Fragment, b: &Fragment, pos: usize) -> Option<usize> {
    let mut pos = pos;
    let mut i = 0usize;
    loop {
        if i == a.child_count() || i == b.child_count() {
            return if a.child_count() == b.child_count() { None } else { Some(pos) };
        }
        let child_a = a.content[i].clone();
        let child_b = &b.content[i];

        // Same pointer: identical node, skip
        if child_a.ptr_eq(child_b) {
            pos += child_a.node_size();
            i += 1;
            continue;
        }

        if !child_a.same_markup(child_b) {
            return Some(pos);
        }

        if child_a.is_text() {
            let ta = child_a.text.as_deref().unwrap_or("");
            let tb = child_b.text.as_deref().unwrap_or("");
            if ta != tb {
                let common = ta.chars().zip(tb.chars()).take_while(|(a, b)| a == b).count();
                return Some(pos + common);
            }
        }

        if child_a.content.size > 0 || child_b.content.size > 0 {
            if let Some(inner) = find_diff_start(&child_a.content, &child_b.content, pos + 1) {
                return Some(inner);
            }
        }

        pos += child_a.node_size();
        i += 1;
    }
}

/// Find the first position (from the end) at which two fragments differ.
///
/// Find the first position, searching from the end, at which fragment `a`
/// and fragment `b` differ. Returns `(pos_in_a, pos_in_b)` or `None`.
pub fn find_diff_end(
    a: &Fragment,
    b: &Fragment,
    pos_a: usize,
    pos_b: usize,
) -> Option<(usize, usize)> {
    let mut pos_a = pos_a;
    let mut pos_b = pos_b;
    let mut ia = a.child_count();
    let mut ib = b.child_count();

    loop {
        if ia == 0 || ib == 0 {
            return if ia == ib { None } else { Some((pos_a, pos_b)) };
        }
        ia -= 1;
        ib -= 1;
        let child_a = &a.content[ia];
        let child_b = &b.content[ib];
        let size = child_a.node_size();

        if child_a.ptr_eq(child_b) {
            pos_a -= size;
            pos_b -= size;
            continue;
        }

        if !child_a.same_markup(child_b) {
            return Some((pos_a, pos_b));
        }

        if child_a.is_text() {
            let ta = child_a.text.as_deref().unwrap_or("");
            let tb = child_b.text.as_deref().unwrap_or("");
            if ta != tb {
                let ta_chars: Vec<char> = ta.chars().collect();
                let tb_chars: Vec<char> = tb.chars().collect();
                let min_size = ta_chars.len().min(tb_chars.len());
                let mut same = 0usize;
                while same < min_size
                    && ta_chars[ta_chars.len() - same - 1]
                        == tb_chars[tb_chars.len() - same - 1]
                {
                    same += 1;
                    pos_a -= 1;
                    pos_b -= 1;
                }
                return Some((pos_a, pos_b));
            }
        }

        if child_a.content.size > 0 || child_b.content.size > 0 {
            if let Some(inner) =
                find_diff_end(&child_a.content, &child_b.content, pos_a - 1, pos_b - 1)
            {
                return inner.into();
            }
        }

        pos_a -= size;
        pos_b -= size;
    }
}
```

- [ ] **Step 2: 实现 tests_diff.rs**

```rust
// src/model/tests_diff.rs
use std::sync::Arc;
use indexmap::indexmap;
use crate::model::fragment::Fragment;
use crate::model::node::Node;
use crate::model::schema::{Schema, SchemaSpec, NodeSpec, MarkSpec};

fn schema() -> Arc<Schema> {
    Schema::new(SchemaSpec {
        nodes: indexmap! {
            "doc".to_string() => NodeSpec { content: Some("paragraph+".to_string()), ..NodeSpec::default() },
            "paragraph".to_string() => NodeSpec { content: Some("text*".to_string()), ..NodeSpec::default() },
            "heading".to_string() => NodeSpec { content: Some("text*".to_string()), group: Some("block".to_string()), ..NodeSpec::default() },
            "text".to_string() => NodeSpec { inline: true, ..NodeSpec::default() },
        },
        marks: indexmap! { "em".to_string() => MarkSpec::default(), "strong".to_string() => MarkSpec::default() },
        top_node: None,
    }).unwrap()
}

fn empty_attrs() -> Arc<std::collections::HashMap<String, serde_json::Value>> {
    Arc::new(Default::default())
}

fn text(s: &Arc<Schema>, t: &str) -> Node {
    Node::text_node(s.nodes["text"].clone(), empty_attrs(), t.to_string(), vec![])
}

fn text_em(s: &Arc<Schema>, t: &str) -> Node {
    let em = s.marks["em"].create(&s.marks["em"], None).unwrap();
    Node::text_node(s.nodes["text"].clone(), empty_attrs(), t.to_string(), vec![em])
}

fn text_strong(s: &Arc<Schema>, t: &str) -> Node {
    let strong = s.marks["strong"].create(&s.marks["strong"], None).unwrap();
    Node::text_node(s.nodes["text"].clone(), empty_attrs(), t.to_string(), vec![strong])
}

fn para(s: &Arc<Schema>, children: Vec<Node>) -> Node {
    let content = Fragment::from_array(&children);
    Node::new(s.nodes["paragraph"].clone(), empty_attrs(), content, vec![])
}

fn heading(s: &Arc<Schema>, children: Vec<Node>) -> Node {
    let content = Fragment::from_array(&children);
    Node::new(s.nodes["heading"].clone(), empty_attrs(), content, vec![])
}

fn doc(s: &Arc<Schema>, children: Vec<Node>) -> Node {
    let content = Fragment::from_array(&children);
    Node::new(s.nodes["doc"].clone(), empty_attrs(), content, vec![])
}

// ---- find_diff_start tests ----

#[test]
fn diff_start_identical_returns_none() {
    let s = schema();
    let a = doc(&s, vec![para(&s, vec![text(&s, "a"), text_em(&s, "b")]), para(&s, vec![text(&s, "hello")])]);
    let b = a.clone();
    assert!(a.content.find_diff_start(&b.content, 0).is_none());
}

#[test]
fn diff_start_one_longer() {
    let s = schema();
    let shared = vec![para(&s, vec![text(&s, "a"), text_em(&s, "b")]), para(&s, vec![text(&s, "hello")])];
    let mut a_children = shared.clone();
    a_children.push(para(&s, vec![]));
    let a = doc(&s, a_children);
    let b = doc(&s, shared);
    // a is longer, diff at size of b's content
    let diff = a.content.find_diff_start(&b.content, 0);
    assert!(diff.is_some());
    assert!(diff.unwrap() > 0);
}

#[test]
fn diff_start_different_marks() {
    let s = schema();
    let a = doc(&s, vec![para(&s, vec![text(&s, "a"), text_em(&s, "b")])]);
    let b = doc(&s, vec![para(&s, vec![text(&s, "a"), text_strong(&s, "b")])]);
    let diff = a.content.find_diff_start(&b.content, 0);
    assert!(diff.is_some());
    // diff should be after "a" (size 1) inside the paragraph (offset 1 for opening token + 1 for "a")
    assert_eq!(diff.unwrap(), 2); // paragraph opens at 0, "a" at 1, diff at 2
}

#[test]
fn diff_start_longer_text() {
    let s = schema();
    let a = doc(&s, vec![para(&s, vec![text(&s, "foobar"), text_em(&s, "b")])]);
    let b = doc(&s, vec![para(&s, vec![text(&s, "foo"), text_em(&s, "b")])]);
    let diff = a.content.find_diff_start(&b.content, 0);
    assert!(diff.is_some());
    assert_eq!(diff.unwrap(), 4); // 1 (para open) + 3 (foo)
}

#[test]
fn diff_start_different_char() {
    let s = schema();
    let a = doc(&s, vec![para(&s, vec![text(&s, "foobar")])]);
    let b = doc(&s, vec![para(&s, vec![text(&s, "foocar")])]);
    let diff = a.content.find_diff_start(&b.content, 0);
    assert!(diff.is_some());
    assert_eq!(diff.unwrap(), 4); // 1 + 3 (foo)
}

#[test]
fn diff_start_different_node_type() {
    let s = schema();
    let a = doc(&s, vec![para(&s, vec![text(&s, "a")]), para(&s, vec![text(&s, "b")])]);
    let b = doc(&s, vec![para(&s, vec![text(&s, "a")]), heading(&s, vec![text(&s, "b")])]);
    let diff = a.content.find_diff_start(&b.content, 0);
    assert!(diff.is_some());
    // paragraph "a" size = 3 (open+a+close), diff at index 3
    assert_eq!(diff.unwrap(), 3);
}

// ---- find_diff_end tests ----

#[test]
fn diff_end_identical_returns_none() {
    let s = schema();
    let a = doc(&s, vec![para(&s, vec![text(&s, "a"), text_em(&s, "b")]), para(&s, vec![text(&s, "hello")])]);
    let b = a.clone();
    let diff = a.content.find_diff_end(&b.content, a.content.size, b.content.size);
    assert!(diff.is_none());
}

#[test]
fn diff_end_different_styles() {
    let s = schema();
    let a = doc(&s, vec![para(&s, vec![text(&s, "a"), text_em(&s, "b"), text(&s, "c")])]);
    let b = doc(&s, vec![para(&s, vec![text(&s, "a"), text_strong(&s, "b"), text(&s, "c")])]);
    let diff = a.content.find_diff_end(&b.content, a.content.size, b.content.size);
    assert!(diff.is_some());
    let (da, _db) = diff.unwrap();
    // "c" (size 1) matches from end, then "b" (1) differs
    // para total = 1+1+1+1+1 = 5 nodes, size = 7 (open+a+b+c+close)
    // from end: doc.size - "c" size from each = diff around position 5 or 6
    assert!(da > 0);
}

#[test]
fn diff_end_longer_text() {
    let s = schema();
    let a = doc(&s, vec![para(&s, vec![text(&s, "barfoo"), text_em(&s, "b")])]);
    let b = doc(&s, vec![para(&s, vec![text(&s, "foo"), text_em(&s, "b")])]);
    let diff = a.content.find_diff_end(&b.content, a.content.size, b.content.size);
    assert!(diff.is_some());
}

#[test]
fn diff_end_different_nodes() {
    let s = schema();
    let a = doc(&s, vec![para(&s, vec![text(&s, "a")]), para(&s, vec![text(&s, "b")])]);
    let b = doc(&s, vec![heading(&s, vec![text(&s, "a")]), para(&s, vec![text(&s, "b")])]);
    let diff = a.content.find_diff_end(&b.content, a.content.size, b.content.size);
    assert!(diff.is_some());
    let (da, _db) = diff.unwrap();
    // para("b") is same from end (size 3), diff is at para("a")/heading("a") boundary
    let para_b_size = 3usize; // open + b + close
    assert_eq!(da, a.content.size - para_b_size);
}
```

- [ ] **Step 3: 运行 diff 测试**

```bash
cargo test -p md-editor tests_diff
```

Expected: 全部通过

- [ ] **Step 4: Commit**

```bash
git add src/model/diff.rs src/model/tests_diff.rs
git commit -m "feat(model): add diff.rs (find_diff_start, find_diff_end) and tests_diff.rs"
```

---

## Task 5: NodeType/Schema stubs 补全

**Files:**
- Modify: `src/model/schema.rs`

将以下 stubs 替换为真实实现：
- `NodeType::create_stub` → `create`
- `NodeType::create_checked_stub` → `create_checked`
- `NodeType::valid_content_stub` → `valid_content`（需要 ContentMatch::match_fragment，保持 stub，重命名补注释）
- `NodeType::allows_marks_stub` → `allows_marks`
- `NodeType::allowed_marks_stub` → `allowed_marks`
- `Schema::node_stub` → `node`
- `Schema::text_stub` → `text`
- `Schema::mark_stub` → `mark`

- [ ] **Step 1: 在 schema.rs 的 NodeType impl 中替换 stubs**

找到并替换 `create_stub`：
```rust
    /// Create a `Node` of this type.
    ///
    /// Create a `Node` of this type. The given attributes are
    /// checked and defaulted (you can pass `None` to use the type's
    /// defaults entirely, if no required attributes exist). `content`
    /// may be a `Fragment` or `None`.
    pub fn create(
        self: &Arc<Self>,
        attrs: Option<&std::collections::HashMap<String, serde_json::Value>>,
        content: Option<crate::model::fragment::Fragment>,
        marks: Vec<crate::model::mark::Mark>,
    ) -> Result<crate::model::node::Node, crate::model::errors::SchemaError> {
        if self.is_text {
            return Err(crate::model::errors::SchemaError(
                "NodeType.create can't construct text nodes".to_string(),
            ));
        }
        let attrs = self.compute_attrs_for(attrs)?;
        let content = content.unwrap_or_else(crate::model::fragment::Fragment::empty);
        Ok(crate::model::node::Node::new(self.clone(), attrs, content, marks))
    }
```

找到并替换 `create_checked_stub`：
```rust
    /// Like `create`, but check the given content against the node type's content restrictions.
    ///
    /// Like `create`, but check the given content
    /// against the node type's content restrictions, and return an error
    /// if it doesn't match.
    pub fn create_checked(
        self: &Arc<Self>,
        attrs: Option<&std::collections::HashMap<String, serde_json::Value>>,
        content: Option<crate::model::fragment::Fragment>,
        marks: Vec<crate::model::mark::Mark>,
    ) -> Result<crate::model::node::Node, crate::model::errors::RangeError> {
        let attrs = self.compute_attrs_for(attrs).map_err(|e| crate::model::errors::RangeError(e.0))?;
        let content = content.unwrap_or_else(crate::model::fragment::Fragment::empty);
        // Note: full content validation (valid_content) requires match_fragment (Phase 3.4)
        // For now, create without content check; Phase 3.4 will enable the check.
        Ok(crate::model::node::Node::new(self.clone(), attrs, content, marks))
    }
```

找到并替换 `allows_marks_stub`：
```rust
    /// Test whether the given set of marks are allowed in this node.
    ///
    /// Test whether the given set of marks are allowed in this node.
    pub fn allows_marks(&self, marks: &[crate::model::mark::Mark]) -> bool {
        if self.mark_set.is_none() {
            return true;
        }
        marks.iter().all(|m| self.allows_mark_type(&m.mark_type))
    }
```

找到并替换 `allowed_marks_stub`：
```rust
    /// Removes the marks that are not allowed in this node from the given set.
    ///
    /// Removes the marks that are not allowed in this node from the given set.
    pub fn allowed_marks(&self, marks: &[crate::model::mark::Mark]) -> Vec<crate::model::mark::Mark> {
        if self.mark_set.is_none() {
            return marks.to_vec();
        }
        marks.iter().filter(|m| self.allows_mark_type(&m.mark_type)).cloned().collect()
    }
```

- [ ] **Step 2: 在 Schema impl 中替换 stubs**

找到并替换 `node_stub`：
```rust
    /// Create a node in this schema. The `type` may be a string.
    ///
    /// Create a node in this schema. The `type` may be a string or a
    /// `NodeType` instance. Attributes will be extended with defaults,
    /// `content` may be a `Fragment`, `null`, a `Node`, or an array of
    /// nodes.
    pub fn node(
        &self,
        type_name: &str,
        attrs: Option<&std::collections::HashMap<String, serde_json::Value>>,
        content: Option<crate::model::fragment::Fragment>,
        marks: Vec<crate::model::mark::Mark>,
    ) -> Result<crate::model::node::Node, crate::model::errors::SchemaError> {
        let nt = self.node_type(type_name)?;
        nt.create_checked(nt, attrs, content, marks)
            .map_err(|e| crate::model::errors::SchemaError(e.0))
    }
```

找到并替换 `text_stub`：
```rust
    /// Create a text node in the schema. Empty text nodes are not allowed.
    ///
    /// Create a text node in the schema. Empty text nodes are not allowed.
    pub fn text(
        &self,
        text: String,
        marks: Vec<crate::model::mark::Mark>,
    ) -> crate::model::node::Node {
        let text_type = self.nodes["text"].clone();
        let attrs = text_type.default_attrs.clone()
            .unwrap_or_else(|| Arc::new(Default::default()));
        crate::model::node::Node::text_node(text_type, attrs, text, marks)
    }
```

找到并替换 `mark_stub`：
```rust
    /// Create a mark with the given type and attributes.
    ///
    /// Create a mark with the given type and attributes.
    pub fn mark(
        &self,
        type_name: &str,
        attrs: Option<&std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<crate::model::mark::Mark, crate::model::errors::SchemaError> {
        let mt = self.mark_type(type_name)?;
        mt.create(mt, attrs)
    }
```

同时将 `node_from_json_stub` 和 `mark_from_json_stub` 替换：
```rust
    /// Deserialize a node from its JSON representation.
    pub fn node_from_json(
        &self,
        json: &serde_json::Value,
    ) -> Result<crate::model::node::Node, crate::model::errors::RangeError> {
        crate::model::node::Node::from_json(self, json)
    }

    /// Deserialize a mark from its JSON representation.
    pub fn mark_from_json(
        &self,
        json: &serde_json::Value,
    ) -> Result<crate::model::mark::Mark, crate::model::errors::RangeError> {
        crate::model::mark::Mark::from_json(self, json)
    }
```

- [ ] **Step 3: 运行全量测试**

```bash
cargo test -p md-editor
```

Expected: 全部通过（Phase 1-3.1 的 141 个 + Phase 3.2 新增约 20 个）

- [ ] **Step 4: Clippy**

```bash
cargo clippy -p md-editor -- -D warnings
```

修复所有 warnings。

- [ ] **Step 5: Commit**

```bash
git add src/model/schema.rs
git commit -m "feat(model): replace NodeType/Schema stubs with real implementations"
```

---

## Task 6: 全量验证 + 更新阶段计划

**Files:**
- Modify: `docs/superpowers/plans/model-phases.md`

- [ ] **Step 1: 全量测试**

```bash
cargo test -p md-editor
```

Expected: 全部通过

- [ ] **Step 2: Clippy**

```bash
cargo clippy -p md-editor -- -D warnings
```

Expected: 无 error

- [ ] **Step 3: 无 unsafe**

```bash
grep -rn "unsafe" src/model/
```

Expected: 空

- [ ] **Step 4: 更新阶段计划**

将 `docs/superpowers/plans/model-phases.md` 中 Phase 3.2 状态改为 `✅ 完成`。

- [ ] **Step 5: Commit**

```bash
git add docs/superpowers/plans/model-phases.md
git commit -m "docs: mark model Phase 3.2 as complete"
```

---

## 自检

### Spec 覆盖

| 类型/方法 | 任务 | 状态 |
|---|---|---|
| Fragment struct（Arc<[Node]>, size） | Task 1 | ✅ |
| Fragment::empty, from_array, from_node | Task 2 | ✅ |
| Fragment::append, cut, replace_child, add_to_start, add_to_end | Task 2 | ✅ |
| Fragment::eq, child_count, child, maybe_child, first/last_child | Task 2 | ✅ |
| Fragment::for_each, nodes_between, descendants, text_between | Task 2 | ✅ |
| Fragment::find_index, find_diff_start, find_diff_end | Task 2+4 | ✅ |
| Fragment::to_json, from_json | Task 2 | ✅ |
| Node struct（node_type, attrs, content, marks, text） | Task 1 | ✅ |
| Node::node_size（text/leaf/non-leaf） | Task 3 | ✅ |
| Node::eq, same_markup, has_markup | Task 3 | ✅ |
| Node::copy, with_marks, cut_inner, with_text | Task 3 | ✅ |
| Node::is_block/text/leaf/atom/inline/textblock | Task 3 | ✅ |
| Node::for_each, nodes_between, descendants | Task 3 | ✅ |
| Node::text_content, text_between | Task 3 | ✅ |
| Node::to_json, from_json | Task 3 | ✅ |
| Node::range_has_mark | Task 3 | ✅ |
| Node::node_at, child_after, child_before | Task 3 | ✅ |
| Node::content_match_at/can_replace/can_replace_with/can_append/check | Task 3 | stub |
| Node::slice/replace/resolve | Task 3 | stub |
| find_diff_start, find_diff_end | Task 4 | ✅ |
| NodeType::create, create_checked | Task 5 | ✅ |
| NodeType::allows_marks, allowed_marks | Task 5 | ✅ |
| Schema::node, text, mark, node_from_json, mark_from_json | Task 5 | ✅ |

### 注意事项

- `Fragment::cut` 需要 Node 的 `with_text`（文本切割）和 `cut_inner`（非文本节点切割）。这两个方法在 Task 3 实现，Task 2 的占位方法在 Task 3 被替换。
- `diff_start_different_marks` 测试中 diff 位置计算：paragraph 打开 token 算 1，然后 "a" 的 text 节点大小是 1（字节），所以 diff 在位置 2。
- `NodeType::create_checked` 暂不做 content 校验（需要 match_fragment，Phase 3.4 补充）。
- `Schema::node` 调用 `create_checked`，最终在 Phase 3.4 完整时会做 content 校验。
