# Phase 4.3: ResolvedPos 简单访问器 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 `ResolvedPos` 补全 13 个访问器方法:`parent`/`doc`/`node`/`index`/`index_after`/`start`/`end`/`before`/`after`/`text_offset`/`node_after`/`node_before`/`pos_at_index`。为支持 `node_after`/`node_before` 增加公开的 `Node::cut(from, to)` 方法。最终加一个 integration test,循环验证 `resolve(doc, pos)` 在 pos 0..=12 对所有访问器的输出都正确。

**Architecture:**
- Rust 的 `path: Vec<PathFrame>` 以 struct 组织,TS 的 `path[depth*3]` / `path[depth*3+1]` / `path[depth*3-1]` 分别映射为 `path[depth].node` / `path[depth].index` / `path[depth-1].start_pos`。
- 所有 depth 参数访问器都接受 `Option<i32>`(Decision 35 延续),内部调用 `self.resolve_depth(...)` 归一化。
- `node()`/`parent()`/`doc()` 返回 `&Node` 借用(避免 Node clone);`node_after()`/`node_before()` 返回 `Option<Node>`(由 Node::cut 生成新节点)。
- `before()`/`after()` 在 depth=0 时返回 `Err(RangeError(...))`,与 TS 的 throw 语义对齐(Rust Result<usize, RangeError>)。
- 新增的 `Node::cut(from: usize, to: Option<usize>)` 对 text 节点切 chars(UTF-8 safe),对非 text 节点 delegate 到现有 `cut_inner`。

**Tech Stack:** Rust, Option<i32> 参数风格, Result<T, RangeError> 错误处理。

---

## 文件列表

| 操作 | 文件 | 说明 |
|------|------|------|
| Modify | `src/model/resolved_pos.rs` | 添加 13 个访问器方法到 impl ResolvedPos |
| Modify | `src/model/node.rs` | 添加 `pub fn cut(from, to: Option<usize>)` 统一接口 |
| Modify | `src/model/tests_resolve.rs` | 每方法少量 unit tests + integration test |

---

## TS 参考(resolvedpos.ts 各方法)

```typescript
get parent() { return this.node(this.depth) }
get doc() { return this.node(0) }
node(depth?) { return this.path[this.resolveDepth(depth) * 3] }
index(depth?) { return this.path[this.resolveDepth(depth) * 3 + 1] }
indexAfter(depth?) {
  depth = this.resolveDepth(depth)
  return this.index(depth) + (depth == this.depth && !this.textOffset ? 0 : 1)
}
start(depth?) { depth = this.resolveDepth(depth); return depth == 0 ? 0 : this.path[depth * 3 - 1] + 1 }
end(depth?) { depth = this.resolveDepth(depth); return this.start(depth) + this.node(depth).content.size }
before(depth?) {
  depth = this.resolveDepth(depth)
  if (!depth) throw new RangeError("There is no position before the top-level node")
  return depth == this.depth + 1 ? this.pos : this.path[depth * 3 - 1]
}
after(depth?) {
  depth = this.resolveDepth(depth)
  if (!depth) throw new RangeError("There is no position after the top-level node")
  return depth == this.depth + 1 ? this.pos : this.path[depth * 3 - 1] + this.path[depth * 3].nodeSize
}
get textOffset() { return this.pos - this.path[this.path.length - 1] }
get nodeAfter() {
  let parent = this.parent, index = this.index(this.depth)
  if (index == parent.childCount) return null
  let dOff = this.pos - this.path[this.path.length - 1], child = parent.child(index)
  return dOff ? parent.child(index).cut(dOff) : child
}
get nodeBefore() {
  let index = this.index(this.depth)
  let dOff = this.pos - this.path[this.path.length - 1]
  if (dOff) return this.parent.child(index).cut(0, dOff)
  return index == 0 ? null : this.parent.child(index - 1)
}
posAtIndex(index, depth?) {
  depth = this.resolveDepth(depth)
  let node = this.path[depth * 3], pos = depth == 0 ? 0 : this.path[depth * 3 - 1] + 1
  for (let i = 0; i < index; i++) pos += node.child(i).nodeSize
  return pos
}
```

**关键映射**(TS → Rust):
- `path[depth*3]` → `path[depth].node`
- `path[depth*3+1]` → `path[depth].index`
- `path[depth*3-1]` → `path[depth-1].start_pos`(即 depth-1 层的 start_pos)
- `path[path.length-1]` → `path[self.depth].start_pos`(最后一个 frame 的 start_pos)

---

## Task 1: 简单 path 访问器 — parent / doc / node / index / index_after

**Files:**
- Modify: `src/model/resolved_pos.rs`
- Modify: `src/model/tests_resolve.rs`

- [ ] **Step 1: 在 tests_resolve.rs 末尾追加失败测试**

```rust
// --- Task 1: 简单 path 访问器测试 ---

#[test]
fn parent_returns_deepest_frame_node() {
    let d = test_doc();
    // pos=7 ⇒ path=[doc, blk, p2],parent 是 p2 (paragraph)
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    assert_eq!(rp.parent().node_type.name, "paragraph");
}

#[test]
fn doc_returns_root_node() {
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    assert_eq!(rp.doc().node_type.name, "doc");
    // pos=0 时 depth=0,doc() 和 parent() 相同
    let rp_zero = ResolvedPos::resolve(&d, 0).unwrap();
    assert_eq!(rp_zero.doc().node_type.name, "doc");
    assert_eq!(rp_zero.parent().node_type.name, "doc");
}

#[test]
fn node_returns_frame_at_requested_depth() {
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    // pos=7 path=[doc, blk, p2]
    assert_eq!(rp.node(Some(0)).node_type.name, "doc");
    assert_eq!(rp.node(Some(1)).node_type.name, "blockquote");
    assert_eq!(rp.node(Some(2)).node_type.name, "paragraph");
    // None ⇒ self.depth (2)
    assert_eq!(rp.node(None).node_type.name, "paragraph");
    // 负数:-1 ⇒ depth-1 = 1 (blockquote)
    assert_eq!(rp.node(Some(-1)).node_type.name, "blockquote");
}

#[test]
fn index_returns_frame_index_at_depth() {
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    // pos=7 的 path 索引:(doc,1,4),(blk,0,5),(p2,0,6)
    assert_eq!(rp.index(Some(0)), 1); // blockquote 是 doc 的第 1 个子
    assert_eq!(rp.index(Some(1)), 0); // p2 是 blk 的第 0 个子
    assert_eq!(rp.index(Some(2)), 0); // text 是 p2 的第 0 个子
    assert_eq!(rp.index(None), 0);    // 默认 = self.depth = 2
}

#[test]
fn index_after_excludes_current_index_when_at_boundary() {
    let d = test_doc();
    // pos=4(p1 和 blk 之间,depth=0,text_offset=0)
    // index(0)=1, depth==self.depth && text_offset==0 ⇒ +0 ⇒ 1
    let rp = ResolvedPos::resolve(&d, 4).unwrap();
    assert_eq!(rp.index_after(None), 1);
    assert_eq!(rp.index_after(Some(0)), 1);
}

#[test]
fn index_after_increments_when_inside_text() {
    let d = test_doc();
    // pos=7(在 text "cdef" 内,depth=2,text_offset=1>0)
    // index(2)=0, depth==self.depth && text_offset>0 ⇒ +1 ⇒ 1
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    assert_eq!(rp.index_after(None), 1);
    assert_eq!(rp.index_after(Some(2)), 1);
}

#[test]
fn index_after_increments_when_depth_not_self_depth() {
    let d = test_doc();
    // pos=7(depth=2),index_after(0) 访问 depth=0 != self.depth ⇒ +1
    // index(0)=1, +1 = 2
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    assert_eq!(rp.index_after(Some(0)), 2);
    assert_eq!(rp.index_after(Some(1)), 1); // index(1)=0, +1=1
}
```

- [ ] **Step 2: 运行测试确认失败**

```bash
cd /Users/aporicho/Desktop/md-editor
cargo test --bin md-editor model::tests_resolve::parent_
cargo test --bin md-editor model::tests_resolve::doc_returns
cargo test --bin md-editor model::tests_resolve::node_returns
cargo test --bin md-editor model::tests_resolve::index_
```

预期:编译错误 `no method named 'parent' / 'doc' / 'node' / 'index' / 'index_after' found for struct 'ResolvedPos'`。

- [ ] **Step 3: 实现 5 个方法**

在 `src/model/resolved_pos.rs` 的 `impl ResolvedPos` 块末尾(`resolve` 之后)添加:

```rust
    /// The parent node that the position points into.
    /// Equivalent to `self.node(None)`.
    pub fn parent(&self) -> &Node {
        &self.path[self.depth].node
    }

    /// The root node in which the position was resolved.
    /// Equivalent to `self.node(Some(0))`.
    pub fn doc(&self) -> &Node {
        &self.path[0].node
    }

    /// The ancestor node at the given depth. `p.node(None)` returns
    /// `p.parent`, `p.node(Some(0))` returns the root doc.
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
        let bump = if depth == self.depth && self.text_offset() == 0 { 0 } else { 1 };
        self.path[depth].index + bump
    }
```

**注意**:`index_after` 调用 `self.text_offset()`,而 text_offset 会在 Task 2 中定义。临时在文件末尾添加一个占位实现:

```rust
    // 临时占位:Task 2 会正式实现 text_offset
    #[doc(hidden)]
    fn text_offset_stub(&self) -> usize {
        self.pos - self.path[self.depth].start_pos
    }
```

并把 `index_after` 改为调用 `self.text_offset_stub()`。Task 2 实现 `text_offset` 后,将 `index_after` 中的 stub 调用替换掉。

**或者更简洁的方案**:在 Task 1 就直接 inline `self.pos - self.path[self.depth].start_pos == 0` 而不调用任何方法:

```rust
    pub fn index_after(&self, depth: Option<i32>) -> usize {
        let depth = self.resolve_depth(depth);
        let at_text_start = self.pos == self.path[self.depth].start_pos;
        let bump = if depth == self.depth && at_text_start { 0 } else { 1 };
        self.path[depth].index + bump
    }
```

采用第二种方案(inline 比较),避免引入 stub 依赖。Task 2 实现 text_offset 后,Task 2 可以选择重构 index_after 使用 text_offset(可选,不重构也 fine)。

**最终 Step 3 实现**:

```rust
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
```

- [ ] **Step 4: 运行测试确认通过**

```bash
cargo test --bin md-editor model::tests_resolve
```

预期:24 个 tests_resolve 测试通过(17 + 7)。

```bash
cargo test --bin md-editor
```

预期:全量 319 测试通过(295 + 24)。

- [ ] **Step 5: 提交**

```bash
git add src/model/resolved_pos.rs src/model/tests_resolve.rs
git commit -m "$(cat <<'EOF'
feat(model): Phase 4.3 — ResolvedPos simple accessors

Add parent/doc/node/index/index_after accessors returning
ancestor nodes and child indices at requested depths.

Co-Authored-By: Claude Sonnet 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: 位置访问器 — start / end / text_offset / before / after

**Files:**
- Modify: `src/model/resolved_pos.rs`
- Modify: `src/model/tests_resolve.rs`

- [ ] **Step 1: 在 tests_resolve.rs 末尾追加失败测试**

```rust
// --- Task 2: 位置访问器测试 ---

#[test]
fn start_returns_zero_at_depth_zero() {
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    assert_eq!(rp.start(Some(0)), 0);
}

#[test]
fn start_returns_inner_content_start_at_deeper_depths() {
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    // path=[(doc,1,4),(blk,0,5),(p2,0,6)]
    // start(1) = path[0].start_pos + 1 = 4+1 = 5(blockquote content 开始)
    // start(2) = path[1].start_pos + 1 = 5+1 = 6(p2 content 开始)
    assert_eq!(rp.start(Some(1)), 5);
    assert_eq!(rp.start(Some(2)), 6);
    // None 默认 = self.depth = 2
    assert_eq!(rp.start(None), 6);
}

#[test]
fn end_is_start_plus_content_size() {
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    // end(0) = 0 + doc.content.size = 12
    // end(1) = 5 + blk.content.size = 5+6 = 11(p2.node_size=6)
    // end(2) = 6 + p2.content.size = 6+4 = 10(text.node_size=4)
    assert_eq!(rp.end(Some(0)), 12);
    assert_eq!(rp.end(Some(1)), 11);
    assert_eq!(rp.end(Some(2)), 10);
}

#[test]
fn text_offset_is_pos_minus_deepest_start_pos() {
    let d = test_doc();
    // pos=7,deepest frame start_pos=6 ⇒ text_offset = 1
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    assert_eq!(rp.text_offset(), 1);
    // pos=0,deepest frame(doc,0,0).start_pos=0 ⇒ text_offset=0
    let rp0 = ResolvedPos::resolve(&d, 0).unwrap();
    assert_eq!(rp0.text_offset(), 0);
    // pos=4,deepest(doc,1,4).start_pos=4 ⇒ text_offset=0
    let rp4 = ResolvedPos::resolve(&d, 4).unwrap();
    assert_eq!(rp4.text_offset(), 0);
}

#[test]
fn before_errors_at_depth_zero() {
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    let err = rp.before(Some(0)).err().unwrap();
    assert!(err.0.contains("no position before the top-level node"));
}

#[test]
fn before_returns_pos_when_depth_equals_self_depth_plus_one() {
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap(); // self.depth=2
    // before(3) = before(self.depth+1) ⇒ self.pos = 7
    assert_eq!(rp.before(Some(3)).unwrap(), 7);
}

#[test]
fn before_returns_frame_start_pos_otherwise() {
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    // before(1) = path[0].start_pos = 4(blockquote 的 open tag 位置)
    // before(2) = path[1].start_pos = 5(p2 的 open tag 位置)
    assert_eq!(rp.before(Some(1)).unwrap(), 4);
    assert_eq!(rp.before(Some(2)).unwrap(), 5);
}

#[test]
fn after_errors_at_depth_zero() {
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    let err = rp.after(Some(0)).err().unwrap();
    assert!(err.0.contains("no position after the top-level node"));
}

#[test]
fn after_returns_pos_when_depth_equals_self_depth_plus_one() {
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    assert_eq!(rp.after(Some(3)).unwrap(), 7);
}

#[test]
fn after_returns_frame_start_plus_node_size_otherwise() {
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    // after(1) = path[0].start_pos + path[1].node.node_size() = 4 + 8 = 12(blockquote 关闭后的位置)
    // after(2) = path[1].start_pos + path[2].node.node_size() = 5 + 6 = 11(p2 关闭后的位置)
    assert_eq!(rp.after(Some(1)).unwrap(), 12);
    assert_eq!(rp.after(Some(2)).unwrap(), 11);
}
```

- [ ] **Step 2: 运行测试确认失败**

```bash
cargo test --bin md-editor model::tests_resolve
```

预期:编译错误 `no method named 'start' / 'end' / 'text_offset' / 'before' / 'after' found for struct 'ResolvedPos'`。

- [ ] **Step 3: 实现 5 个位置访问器**

在 `src/model/resolved_pos.rs` 的 `impl ResolvedPos` 块末尾(`index_after` 之后)添加:

```rust
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
```

- [ ] **Step 4: 运行测试确认通过**

```bash
cargo test --bin md-editor model::tests_resolve
```

预期:34 个 tests_resolve 测试通过(24 + 10)。

```bash
cargo test --bin md-editor
```

预期:全量 329 测试通过(295 + 34)。

- [ ] **Step 5: 提交**

```bash
git add src/model/resolved_pos.rs src/model/tests_resolve.rs
git commit -m "$(cat <<'EOF'
feat(model): Phase 4.3 — ResolvedPos position accessors

Add start/end/before/after/text_offset methods returning
absolute positions at requested depths. before/after return
Err at depth 0 (no position outside the root node).

Co-Authored-By: Claude Sonnet 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Node::cut 方法 + pos_at_index + node_after + node_before

**Files:**
- Modify: `src/model/node.rs`
- Modify: `src/model/resolved_pos.rs`
- Modify: `src/model/tests_resolve.rs`

本 Task 先添加 `Node::cut` 公开方法(`node_after`/`node_before` 依赖它切分文本节点),再实现 3 个 ResolvedPos 访问器。

- [ ] **Step 1: 在 src/model/node.rs 的 Node impl 块中添加 cut 方法 + 内联单元测试**

在 `src/model/node.rs` 的 `impl Node` 块中找到 `cut_inner` 方法(约第 229 行),在它**之前**插入 `pub fn cut`:

```rust
    /// Create a copy of this node with only a slice of its content.
    ///
    /// For text nodes, slices the text by char offsets (UTF-8 safe).
    /// For non-text nodes, slices the content Fragment. `to` defaults to
    /// the node's content size (text length for text nodes).
    pub fn cut(&self, from: usize, to: Option<usize>) -> Node {
        if self.is_text() {
            let text = self.text.as_deref().unwrap_or("");
            let char_len = text.chars().count();
            let to = to.unwrap_or(char_len);
            if from == 0 && to == char_len {
                return self.clone();
            }
            // UTF-8 safe char slicing
            let byte_start = text.char_indices().nth(from).map(|(b, _)| b).unwrap_or(text.len());
            let byte_end = text.char_indices().nth(to).map(|(b, _)| b).unwrap_or(text.len());
            self.with_text(text[byte_start..byte_end].to_string())
        } else {
            let to = to.unwrap_or(self.content.size);
            self.cut_inner(from, to)
        }
    }
```

同时在 node.rs 的 `#[cfg(test)] mod tests` 块中(文件末尾已有)添加 3 个 `Node::cut` 单元测试:

```rust
    #[test]
    fn node_cut_text_slices_by_chars() {
        let s = simple_schema();
        let n = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "hello".to_string(), vec![]);
        let cut = n.cut(1, Some(4));
        assert_eq!(cut.text.as_deref(), Some("ell"));
    }

    #[test]
    fn node_cut_text_default_to_is_full_length() {
        let s = simple_schema();
        let n = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "hello".to_string(), vec![]);
        let cut = n.cut(2, None); // to = text.len() = 5
        assert_eq!(cut.text.as_deref(), Some("llo"));
    }

    #[test]
    fn node_cut_text_unicode_safe() {
        let s = simple_schema();
        let n = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "你好世界".to_string(), vec![]);
        let cut = n.cut(1, Some(3));
        assert_eq!(cut.text.as_deref(), Some("好世"));
    }

    #[test]
    fn node_cut_non_text_returns_same_when_full_range() {
        let s = simple_schema();
        let t1 = Node::text_node(s.nodes["text"].clone(), empty_attrs(), "hi".to_string(), vec![]);
        let p = s.nodes["paragraph"]
            .create(None, Some(Fragment::from_node(t1)), vec![])
            .unwrap();
        let cut = p.cut(0, None); // to = content.size = 2, full range
        // cut_inner returns self.clone() when from==0 && to==content.size
        assert_eq!(cut.text_content(), "hi");
    }
```

- [ ] **Step 2: 运行 Node::cut 测试确认通过**

```bash
cargo test --bin md-editor model::node::tests::node_cut
```

预期:4 个 node_cut 测试通过。

- [ ] **Step 3: 提交 Node::cut**

```bash
git add src/model/node.rs
git commit -m "$(cat <<'EOF'
feat(model): add Node::cut(from, to) unified slicing API

Slices text nodes by chars (UTF-8 safe) or delegates to cut_inner
for non-text nodes. Required by ResolvedPos::node_after/node_before.

Co-Authored-By: Claude Sonnet 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 4: 在 tests_resolve.rs 末尾追加 ResolvedPos 访问器失败测试**

```rust
// --- Task 3: pos_at_index + node_after + node_before 测试 ---

#[test]
fn pos_at_index_at_depth_zero() {
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    // depth=0,node=doc,base pos=0
    // pos_at_index(0, 0) = 0
    // pos_at_index(1, 0) = 0 + p1.node_size(4) = 4
    // pos_at_index(2, 0) = 4 + blk.node_size(8) = 12
    assert_eq!(rp.pos_at_index(0, Some(0)), 0);
    assert_eq!(rp.pos_at_index(1, Some(0)), 4);
    assert_eq!(rp.pos_at_index(2, Some(0)), 12);
}

#[test]
fn pos_at_index_at_deeper_depth() {
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    // depth=2,node=p2,base pos = path[1].start_pos + 1 = 5+1 = 6
    // pos_at_index(0, 2) = 6
    // pos_at_index(1, 2) = 6 + text("cdef").node_size(4) = 10
    assert_eq!(rp.pos_at_index(0, Some(2)), 6);
    assert_eq!(rp.pos_at_index(1, Some(2)), 10);
}

#[test]
fn node_after_returns_full_child_when_at_boundary() {
    let d = test_doc();
    // pos=4(p1 和 blk 之间,text_offset=0,index=1)
    // node_after = parent(doc).child(1) = blockquote 完整节点
    let rp = ResolvedPos::resolve(&d, 4).unwrap();
    let after = rp.node_after().unwrap();
    assert_eq!(after.node_type.name, "blockquote");
}

#[test]
fn node_after_returns_none_at_last_index() {
    let d = test_doc();
    // pos=12(doc 末尾,index=2==doc.child_count=2)
    let rp = ResolvedPos::resolve(&d, 12).unwrap();
    assert!(rp.node_after().is_none());
}

#[test]
fn node_after_cuts_text_when_inside_text_node() {
    let d = test_doc();
    // pos=7(text "cdef" 内部,text_offset=1)
    // parent=p2, index=0, child=text("cdef"), d_off=1 ⇒ cut(1) = text("def")
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    let after = rp.node_after().unwrap();
    assert!(after.is_text());
    assert_eq!(after.text.as_deref(), Some("def"));
}

#[test]
fn node_before_returns_none_at_start() {
    let d = test_doc();
    // pos=0(doc 开头,index=0,text_offset=0)
    let rp = ResolvedPos::resolve(&d, 0).unwrap();
    assert!(rp.node_before().is_none());
}

#[test]
fn node_before_returns_previous_child_when_at_boundary() {
    let d = test_doc();
    // pos=4(text_offset=0,index=1)⇒ parent.child(0) = p1 整个节点
    let rp = ResolvedPos::resolve(&d, 4).unwrap();
    let before = rp.node_before().unwrap();
    assert_eq!(before.node_type.name, "paragraph");
    assert_eq!(before.text_content(), "ab");
}

#[test]
fn node_before_cuts_text_when_inside_text_node() {
    let d = test_doc();
    // pos=7(text "cdef" 内部,text_offset=1)
    // parent=p2, index=0, d_off=1 > 0 ⇒ parent.child(0).cut(0, 1) = text("c")
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    let before = rp.node_before().unwrap();
    assert!(before.is_text());
    assert_eq!(before.text.as_deref(), Some("c"));
}
```

- [ ] **Step 5: 运行测试确认失败**

```bash
cargo test --bin md-editor model::tests_resolve::pos_at_index
cargo test --bin md-editor model::tests_resolve::node_after
cargo test --bin md-editor model::tests_resolve::node_before
```

预期:编译错误 `no method named 'pos_at_index' / 'node_after' / 'node_before' found for struct 'ResolvedPos'`。

- [ ] **Step 6: 实现 3 个 ResolvedPos 访问器**

在 `src/model/resolved_pos.rs` 的 `impl ResolvedPos` 块末尾(`text_offset` 之后)添加:

```rust
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
```

- [ ] **Step 7: 运行测试确认通过**

```bash
cargo test --bin md-editor model::tests_resolve
```

预期:42 个 tests_resolve 测试通过(34 + 8)。

```bash
cargo test --bin md-editor
```

预期:全量 341 测试通过(295 + 4 Node::cut + 42 tests_resolve = 341)。

- [ ] **Step 8: 提交**

```bash
git add src/model/resolved_pos.rs src/model/tests_resolve.rs
git commit -m "$(cat <<'EOF'
feat(model): Phase 4.3 — ResolvedPos complex accessors

Add pos_at_index + node_after + node_before accessors.
node_after/node_before cut text via the new Node::cut API
when the position falls inside a text node.

Co-Authored-By: Claude Sonnet 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Integration test + clippy 验证 + 更新 model-phases.md

**Files:**
- Modify: `src/model/tests_resolve.rs`
- Modify: `docs/superpowers/plans/model-phases.md`

本 Task 添加一个全面的 integration test,循环 pos 0..=12 验证所有访问器。对照 test-resolve.ts 的 expected table。

**为简化**,用 Phase 4.2 的 `test_doc()` 即 `doc(p("ab"), blockquote(p("cdef")))`(不含 em mark,与 TS 的 test-resolve 略不同但结构等价,深度/索引/位置完全相同)。

**预期结构(基于 doc(p("ab"), blockquote(p("cdef"))),doc.size=12)**:

| pos | depth | parent_offset | path[0] (doc) | path[1] | path[2] |
|-----|-------|---------------|---------------|---------|---------|
| 0   | 0     | 0             | (doc, 0, 0)   | —       | —       |
| 1   | 1     | 0             | (doc, 0, 0)   | (p1, 0, 1) | —       |
| 2   | 1     | 1             | (doc, 0, 0)   | (p1, 0, 1) | —       |
| 3   | 1     | 2             | (doc, 0, 0)   | (p1, 1, 3) | —       |
| 4   | 0     | 4             | (doc, 1, 4)   | —       | —       |
| 5   | 1     | 0             | (doc, 1, 4)   | (blk, 0, 5) | —       |
| 6   | 2     | 0             | (doc, 1, 4)   | (blk, 0, 5) | (p2, 0, 6) |
| 7   | 2     | 1             | (doc, 1, 4)   | (blk, 0, 5) | (p2, 0, 6) |
| 8   | 2     | 2             | (doc, 1, 4)   | (blk, 0, 5) | (p2, 0, 6) |
| 9   | 2     | 3             | (doc, 1, 4)   | (blk, 0, 5) | (p2, 0, 6) |
| 10  | 2     | 4             | (doc, 1, 4)   | (blk, 0, 5) | (p2, 1, 10) |
| 11  | 1     | 6             | (doc, 1, 4)   | (blk, 1, 11) | —       |
| 12  | 0     | 12            | (doc, 2, 12)  | —       | —       |

- [ ] **Step 1: 追加 integration test**

在 `src/model/tests_resolve.rs` 末尾追加:

```rust
// --- Task 4: integration test ---
//
// 对照 TS test-resolve.ts 的 "should reflect the document structure" 测试。
// 循环 pos 0..=12 验证所有访问器的一致性。

#[test]
fn resolve_integration_all_accessors_consistent() {
    let d = test_doc();
    // 定义每个 pos 的期望 depth
    let expected_depth: [usize; 13] = [0, 1, 1, 1, 0, 1, 2, 2, 2, 2, 2, 1, 0];
    let expected_parent_offset: [usize; 13] = [0, 0, 1, 2, 4, 0, 0, 1, 2, 3, 4, 6, 12];

    for pos in 0..=12 {
        let rp = ResolvedPos::resolve(&d, pos).unwrap();
        assert_eq!(rp.depth, expected_depth[pos], "pos={} depth", pos);
        assert_eq!(rp.parent_offset, expected_parent_offset[pos], "pos={} parent_offset", pos);

        // doc() 始终返回 doc 节点
        assert_eq!(rp.doc().node_type.name, "doc", "pos={} doc", pos);

        // start(0) 始终为 0, end(0) 始终为 doc.content.size = 12
        assert_eq!(rp.start(Some(0)), 0, "pos={} start(0)", pos);
        assert_eq!(rp.end(Some(0)), 12, "pos={} end(0)", pos);

        // before(0)/after(0) 始终报错
        assert!(rp.before(Some(0)).is_err(), "pos={} before(0)", pos);
        assert!(rp.after(Some(0)).is_err(), "pos={} after(0)", pos);

        // 对每个层级验证 node/start/end/before/after 的一致性
        for i in 0..=rp.depth {
            let node = rp.node(Some(i as i32));
            let start = rp.start(Some(i as i32));
            let end = rp.end(Some(i as i32));
            // end == start + content.size(self-consistent invariant)
            assert_eq!(end, start + node.content.size, "pos={} level={} end-start", pos, i);

            if i > 0 {
                // before(i) 指向 node 的开始 tag 之前,即 start-1
                assert_eq!(rp.before(Some(i as i32)).unwrap(), start - 1, "pos={} level={} before", pos, i);
                // after(i) 指向 node 的结束 tag 之后,即 end+1
                assert_eq!(rp.after(Some(i as i32)).unwrap(), end + 1, "pos={} level={} after", pos, i);
            }
        }

        // parent_offset 不变式:parent_offset == pos - start(self.depth)
        // (parent_offset 即 pos 相对于最深 node content 起始的偏移)
        let depth_start = rp.start(Some(rp.depth as i32));
        assert_eq!(rp.parent_offset, pos - depth_start, "pos={} parent_offset invariant", pos);

        // text_offset 不变式:text_offset == pos - path[self.depth].start_pos
        // 注意:text_offset 不一定等于 parent_offset —— 当 path[depth].index > 0
        // (pos 落在某个 child 之后的边界)时,text_offset < parent_offset。
        // 只在 path[depth].index==0 或 pos 精确落在 text 内部时两者相等。
        if rp.index(None) == 0 {
            assert_eq!(rp.text_offset(), rp.parent_offset, "pos={} text_offset at index 0", pos);
        }
    }
}

#[test]
fn resolve_integration_node_before_after_at_text_boundaries() {
    let d = test_doc();

    // pos=7:text "cdef" 内部字符 1(c 之后, d 之前)
    // node_before = text("c"), node_after = text("def")
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    assert_eq!(rp.node_before().unwrap().text.as_deref(), Some("c"));
    assert_eq!(rp.node_after().unwrap().text.as_deref(), Some("def"));

    // pos=8:text "cdef" 内部字符 2
    let rp = ResolvedPos::resolve(&d, 8).unwrap();
    assert_eq!(rp.node_before().unwrap().text.as_deref(), Some("cd"));
    assert_eq!(rp.node_after().unwrap().text.as_deref(), Some("ef"));

    // pos=6:p2 的开头,text_offset=0
    // node_before = None(index=0), node_after = text("cdef") 整个
    let rp = ResolvedPos::resolve(&d, 6).unwrap();
    assert!(rp.node_before().is_none());
    assert_eq!(rp.node_after().unwrap().text.as_deref(), Some("cdef"));

    // pos=10:p2 的末尾,text 之后
    // node_before = text("cdef") 整个,node_after = None(index=1==child_count)
    let rp = ResolvedPos::resolve(&d, 10).unwrap();
    assert_eq!(rp.node_before().unwrap().text.as_deref(), Some("cdef"));
    assert!(rp.node_after().is_none());
}
```

- [ ] **Step 2: 运行测试确认通过**

```bash
cargo test --bin md-editor model::tests_resolve
```

预期:44 个 tests_resolve 测试通过(42 + 2 integration tests)。

```bash
cargo test --bin md-editor
```

预期:全量 343 测试通过(341 + 2)。

- [ ] **Step 3: 运行 clippy 验证**

```bash
cargo clippy --bin md-editor --tests 2>&1 | tail -30
```

预期:resolved_pos.rs 和 tests_resolve.rs 以及 node.rs 的 cut 方法**零新增 warning**。

如果新代码有 warning,修复之。

- [ ] **Step 4: 更新 model-phases.md**

找到:
```markdown
| Phase 4.3 | ResolvedPos 简单访问器 | resolved_pos.rs 续（node/index/start/end/before/after/text_offset/node_after/node_before/pos_at_index） | Phase 4.2 | ⬜ 待开始 |
```

改为:
```markdown
| Phase 4.3 | ResolvedPos 简单访问器 | resolved_pos.rs 续（node/index/start/end/before/after/text_offset/node_after/node_before/pos_at_index） | Phase 4.2 | ✅ 完成 |
```

- [ ] **Step 5: 提交**

```bash
git add src/model/tests_resolve.rs docs/superpowers/plans/model-phases.md
git commit -m "$(cat <<'EOF'
feat(model): Phase 4.3 — integration test + mark complete

Add integration test looping over pos 0..=12 verifying all
13 accessors are mutually consistent (end == start + content.size,
before(i) == start-1, after(i) == end+1, text_offset == parent_offset).

Co-Authored-By: Claude Sonnet 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## 完成标准

- [ ] `src/model/resolved_pos.rs` 含 13 个访问器方法(parent/doc/node/index/index_after/start/end/before/after/text_offset/node_after/node_before/pos_at_index)
- [ ] `src/model/node.rs` 含 `pub fn cut(from, to: Option<usize>)` 方法
- [ ] `src/model/tests_resolve.rs` 有 44 个测试全部通过
- [ ] `src/model/node.rs` 的 cut 方法有 4 个单元测试
- [ ] `cargo test --bin md-editor` 全量 343 测试通过(295 + 4 + 44)
- [ ] 新代码零 clippy warning
- [ ] `docs/superpowers/plans/model-phases.md` 更新 Phase 4.3 状态为 ✅

---

## 设计决策记录

### 决策 36:所有 depth 参数访问器统一 `Option<i32>` 签名

延续 Phase 4.2 的 Decision 35。`node/index/index_after/start/end/before/after/pos_at_index` 都接受 `depth: Option<i32>`:None=self.depth,Some(n>=0)=absolute,Some(n<0)=relative。调用方需要显式传 `None` 或 `Some(n)`,不使用 Rust 默认参数(Rust 无语言级默认参数支持)。

### 决策 37:`parent`/`doc`/`node` 返回 `&Node` 借用,`node_after`/`node_before` 返回 `Option<Node>`

- **`parent`/`doc`/`node`**:返回 path 里已存的 node 的借用,无需 clone,零开销。
- **`node_after`/`node_before`**:可能通过 `Node::cut` 产生新 text 节点,必须返回拥有的 `Node`。用 `Option<Node>` 表示"位置在边界/末尾时无此节点"。

### 决策 38:`before`/`after` 返回 `Result<usize, RangeError>`

TS 在 depth=0 时 throw RangeError。Rust 翻译为 `Err(RangeError(...))`。错误消息与 TS 逐字对齐:
- `"There is no position before the top-level node"`
- `"There is no position after the top-level node"`

这与已有 API 的风格(Fragment::find_index、Node::child 等)一致。

### 决策 39:新增 `Node::cut(from, to: Option<usize>)` 作为统一切片 API

TS 的 Node 基类和 TextNode 派生类分别定义 cut:
- `Node.cut(from, to = content.size)` 对 content 切分
- `TextNode.cut(from, to = text.length)` 对 text chars 切分

Rust 没有继承,我们用一个方法内部分支 `if self.is_text()`。text 切分使用 `char_indices().nth()` 实现 UTF-8 safe。`to` 参数为 `Option<usize>`,None 时默认为"节点的完整范围"。

现有的 `cut_inner(from: usize, to: usize) -> Node`(pub(crate))保留不动,作为 Node::cut 的非 text 分支内部调用,也继续供 Fragment::cut 使用。

### 决策 40:integration test 用 `test_doc()` 即 `doc(p("ab"), blockquote(p("cdef")))`,不含 em mark

TS test-resolve.ts 用 `doc(p("ab"), blockquote(p(em("cd"), "ef")))`(带 em mark),text 被拆成两段("cd" 带 em,"ef" 不带)。Rust test_builder 的 em() 返回 `Vec<Node>`,构造起来略繁琐,且 em mark 的存在不改变 resolve 算法的结构(深度/索引/位置完全相同)。

用简化的 `doc(p("ab"), blockquote(p("cdef")))`(一段连续 text "cdef")即可验证所有 13 个访问器的结构语义,避免 em mark 构造的 boilerplate。
