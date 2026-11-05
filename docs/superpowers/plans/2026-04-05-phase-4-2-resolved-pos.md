# Phase 4.2: ResolvedPos struct + resolve 算法 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 创建 `src/model/resolved_pos.rs`,实现 `ResolvedPos` struct、`PathFrame` struct、`resolve_no_cache()` 路径解析算法、`resolve()` 入口(本阶段 delegate 到 resolve_no_cache)。在 `src/model/tests_resolve.rs` 写基础结构验证测试。Phase 4.3/4.4 将续写访问器方法(node/start/end/before/after/marks 等)。

**Architecture:**
- `ResolvedPos` 持有 `pos`、`path: Vec<PathFrame>`、`parent_offset`、`depth` 字段,全部 `pub(crate)` 可见(测试与后续访问器需要访问)。
- `PathFrame` 是每层路径的记录单元,包含 `{ node: Node, index: usize, start_pos: usize }`(Node by value,通过 Arc-backed Fragment 共享数据,clone 成本低)。
- `resolve_no_cache(doc: &Node, pos: usize) -> Result<ResolvedPos, RangeError>` 是核心算法:从 doc 开始循环调用 `content.find_index(parent_offset)`,下降到文本节点或命中 offset 边界为止。
- `resolve(doc: &Node, pos: usize)` 本阶段仅 delegate 到 resolve_no_cache;TS 的 `ResolvedPos.resolveCached` 使用 `WeakMap<Node, ResolveCache>` 缓存,Rust 等价实现需要 Node 使用 `Arc` 识别身份,本阶段推迟到 Phase 4.7(设计决策 32)。
- Phase 4.2 测试只验证 path 结构(depth、path.len、每层 node.type.name / index / start_pos、parent_offset),不依赖 Phase 4.3/4.4 的访问器。

**Tech Stack:** Rust, Fragment::find_index, Node::content (Fragment), serde_json::Value (无), RangeError。

---

## 文件列表

| 操作 | 文件 | 说明 |
|------|------|------|
| Create | `src/model/resolved_pos.rs` | PathFrame + ResolvedPos struct + new + resolve_depth + resolve_no_cache + resolve |
| Create | `src/model/tests_resolve.rs` | 基础结构验证测试(Phase 4.2 范围)|
| Modify | `src/model/mod.rs` | 声明 `pub mod resolved_pos` 和 `#[cfg(test)] mod tests_resolve` |

---

## 关键已有 API(可直接调用)

```rust
// Node（src/model/node.rs）
pub struct Node {
    pub node_type: Arc<NodeType>,
    pub attrs: Attrs,
    pub content: Fragment,
    pub marks: Vec<Mark>,
    pub text: Option<String>,
}
impl Node {
    pub fn is_text(&self) -> bool;
    // Node: #[derive(Clone)] — clone 成本低(Arc bump + Vec<Mark> clone)
}
impl Clone for Node { ... } // derived

// Fragment（src/model/fragment.rs）
pub struct Fragment {
    pub(crate) content: Arc<[Node]>,
    pub size: usize,
}
impl Fragment {
    pub fn child(&self, index: usize) -> Result<&Node, RangeError>;
    pub(crate) fn find_index(&self, pos: usize) -> Result<(usize, usize), RangeError>;
}

// NodeType（src/model/schema.rs）
pub struct NodeType {
    pub name: String,  // 公开字段
    // ...
}

// errors
pub struct RangeError(pub String);

// test_builder（仅 #[cfg(test)]）
pub fn test_schema() -> &'static Arc<Schema>;
pub fn doc(children: &[Node]) -> Node;
pub fn p(children: &[Node]) -> Node;
pub fn blockquote(children: &[Node]) -> Node;
pub fn t(s: &str) -> Node;  // 纯文本节点
```

---

## TS 参考算法(resolvedpos.ts:218-233)

```typescript
static resolve(doc: Node, pos: number): ResolvedPos {
  if (!(pos >= 0 && pos <= doc.content.size)) throw new RangeError("Position " + pos + " out of range")
  let path: Array<Node | number> = []
  let start = 0, parentOffset = pos
  for (let node = doc;;) {
    let {index, offset} = node.content.findIndex(parentOffset)
    let rem = parentOffset - offset
    path.push(node, index, start + offset)
    if (!rem) break
    node = node.child(index)
    if (node.isText) break
    parentOffset = rem - 1
    start += offset + 1
  }
  return new ResolvedPos(pos, path, parentOffset)
}
```

**算法要点**:
- `path` 每 3 个元素一组存 (node, index, start_pos)。Rust 用 `Vec<PathFrame>` struct。
- `start` 累加的是"当前 node 的 content 在 root 中的起始绝对位置"。第 0 层 (doc) 的 start=0。
- `start + offset` 是"该层 node 的第 index 个子节点在 root 中的起始绝对位置(child 的 open tag 前)"。
- 如果 `rem == 0` 表示 pos 正好落在某个 child 边界(非文本节点内部),停止下降。
- 如果下一层 `child.isText`,那么 path 记录的是 **text 节点的 parent** + text 的 index,不把 text 节点本身加入 path。

---

## 目标结构预览(最终 resolved_pos.rs 框架)

```rust
//! src/model/resolved_pos.rs
//! Phase 4.2 — ResolvedPos struct + resolve 算法。
//! Phase 4.3+ 将在本文件续写访问器方法(node/start/end/before/after/marks...)。

#![allow(dead_code)] // TODO: Phase 4.4 访问器完整后移除

use crate::model::errors::RangeError;
use crate::model::node::Node;

/// One level of the resolved-position path.
#[derive(Clone)]
pub struct PathFrame {
    pub node: Node,
    pub index: usize,
    pub start_pos: usize,
}

/// A resolved position in a document.
#[derive(Clone)]
pub struct ResolvedPos {
    pub pos: usize,
    pub(crate) path: Vec<PathFrame>,
    pub parent_offset: usize,
    pub depth: usize,
}

impl ResolvedPos {
    pub(crate) fn new(pos: usize, path: Vec<PathFrame>, parent_offset: usize) -> Self { /* ... */ }
    pub(crate) fn resolve_depth(&self, val: Option<i32>) -> usize { /* ... */ }
    pub fn resolve_no_cache(doc: &Node, pos: usize) -> Result<ResolvedPos, RangeError> { /* ... */ }
    pub fn resolve(doc: &Node, pos: usize) -> Result<ResolvedPos, RangeError> { /* ... */ }
}
```

---

## Task 1: 创建 resolved_pos.rs 骨架 + PathFrame + ResolvedPos struct + new + resolve_depth

**Files:**
- Create: `src/model/resolved_pos.rs`
- Create: `src/model/tests_resolve.rs`
- Modify: `src/model/mod.rs`

本任务建立模块文件,定义 struct 与基础构造。resolve 算法本身留到 Task 2。

- [ ] **Step 1: 在 mod.rs 中声明新模块**

修改 `src/model/mod.rs`,在 `pub mod replace;` 之后添加 resolved_pos 声明,并在文件末尾追加 tests_resolve 声明。完整修改后内容:

```rust
// prosemirror-model Rust 重设计
// 参见 docs/superpowers/specs/rust-prosemirror-porting-guide.md

pub mod errors;
pub mod compare_deep;
pub mod render_output;
pub mod schema;
pub mod content;
pub mod mark;
#[cfg(test)]
mod tests_mark;
pub mod fragment;
pub mod node;
pub mod diff;
pub mod replace;
pub mod resolved_pos;
#[cfg(test)]
mod tests_diff;
#[cfg(test)]
pub(crate) mod test_builder;
#[cfg(test)]
mod tests_node;
#[cfg(test)]
mod tests_content;
#[cfg(test)]
mod tests_slice;
#[cfg(test)]
mod tests_resolve;
```

- [ ] **Step 2: 创建 tests_resolve.rs 骨架 + 写第一批失败测试**

创建 `src/model/tests_resolve.rs`,写入:

```rust
// src/model/tests_resolve.rs
//
// Phase 4.2 — ResolvedPos resolve 算法基础测试。
// Phase 4.3/4.4 将在本文件补充访问器(node/start/end/before/after/marks)测试。

use crate::model::resolved_pos::{PathFrame, ResolvedPos};
use crate::model::test_builder::{doc, p, t};

#[test]
fn path_frame_can_be_constructed() {
    let d = doc(&[p(&[t("hi")])]);
    let frame = PathFrame {
        node: d.clone(),
        index: 0,
        start_pos: 0,
    };
    assert_eq!(frame.index, 0);
    assert_eq!(frame.start_pos, 0);
    assert_eq!(frame.node.node_type.name, "doc");
}

#[test]
fn resolved_pos_can_be_constructed_via_new() {
    let d = doc(&[p(&[t("x")])]);
    let path = vec![PathFrame {
        node: d.clone(),
        index: 0,
        start_pos: 0,
    }];
    let rp = ResolvedPos::new(0, path, 0);
    assert_eq!(rp.pos, 0);
    assert_eq!(rp.parent_offset, 0);
    assert_eq!(rp.depth, 0);
    assert_eq!(rp.path.len(), 1);
}

#[test]
fn resolved_pos_depth_is_path_len_minus_one() {
    let d = doc(&[p(&[t("x")])]);
    // 手工构造深度为 2 的 path (doc + blockquote + p)
    let frames = vec![
        PathFrame { node: d.clone(), index: 0, start_pos: 0 },
        PathFrame { node: d.clone(), index: 0, start_pos: 1 },
        PathFrame { node: d.clone(), index: 0, start_pos: 2 },
    ];
    let rp = ResolvedPos::new(3, frames, 0);
    assert_eq!(rp.depth, 2);
    assert_eq!(rp.path.len(), 3);
}

#[test]
fn resolve_depth_defaults_to_self_depth() {
    let d = doc(&[p(&[t("x")])]);
    let frames = vec![
        PathFrame { node: d.clone(), index: 0, start_pos: 0 },
        PathFrame { node: d.clone(), index: 0, start_pos: 1 },
    ];
    let rp = ResolvedPos::new(1, frames, 0);
    assert_eq!(rp.resolve_depth(None), 1);
}

#[test]
fn resolve_depth_accepts_positive_value() {
    let d = doc(&[p(&[t("x")])]);
    let frames = vec![
        PathFrame { node: d.clone(), index: 0, start_pos: 0 },
        PathFrame { node: d.clone(), index: 0, start_pos: 1 },
    ];
    let rp = ResolvedPos::new(1, frames, 0);
    assert_eq!(rp.resolve_depth(Some(0)), 0);
    assert_eq!(rp.resolve_depth(Some(1)), 1);
}

#[test]
fn resolve_depth_negative_wraps_from_self_depth() {
    let d = doc(&[p(&[t("x")])]);
    // depth=2 path
    let frames = vec![
        PathFrame { node: d.clone(), index: 0, start_pos: 0 },
        PathFrame { node: d.clone(), index: 0, start_pos: 1 },
        PathFrame { node: d.clone(), index: 0, start_pos: 2 },
    ];
    let rp = ResolvedPos::new(3, frames, 0);
    // self.depth = 2; -1 ⇒ 2 + (-1) = 1; -2 ⇒ 0
    assert_eq!(rp.resolve_depth(Some(-1)), 1);
    assert_eq!(rp.resolve_depth(Some(-2)), 0);
}

```

- [ ] **Step 3: 运行测试确认失败**

```bash
cd /Users/aporicho/Desktop/md-editor
cargo test --bin md-editor model::tests_resolve
```

预期:编译错误 `unresolved import 'crate::model::resolved_pos'`(因为 resolved_pos.rs 尚未创建)。

- [ ] **Step 4: 创建 resolved_pos.rs**

创建 `src/model/resolved_pos.rs`,写入:

```rust
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
}
```

- [ ] **Step 5: 运行测试确认通过**

```bash
cargo test --bin md-editor model::tests_resolve
```

预期:6 个 tests_resolve 测试通过。

```bash
cargo test --bin md-editor
```

预期:全量 301 测试通过(295 + 6)。

- [ ] **Step 6: 提交**

```bash
git add src/model/resolved_pos.rs src/model/tests_resolve.rs src/model/mod.rs
git commit -m "$(cat <<'EOF'
feat(model): Phase 4.2 — PathFrame + ResolvedPos struct

Create resolved_pos.rs with PathFrame, ResolvedPos struct,
new constructor, and resolve_depth helper (handles None/negative depth args).

Co-Authored-By: Claude Sonnet 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: 实现 resolve_no_cache 算法 + 基础结构验证测试

**Files:**
- Modify: `src/model/resolved_pos.rs`
- Modify: `src/model/tests_resolve.rs`

- [ ] **Step 1: 写失败测试**

在 `src/model/tests_resolve.rs` 顶部 use 列表追加 `blockquote`(与现有导入合并):

```rust
use crate::model::test_builder::{blockquote, doc, p, t};
```

然后在文件末尾追加:

```rust
// --- resolve_no_cache 算法测试 ---
// 测试 doc: doc(p("ab"), blockquote(p("cdef")))
//   p("ab").size = 2+2 = 4
//   blockquote(p("cdef")).size = 2+(2+4) = 8
//   doc.size = 4+8 = 12

fn test_doc() -> crate::model::node::Node {
    doc(&[p(&[t("ab")]), blockquote(&[p(&[t("cdef")])])])
}

#[test]
fn resolve_no_cache_pos_zero_stays_in_doc() {
    let d = test_doc();
    let rp = ResolvedPos::resolve_no_cache(&d, 0).unwrap();
    // pos=0 ⇒ 处于 doc 层的 offset 0,rem=0 立即 break
    assert_eq!(rp.depth, 0);
    assert_eq!(rp.parent_offset, 0);
    assert_eq!(rp.path.len(), 1);
    assert_eq!(rp.path[0].node.node_type.name, "doc");
    assert_eq!(rp.path[0].index, 0);
    assert_eq!(rp.path[0].start_pos, 0);
}

#[test]
fn resolve_no_cache_pos_one_inside_first_paragraph() {
    let d = test_doc();
    let rp = ResolvedPos::resolve_no_cache(&d, 1).unwrap();
    // pos=1 进入 p1 内部,文本 "ab" offset=0 ⇒ rem=1 hit text break
    assert_eq!(rp.depth, 1);
    assert_eq!(rp.parent_offset, 0); // "ab" 的字符 0
    assert_eq!(rp.path[0].node.node_type.name, "doc");
    assert_eq!(rp.path[0].index, 0);
    assert_eq!(rp.path[0].start_pos, 0);
    assert_eq!(rp.path[1].node.node_type.name, "paragraph");
    assert_eq!(rp.path[1].index, 0);
    assert_eq!(rp.path[1].start_pos, 1);
}

#[test]
fn resolve_no_cache_pos_three_at_p1_end_boundary() {
    let d = test_doc();
    let rp = ResolvedPos::resolve_no_cache(&d, 3).unwrap();
    // pos=3 在 p1 末尾(text "ab" 之后,p1 close 之前)
    // doc findIndex(3): index=0, offset=0, rem=3, 下降进 p1
    // p1 findIndex(2): 2==text("ab").size=2 ⇒ (index=1, offset=2), rem=0 break
    assert_eq!(rp.depth, 1);
    assert_eq!(rp.parent_offset, 2);
    assert_eq!(rp.path[0].node.node_type.name, "doc");
    assert_eq!(rp.path[0].index, 0);
    assert_eq!(rp.path[1].node.node_type.name, "paragraph");
    assert_eq!(rp.path[1].index, 1); // 在 text "ab" 之后
    // start_pos = start(1) + offset(2) = 3(p1 末尾的绝对位置)
    assert_eq!(rp.path[1].start_pos, 3);
}

#[test]
fn resolve_no_cache_pos_four_between_blocks() {
    let d = test_doc();
    let rp = ResolvedPos::resolve_no_cache(&d, 4).unwrap();
    // pos=4 在 p1 和 blockquote 之间
    // doc findIndex(4): p1.size=4, 4==4 ⇒ (index=1, offset=4), rem=0 break
    assert_eq!(rp.depth, 0);
    assert_eq!(rp.parent_offset, 4);
    assert_eq!(rp.path[0].node.node_type.name, "doc");
    assert_eq!(rp.path[0].index, 1);
    assert_eq!(rp.path[0].start_pos, 4);
}

#[test]
fn resolve_no_cache_pos_seven_deep_in_nested_paragraph() {
    let d = test_doc();
    let rp = ResolvedPos::resolve_no_cache(&d, 7).unwrap();
    // pos=7 在 blockquote > p2 > text "cdef" 的字符 1(即 "c" 后 "d" 前)
    // doc.findIndex(7): blockquote@offset 4..12 ⇒ (1, 4), rem=3
    // 下降到 blk, parentOffset=2, start=5
    // blk.findIndex(2): p2@offset 0..6, 2<6 ⇒ (0, 0), rem=2
    // 下降到 p2, parentOffset=1, start=6
    // p2.findIndex(1): text("cdef")@size 4, 1<4 ⇒ (0, 0), rem=1
    // 下降到 text("cdef"), isText ⇒ break
    assert_eq!(rp.depth, 2);
    assert_eq!(rp.parent_offset, 1);
    assert_eq!(rp.path[0].node.node_type.name, "doc");
    assert_eq!(rp.path[0].index, 1);
    assert_eq!(rp.path[0].start_pos, 4);
    assert_eq!(rp.path[1].node.node_type.name, "blockquote");
    assert_eq!(rp.path[1].index, 0);
    assert_eq!(rp.path[1].start_pos, 5);
    assert_eq!(rp.path[2].node.node_type.name, "paragraph");
    assert_eq!(rp.path[2].index, 0);
    assert_eq!(rp.path[2].start_pos, 6);
}

#[test]
fn resolve_no_cache_pos_eleven_at_p2_end() {
    let d = test_doc();
    let rp = ResolvedPos::resolve_no_cache(&d, 11).unwrap();
    // pos=11 在 p2 末尾(text 之后,p2 close 之前)
    // doc.findIndex(11): blockquote@offset 4..12, 11<12 ⇒ (1, 4), rem=7
    // 下降到 blk, parentOffset=6, start=5
    // blk.findIndex(6): p2.size=6, 6==6 ⇒ (1, 6), rem=0 break
    assert_eq!(rp.depth, 1);
    assert_eq!(rp.parent_offset, 6);
    assert_eq!(rp.path[0].node.node_type.name, "doc");
    assert_eq!(rp.path[0].index, 1);
    assert_eq!(rp.path[0].start_pos, 4);
    assert_eq!(rp.path[1].node.node_type.name, "blockquote");
    assert_eq!(rp.path[1].index, 1); // 在 p2 之后
    // start_pos = start(5) + offset(6) = 11(blk 内容末尾的绝对位置)
    assert_eq!(rp.path[1].start_pos, 11);
}

#[test]
fn resolve_no_cache_pos_twelve_at_doc_end() {
    let d = test_doc();
    let rp = ResolvedPos::resolve_no_cache(&d, 12).unwrap();
    // pos=12 在 doc 末尾
    // doc.findIndex(12): doc.size=12, 12==12 ⇒ (2, 12), rem=0 break
    assert_eq!(rp.depth, 0);
    assert_eq!(rp.parent_offset, 12);
    assert_eq!(rp.path[0].node.node_type.name, "doc");
    assert_eq!(rp.path[0].index, 2); // 在 blockquote 之后
    assert_eq!(rp.path[0].start_pos, 12);
}
```

- [ ] **Step 2: 运行测试确认失败**

```bash
cargo test --bin md-editor model::tests_resolve::resolve_no_cache
```

预期:编译错误 `no function or associated item named 'resolve_no_cache' found for struct 'ResolvedPos'`。

- [ ] **Step 3: 实现 resolve_no_cache**

在 `src/model/resolved_pos.rs` 的 `impl ResolvedPos` 块末尾(`resolve_depth` 之后)添加:

```rust
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
```

- [ ] **Step 4: 运行测试确认通过**

```bash
cargo test --bin md-editor model::tests_resolve
```

预期:13 个 tests_resolve 测试通过(6 + 7)。

```bash
cargo test --bin md-editor
```

预期:全量 308 测试通过(295 + 13)。

- [ ] **Step 5: 提交**

```bash
git add src/model/resolved_pos.rs src/model/tests_resolve.rs
git commit -m "$(cat <<'EOF'
feat(model): Phase 4.2 — ResolvedPos::resolve_no_cache algorithm

Implement position resolution by descending from doc via
Fragment::find_index, accumulating (node, index, start_pos)
frames at each level until reaching a text node or child boundary.

Co-Authored-By: Claude Sonnet 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: 实现 resolve 入口 + out-of-range 错误测试 + 最终验证

**Files:**
- Modify: `src/model/resolved_pos.rs`
- Modify: `src/model/tests_resolve.rs`

- [ ] **Step 1: 写失败测试**

在 `src/model/tests_resolve.rs` 末尾追加:

```rust
#[test]
fn resolve_out_of_range_errors() {
    let d = test_doc();
    // doc.content.size = 12,超过 12 应报错
    let err = ResolvedPos::resolve(&d, 13).err().unwrap();
    assert!(err.0.contains("Position 13 out of range"));
}

#[test]
fn resolve_pos_at_exact_size_is_valid() {
    let d = test_doc();
    // pos == doc.content.size 是合法的(在 doc 末尾)
    let rp = ResolvedPos::resolve(&d, 12).unwrap();
    assert_eq!(rp.pos, 12);
    assert_eq!(rp.depth, 0);
}

#[test]
fn resolve_delegates_to_resolve_no_cache() {
    let d = test_doc();
    // resolve 应返回与 resolve_no_cache 结构相同的 ResolvedPos
    let r_cached = ResolvedPos::resolve(&d, 7).unwrap();
    let r_nocache = ResolvedPos::resolve_no_cache(&d, 7).unwrap();
    assert_eq!(r_cached.pos, r_nocache.pos);
    assert_eq!(r_cached.depth, r_nocache.depth);
    assert_eq!(r_cached.parent_offset, r_nocache.parent_offset);
    assert_eq!(r_cached.path.len(), r_nocache.path.len());
    for i in 0..r_cached.path.len() {
        assert_eq!(r_cached.path[i].node.node_type.name, r_nocache.path[i].node.node_type.name);
        assert_eq!(r_cached.path[i].index, r_nocache.path[i].index);
        assert_eq!(r_cached.path[i].start_pos, r_nocache.path[i].start_pos);
    }
}

#[test]
fn resolve_in_empty_doc() {
    // doc 内部必须至少有一个 block(schema 要求 block+),用最小的 doc
    let d = doc(&[p(&[])]);
    // p() 空段落 size = 2,doc.size = 2
    // pos=0: 在 doc 层 offset 0,rem=0 break ⇒ depth=0
    let rp = ResolvedPos::resolve(&d, 0).unwrap();
    assert_eq!(rp.depth, 0);
    assert_eq!(rp.parent_offset, 0);
    // pos=1: 进入 p,parentOffset=0,p.content.size=0 ⇒ find_index(0)=(0,0), rem=0 break
    let rp = ResolvedPos::resolve(&d, 1).unwrap();
    assert_eq!(rp.depth, 1);
    assert_eq!(rp.parent_offset, 0);
    assert_eq!(rp.path[1].node.node_type.name, "paragraph");
    // pos=2: 在 doc 末尾
    let rp = ResolvedPos::resolve(&d, 2).unwrap();
    assert_eq!(rp.depth, 0);
    assert_eq!(rp.parent_offset, 2);
}
```

- [ ] **Step 2: 运行测试确认失败**

```bash
cargo test --bin md-editor model::tests_resolve::resolve_out_of_range
```

预期:编译错误 `no function or associated item named 'resolve' found for struct 'ResolvedPos'`。

- [ ] **Step 3: 实现 resolve**

在 `src/model/resolved_pos.rs` 的 `impl ResolvedPos` 块末尾(`resolve_no_cache` 之后)添加:

```rust
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
```

- [ ] **Step 4: 运行测试确认通过**

```bash
cargo test --bin md-editor model::tests_resolve
```

预期:17 个 tests_resolve 测试通过(13 + 4)。

```bash
cargo test --bin md-editor
```

预期:全量 312 测试通过(295 + 17)。

- [ ] **Step 5: 运行 clippy 验证**

```bash
cargo clippy --bin md-editor --tests 2>&1 | tail -30
```

预期:**resolved_pos.rs 和 tests_resolve.rs 两个新文件零 warning**。既有 warnings(src/model/content.rs、src/model/node.rs 等已存在的)继续存在,不在本 Task 修复范围。

如果新文件有 warning,修复之。

- [ ] **Step 6: 更新 model-phases.md**

修改 `docs/superpowers/plans/model-phases.md`。找到这行:

```markdown
| Phase 4.2 | ResolvedPos struct + resolve 算法 | resolved_pos.rs（struct + resolve + resolve_no_cache）+ 基础测试 | Phase 3.4 | ⬜ 待开始 |
```

改为:

```markdown
| Phase 4.2 | ResolvedPos struct + resolve 算法 | resolved_pos.rs（struct + resolve + resolve_no_cache）+ 基础测试 | Phase 3.4 | ✅ 完成 |
```

- [ ] **Step 7: 提交**

```bash
git add src/model/resolved_pos.rs src/model/tests_resolve.rs docs/superpowers/plans/model-phases.md
git commit -m "$(cat <<'EOF'
feat(model): Phase 4.2 — ResolvedPos::resolve + mark complete

Add resolve() entry point (delegates to resolve_no_cache for now; 
identity-based caching deferred to Phase 4.7+). Add out-of-range 
error tests, empty-doc tests, and delegate-equivalence test.

Co-Authored-By: Claude Sonnet 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## 完成标准

- [ ] `src/model/resolved_pos.rs` 存在,含 PathFrame + ResolvedPos struct + new + resolve_depth + resolve_no_cache + resolve
- [ ] `src/model/tests_resolve.rs` 有 17 个测试全部通过
- [ ] `cargo test --bin md-editor` 全量 312 测试通过(295 + 17)
- [ ] resolved_pos.rs 和 tests_resolve.rs 零 clippy warnings
- [ ] `docs/superpowers/plans/model-phases.md` 更新 Phase 4.2 状态为 ✅

---

## 设计决策记录

### 决策 32:resolve() 本阶段暂不实现 identity cache

TS 的 `ResolvedPos.resolveCached` 使用 `WeakMap<Node, ResolveCache>` 基于 doc 节点身份缓存 ResolvedPos 对象(缓存 12 个 pos)。Rust 当前 `Node` 是 `#[derive(Clone)]` by-value 类型,clone 时产生新实例,无法直接用作 WeakMap key。实现等效缓存需要将 Node 改造为 `Arc<Node>` 或在 Schema 层面重构所有权,这超出 Phase 4.2 范围。

**本阶段方案**:`resolve(doc, pos)` 直接 delegate 到 `resolve_no_cache(doc, pos)`,API 保持一致。Phase 4.7+ 引入 replace 算法(大量调用 resolve)时再评估缓存收益,并考虑 `Arc<Node>` 改造。

### 决策 33:PathFrame 使用 struct 而非 tuple 或三并行 vec

TS 用 `path: any[]` 按每 3 元素一组(node, index, start_pos)存储。Rust 可选方案:
- **tuple** `(Node, usize, usize)`:紧凑但访问需要 `.0/.1/.2`,可读性差。
- **三并行 vec**:`nodes: Vec<Node>, indices: Vec<usize>, starts: Vec<usize>`:需维护一致性,易出 bug。
- **struct** `PathFrame { node, index, start_pos }`:字段命名自文档,类型安全。✓ **采纳**。

### 决策 34:ResolvedPos 字段 pub / pub(crate) 混合策略

- `pos` / `parent_offset` / `depth`:`pub` 公开(TS 的 `readonly` 等价,用户代码需要读取)。
- `path`:`pub(crate)`(TS 标注 `@internal`,只有后续访问器实现需要直接访问)。
- `PathFrame` struct 和字段全 `pub`(测试 + Phase 4.3 访问器都需要)。

### 决策 35:resolve_depth 使用 Option<i32> 表示"默认 / 绝对 / 相对"三种参数形态

TS `resolveDepth(val?: number | null)` 在一个 number 参数里编码三种语义(undefined=默认,负数=相对,非负=绝对)。Rust 对应:`Option<i32>`:`None` → 默认,`Some(n)` 按 TS 语义处理。后续访问器(`node(depth)`, `start(depth)` 等)参数统一 `depth: Option<i32>`,调用链保持一致。

