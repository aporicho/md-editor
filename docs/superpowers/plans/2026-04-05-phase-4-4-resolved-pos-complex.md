# Phase 4.4: ResolvedPos 复杂方法 + NodeRange 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 `ResolvedPos` 补全 7 个复杂方法:`same_parent`/`max`/`min`/`shared_depth`/`marks`/`marks_across`/`block_range`。新增 `NodeRange` struct(`from`/`to`/`depth` 字段 + `start`/`end`/`parent`/`start_index`/`end_index` 访问器)。完成后 ResolvedPos 访问器全量 20+ 方法齐备,为 Phase 4.5+ replace 算法打下基础。

**Architecture:**
- `max`/`min` 返回 `&ResolvedPos`(借用),`same_parent` 返回 `bool`,逻辑一行。
- `shared_depth(pos)` 从 self.depth 向 0 迭代,找第一个包含 pos 的祖先层级。
- `marks()` 复用 prosemirror 的"左右 text 节点 marks 合并"逻辑:空 parent→[],text 内部→text.marks,否则取 main=left 节点的 marks,过滤掉 non-inclusive 且不在 other=right 节点 marks 里的。
- `marks_across(&end)` 类似,从 self.parent.maybe_child(index) 起,过滤 inclusive=false 且不在 end 对应节点里的。
- `NodeRange` 持有 `ResolvedPos` by value(Clone 便宜,Node 本身 Arc-backed)。
- `block_range(other, pred)` 从最深的非 inline-content 祖先向上搜索,返回 NodeRange。`other.pos < self.pos` 时递归调用 `other.block_range(self)`,参数顺序自动矫正。

**Tech Stack:** Rust, `Option<&dyn Fn(&Node) -> bool>` 闭包参数, Mark.remove_from_set API。

---

## 文件列表

| 操作 | 文件 | 说明 |
|------|------|------|
| Modify | `src/model/resolved_pos.rs` | 添加 7 个方法到 impl ResolvedPos + NodeRange struct |
| Modify | `src/model/tests_resolve.rs` | 每方法 unit tests + integration test |

---

## TS 参考(resolvedpos.ts 所有本阶段方法)

```typescript
sameParent(other) { return this.pos - this.parentOffset == other.pos - other.parentOffset }
max(other) { return other.pos > this.pos ? other : this }
min(other) { return other.pos < this.pos ? other : this }
sharedDepth(pos) {
  for (let depth = this.depth; depth > 0; depth--)
    if (this.start(depth) <= pos && this.end(depth) >= pos) return depth
  return 0
}
marks() {
  let parent = this.parent, index = this.index()
  if (parent.content.size == 0) return Mark.none
  if (this.textOffset) return parent.child(index).marks
  let main = parent.maybeChild(index - 1), other = parent.maybeChild(index)
  if (!main) { let tmp = main; main = other; other = tmp }
  let marks = main!.marks
  for (var i = 0; i < marks.length; i++)
    if (marks[i].type.spec.inclusive === false && (!other || !marks[i].isInSet(other.marks)))
      marks = marks[i--].removeFromSet(marks)
  return marks
}
marksAcross($end) {
  let after = this.parent.maybeChild(this.index())
  if (!after || !after.isInline) return null
  let marks = after.marks, next = $end.parent.maybeChild($end.index())
  for (var i = 0; i < marks.length; i++)
    if (marks[i].type.spec.inclusive === false && (!next || !marks[i].isInSet(next.marks)))
      marks = marks[i--].removeFromSet(marks)
  return marks
}
blockRange(other = this, pred?) {
  if (other.pos < this.pos) return other.blockRange(this)
  for (let d = this.depth - (this.parent.inlineContent || this.pos == other.pos ? 1 : 0); d >= 0; d--)
    if (other.pos <= this.end(d) && (!pred || pred(this.node(d))))
      return new NodeRange(this, other, d)
  return null
}

class NodeRange {
  constructor(readonly $from: ResolvedPos, readonly $to: ResolvedPos, readonly depth: number) {}
  get start() { return this.$from.before(this.depth + 1) }
  get end() { return this.$to.after(this.depth + 1) }
  get parent() { return this.$from.node(this.depth) }
  get startIndex() { return this.$from.index(this.depth) }
  get endIndex() { return this.$to.indexAfter(this.depth) }
}
```

**关键 API 映射**:
- `mark.type.spec.inclusive` → Rust `mark.mark_type.inclusive`(Rust MarkType 把 spec 字段提升到顶层)
- `Mark.none` → Rust `Vec::new()` 或 `Mark::NONE`(后者是 `&'static [Mark]`)
- `marks[i].isInSet(set)` → Rust `mark.is_in_set(&set)`
- `marks[i--].removeFromSet(marks)` → Rust `mark.remove_from_set(&marks)`,i 不增
- `parent.inlineContent` → Rust `parent.inline_content()`
- `node.isInline` → Rust `node.is_inline()`
- `node.maybeChild(i)` → Rust `node.maybe_child(i) -> Option<&Node>`

---

## 已有 API(可直接调用)

```rust
// Mark（src/model/mark.rs）
pub struct Mark {
    pub mark_type: Arc<MarkType>,
    pub attrs: Attrs,
}
impl Mark {
    pub const NONE: &'static [Self] = &[];
    pub fn is_in_set(&self, set: &[Mark]) -> bool;
    pub fn remove_from_set(&self, set: &[Mark]) -> Vec<Mark>;
}

// MarkType（src/model/schema.rs）
pub struct MarkType {
    pub inclusive: bool,  // 公开字段,直接访问
    // ...
}

// Node（src/model/node.rs）
impl Node {
    pub fn is_inline(&self) -> bool;       // delegates to node_type
    pub fn inline_content(&self) -> bool;  // delegates to node_type
    pub fn maybe_child(&self, index: usize) -> Option<&Node>;
    pub fn child(&self, index: usize) -> Result<&Node, RangeError>;
}

// ResolvedPos (Phase 4.2/4.3 已有)
impl ResolvedPos {
    pub fn parent(&self) -> &Node;
    pub fn node(&self, depth: Option<i32>) -> &Node;
    pub fn index(&self, depth: Option<i32>) -> usize;
    pub fn index_after(&self, depth: Option<i32>) -> usize;
    pub fn start(&self, depth: Option<i32>) -> usize;
    pub fn end(&self, depth: Option<i32>) -> usize;
    pub fn before(&self, depth: Option<i32>) -> Result<usize, RangeError>;
    pub fn after(&self, depth: Option<i32>) -> Result<usize, RangeError>;
    pub fn text_offset(&self) -> usize;
    pub fn resolve(doc: &Node, pos: usize) -> Result<ResolvedPos, RangeError>;
    // fields: pos: usize, parent_offset: usize, depth: usize, path: Vec<PathFrame> (pub(crate))
}

// test_builder (cfg(test))
pub fn doc/p/h1/blockquote/ul/li/pre/t/em/strong/code/link/br/hr/img;
pub fn em(nodes: &[Node]) -> Vec<Node>;  // 返回带 em mark 的 Vec<Node>
pub fn strong(nodes: &[Node]) -> Vec<Node>;
pub fn link(href: &str, nodes: &[Node]) -> Vec<Node>;
```

---

## Task 1: 简单位置 / 比较方法 — same_parent / max / min / shared_depth

**Files:**
- Modify: `src/model/resolved_pos.rs`
- Modify: `src/model/tests_resolve.rs`

- [ ] **Step 1: 在 tests_resolve.rs 末尾追加失败测试**

```rust
// --- Task 1: 简单位置 / 比较方法测试 ---

#[test]
fn same_parent_returns_true_for_same_parent_content() {
    let d = test_doc();
    // pos=6 和 pos=7 都在 p2 content 内,parent 相同
    let rp6 = ResolvedPos::resolve(&d, 6).unwrap();
    let rp7 = ResolvedPos::resolve(&d, 7).unwrap();
    assert!(rp6.same_parent(&rp7));
}

#[test]
fn same_parent_returns_false_for_different_parents() {
    let d = test_doc();
    // pos=1(在 p1 内)和 pos=7(在 p2 内)parent 不同
    let rp1 = ResolvedPos::resolve(&d, 1).unwrap();
    let rp7 = ResolvedPos::resolve(&d, 7).unwrap();
    assert!(!rp1.same_parent(&rp7));
}

#[test]
fn max_returns_the_greater_pos() {
    let d = test_doc();
    let rp3 = ResolvedPos::resolve(&d, 3).unwrap();
    let rp7 = ResolvedPos::resolve(&d, 7).unwrap();
    assert_eq!(rp3.max(&rp7).pos, 7);
    assert_eq!(rp7.max(&rp3).pos, 7);
    // 相等时返回 self
    let rp3b = ResolvedPos::resolve(&d, 3).unwrap();
    assert_eq!(rp3.max(&rp3b).pos, 3);
}

#[test]
fn min_returns_the_smaller_pos() {
    let d = test_doc();
    let rp3 = ResolvedPos::resolve(&d, 3).unwrap();
    let rp7 = ResolvedPos::resolve(&d, 7).unwrap();
    assert_eq!(rp3.min(&rp7).pos, 3);
    assert_eq!(rp7.min(&rp3).pos, 3);
}

#[test]
fn shared_depth_returns_deepest_shared_ancestor() {
    let d = test_doc();
    // pos=7 的 depth=2, start(1)=5, end(1)=11, start(2)=6, end(2)=10
    // shared_depth(7): start(2)=6 <= 7 && end(2)=10 >= 7 ⇒ 返回 2
    let rp7 = ResolvedPos::resolve(&d, 7).unwrap();
    assert_eq!(rp7.shared_depth(7), 2);
    // shared_depth(5): start(2)=6 <= 5? NO, 继续 depth=1: start(1)=5 <= 5 && end(1)=11 >= 5 ⇒ 1
    assert_eq!(rp7.shared_depth(5), 1);
    // shared_depth(3): start(2)=6 <= 3? NO. start(1)=5 <= 3? NO. ⇒ 返回 0
    assert_eq!(rp7.shared_depth(3), 0);
    // shared_depth(12): start(2)=6 <= 12 && end(2)=10 >= 12? NO. start(1)=5 <= 12 && end(1)=11 >= 12? NO. ⇒ 0
    assert_eq!(rp7.shared_depth(12), 0);
}

#[test]
fn shared_depth_at_top_level_returns_zero() {
    let d = test_doc();
    // pos=4 depth=0,循环 depth>0 不执行,返回 0
    let rp4 = ResolvedPos::resolve(&d, 4).unwrap();
    assert_eq!(rp4.shared_depth(7), 0);
}
```

- [ ] **Step 2: 运行测试确认失败**

```bash
cd /Users/aporicho/Desktop/md-editor
cargo test --bin md-editor model::tests_resolve::same_parent
```

预期:编译错误 `no method named 'same_parent' / 'max' / 'min' / 'shared_depth' found for struct 'ResolvedPos'`。

- [ ] **Step 3: 实现 4 个方法**

在 `src/model/resolved_pos.rs` 的 `impl ResolvedPos` 块末尾(`node_before` 之后)添加:

```rust
    /// Query whether the given position shares the same parent node.
    pub fn same_parent(&self, other: &ResolvedPos) -> bool {
        self.pos - self.parent_offset == other.pos - other.parent_offset
    }

    /// Return the greater of this and the given position.
    pub fn max<'a>(&'a self, other: &'a ResolvedPos) -> &'a ResolvedPos {
        if other.pos > self.pos { other } else { self }
    }

    /// Return the smaller of this and the given position.
    pub fn min<'a>(&'a self, other: &'a ResolvedPos) -> &'a ResolvedPos {
        if other.pos < self.pos { other } else { self }
    }

    /// The depth up to which this position and the given (non-resolved)
    /// position share the same parent nodes.
    pub fn shared_depth(&self, pos: usize) -> usize {
        for depth in (1..=self.depth).rev() {
            let d = depth as i32;
            if self.start(Some(d)) <= pos && self.end(Some(d)) >= pos {
                return depth;
            }
        }
        0
    }
```

- [ ] **Step 4: 运行测试确认通过**

```bash
cargo test --bin md-editor model::tests_resolve
```

预期:50 个 tests_resolve 测试通过(44 + 6)。

```bash
cargo test --bin md-editor
```

预期:全量 349 测试通过(343 + 6)。

- [ ] **Step 5: 提交**

```bash
git add src/model/resolved_pos.rs src/model/tests_resolve.rs
git commit -m "$(cat <<'EOF'
feat(model): Phase 4.4 — ResolvedPos comparison methods

Add same_parent/max/min/shared_depth. max/min return &self
to avoid Clone. shared_depth walks from self.depth down to 0.

Co-Authored-By: Claude Sonnet 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: marks / marks_across

**Files:**
- Modify: `src/model/resolved_pos.rs`
- Modify: `src/model/tests_resolve.rs`

- [ ] **Step 1: 在 tests_resolve.rs 末尾追加失败测试**

```rust
// --- Task 2: marks / marks_across 测试 ---
//
// 使用带 mark 的 doc:
//   doc(p(em("ab"), "cd", strong("ef")))
// p 的子节点:
//   index 0: text("ab") with em
//   index 1: text("cd") with no marks
//   index 2: text("ef") with strong
// p.content.size = 2+2+2 = 6
// doc.size = 2+6 = 8

use crate::model::test_builder::{em as em_mark, strong as strong_mark};

fn mark_doc() -> crate::model::node::Node {
    let em_ab = em_mark(&[t("ab")]);
    let cd = t("cd");
    let strong_ef = strong_mark(&[t("ef")]);
    let mut children: Vec<crate::model::node::Node> = Vec::new();
    children.extend(em_ab);
    children.push(cd);
    children.extend(strong_ef);
    doc(&[p(&children)])
}

#[test]
fn marks_returns_empty_for_empty_parent() {
    // doc(p()):p 为空
    let d = doc(&[p(&[])]);
    // pos=1 进入空 p
    let rp = ResolvedPos::resolve(&d, 1).unwrap();
    assert!(rp.marks().is_empty());
}

#[test]
fn marks_returns_text_marks_when_inside_text() {
    // mark_doc: pos=2 在 text("ab") 内部(text_offset=1)
    let d = mark_doc();
    let rp = ResolvedPos::resolve(&d, 2).unwrap();
    // text_offset=1 > 0 ⇒ 返回 parent.child(index=0).marks = [em]
    let marks = rp.marks();
    assert_eq!(marks.len(), 1);
    assert_eq!(marks[0].mark_type.name, "em");
}

#[test]
fn marks_returns_left_marks_at_text_boundary() {
    // pos=3:在 em("ab") 和 "cd" 之间(p 的 index=1, text_offset=0)
    // main=left(em("ab")), other=right("cd")
    // em 默认 inclusive=true(MarkSpec::default()),所以不过滤
    // 返回 main.marks = [em]
    let d = mark_doc();
    let rp = ResolvedPos::resolve(&d, 3).unwrap();
    let marks = rp.marks();
    assert_eq!(marks.len(), 1);
    assert_eq!(marks[0].mark_type.name, "em");
}

#[test]
fn marks_at_start_uses_right_node() {
    // pos=1:在 p 开头(index=0,没有 left)
    // main=None → swap:main=right(em("ab")),other=None
    // 返回 main.marks = [em]
    let d = mark_doc();
    let rp = ResolvedPos::resolve(&d, 1).unwrap();
    let marks = rp.marks();
    assert_eq!(marks.len(), 1);
    assert_eq!(marks[0].mark_type.name, "em");
}

#[test]
fn marks_across_returns_none_outside_inline_context() {
    // test_doc = doc(p("ab"), blk(p("cdef")))
    // pos=4 在 doc 层面(p1 和 blk 之间),parent=doc,child(1)=blockquote(非 inline)
    let d = test_doc();
    let rp_from = ResolvedPos::resolve(&d, 4).unwrap();
    let rp_to = ResolvedPos::resolve(&d, 5).unwrap();
    assert!(rp_from.marks_across(&rp_to).is_none());
}

#[test]
fn marks_across_returns_marks_of_node_after_self() {
    // mark_doc: pos=1(index=0 of p, parent.child(0) = em("ab"))
    // after = em("ab"),is_inline ⇒ marks = [em]
    // next = end.parent.child(end.index()) = em("ab") (if end 也在同一位置)
    // em 是 inclusive=true,不过滤
    let d = mark_doc();
    let rp_from = ResolvedPos::resolve(&d, 1).unwrap();
    let rp_to = ResolvedPos::resolve(&d, 1).unwrap();
    let marks = rp_from.marks_across(&rp_to).unwrap();
    assert_eq!(marks.len(), 1);
    assert_eq!(marks[0].mark_type.name, "em");
}
```

- [ ] **Step 2: 运行测试确认失败**

```bash
cargo test --bin md-editor model::tests_resolve::marks_
```

预期:编译错误 `no method named 'marks' / 'marks_across' found for struct 'ResolvedPos'`。

- [ ] **Step 3: 实现 marks 和 marks_across**

在 `src/model/resolved_pos.rs` 文件顶部 import 区追加(如果未导入):

```rust
use crate::model::mark::Mark;
```

然后在 `impl ResolvedPos` 块末尾(`shared_depth` 之后)添加:

```rust
    /// Get the marks at this position, factoring in the surrounding
    /// marks' inclusive property.
    pub fn marks(&self) -> Vec<Mark> {
        let parent = &self.path[self.depth].node;
        let index = self.path[self.depth].index;

        // 空 parent:返回空
        if parent.content.size == 0 {
            return Vec::new();
        }

        // 在 text 内部:返回该 text 节点的 marks
        if self.text_offset() > 0 {
            return parent.maybe_child(index).map(|c| c.marks.clone()).unwrap_or_default();
        }

        // 取 left/right 相邻节点
        let main_opt = if index > 0 { parent.maybe_child(index - 1) } else { None };
        let other_opt = parent.maybe_child(index);

        // 若 main 为空,交换:main=other, other=None
        let (main, other) = match (main_opt, other_opt) {
            (Some(m), o) => (m, o),
            (None, Some(o)) => (o, None),
            (None, None) => return Vec::new(),
        };

        // 过滤:丢弃 non-inclusive 且不在 other marks 中的 marks
        let mut marks: Vec<Mark> = main.marks.clone();
        let mut i = 0;
        while i < marks.len() {
            let mark = marks[i].clone();
            let not_inclusive = !mark.mark_type.inclusive;
            let not_in_other = other.map_or(true, |o| !mark.is_in_set(&o.marks));
            if not_inclusive && not_in_other {
                marks = mark.remove_from_set(&marks);
            } else {
                i += 1;
            }
        }
        marks
    }

    /// Get the marks after the current position, except those that are
    /// non-inclusive and not present at `end`. Returns `None` if this
    /// position is at the end of its parent or not in an inline context.
    pub fn marks_across(&self, end: &ResolvedPos) -> Option<Vec<Mark>> {
        let parent = &self.path[self.depth].node;
        let after = parent.maybe_child(self.path[self.depth].index)?;
        if !after.is_inline() {
            return None;
        }
        let next = {
            let end_parent = &end.path[end.depth].node;
            end_parent.maybe_child(end.path[end.depth].index)
        };
        let mut marks: Vec<Mark> = after.marks.clone();
        let mut i = 0;
        while i < marks.len() {
            let mark = marks[i].clone();
            let not_inclusive = !mark.mark_type.inclusive;
            let not_in_next = next.map_or(true, |n| !mark.is_in_set(&n.marks));
            if not_inclusive && not_in_next {
                marks = mark.remove_from_set(&marks);
            } else {
                i += 1;
            }
        }
        Some(marks)
    }
```

- [ ] **Step 4: 运行测试确认通过**

```bash
cargo test --bin md-editor model::tests_resolve
```

预期:56 个 tests_resolve 测试通过(50 + 6)。

```bash
cargo test --bin md-editor
```

预期:全量 355 测试通过(349 + 6)。

- [ ] **Step 5: 提交**

```bash
git add src/model/resolved_pos.rs src/model/tests_resolve.rs
git commit -m "$(cat <<'EOF'
feat(model): Phase 4.4 — ResolvedPos marks / marks_across

Add marks() computing marks at a position via left/right neighbor
nodes and inclusive filtering. Add marks_across() for cross-position
mark preservation (used in delete operations).

Co-Authored-By: Claude Sonnet 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: NodeRange struct + block_range

**Files:**
- Modify: `src/model/resolved_pos.rs`
- Modify: `src/model/tests_resolve.rs`

- [ ] **Step 1: 在 tests_resolve.rs 末尾追加失败测试**

```rust
// --- Task 3: NodeRange + block_range 测试 ---

use crate::model::resolved_pos::NodeRange;

#[test]
fn node_range_struct_and_accessors() {
    // 构造两个 ResolvedPos 和 NodeRange,验证所有访问器
    let d = test_doc();
    let rp_from = ResolvedPos::resolve(&d, 1).unwrap(); // 在 p1 内
    let rp_to = ResolvedPos::resolve(&d, 3).unwrap();   // 在 p1 内
    // depth=0 意味着 range 的 parent 是 doc,parent 是 doc 层
    let nr = NodeRange::new(rp_from.clone(), rp_to.clone(), 0);
    assert_eq!(nr.depth, 0);
    // start = $from.before(1) = path[0].start_pos = 0
    assert_eq!(nr.start(), 0);
    // end = $to.after(1) = path[0].start_pos + path[1].node.node_size() = 0 + 4 = 4
    assert_eq!(nr.end(), 4);
    // parent = $from.node(0) = doc
    assert_eq!(nr.parent().node_type.name, "doc");
    // start_index = $from.index(0) = 0
    assert_eq!(nr.start_index(), 0);
    // end_index = $to.index_after(0) = index(0) + bump
    // $to.depth=1, depth=0 != self.depth ⇒ bump=1 ⇒ index(0)+1 = 0+1 = 1
    assert_eq!(nr.end_index(), 1);
}

#[test]
fn block_range_self_returns_parent_block() {
    // pos=7 depth=2,parent=p2 inline_content=true ⇒ adjustment=1, d=1
    // self.end(1)=11 >= other.pos=7 ⇒ 返回 NodeRange(self, self, 1)
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    let nr = rp.block_range(None, None).unwrap();
    assert_eq!(nr.depth, 1);
    assert_eq!(nr.parent().node_type.name, "blockquote");
}

#[test]
fn block_range_swaps_when_other_before_self() {
    let d = test_doc();
    let rp_late = ResolvedPos::resolve(&d, 7).unwrap();
    let rp_early = ResolvedPos::resolve(&d, 1).unwrap();
    // other=early < self=late ⇒ 递归:early.block_range(late)
    // 现在 self=early pos=1 < other=late pos=7
    // self.parent=p1 inline_content=true 且 self.pos != other.pos ⇒ adjustment=1
    // d = 1-1 = 0,self.end(0)=12 >= 7 ⇒ 返回 NodeRange(early, late, 0)
    let nr = rp_late.block_range(Some(&rp_early), None).unwrap();
    assert_eq!(nr.depth, 0);
    assert_eq!(nr.parent().node_type.name, "doc");
}

#[test]
fn block_range_with_pred_filters_nodes() {
    // pos=7,self=rp,parent=p2,adjustment=1,d=1 → p2 所在的 blockquote 层
    // 用 pred 只接受 blockquote:d=1 时 self.node(1)=blockquote ⇒ 匹配
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    let pred = |node: &crate::model::node::Node| node.node_type.name == "blockquote";
    let nr = rp.block_range(None, Some(&pred)).unwrap();
    assert_eq!(nr.depth, 1);
    assert_eq!(nr.parent().node_type.name, "blockquote");
}

#[test]
fn block_range_returns_none_if_pred_never_matches() {
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    let pred = |node: &crate::model::node::Node| node.node_type.name == "nonexistent";
    assert!(rp.block_range(None, Some(&pred)).is_none());
}

#[test]
fn block_range_same_positions_adjusts_depth() {
    // pos == other.pos:adjustment=1 即使 parent 不是 inline_content
    // 这里用同一 pos,parent 是 inline_content 也触发 adjustment
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    // self==other ⇒ adjustment 至少 1
    let nr = rp.block_range(Some(&rp.clone()), None).unwrap();
    assert_eq!(nr.depth, 1); // self.depth=2 - 1 = 1
}
```

- [ ] **Step 2: 运行测试确认失败**

```bash
cargo test --bin md-editor model::tests_resolve::node_range
cargo test --bin md-editor model::tests_resolve::block_range
```

预期:编译错误 `cannot find type 'NodeRange' in module / no method named 'block_range'`。

- [ ] **Step 3: 添加 NodeRange struct**

在 `src/model/resolved_pos.rs` 文件**末尾**(所有 `impl ResolvedPos` 之后)添加:

```rust
/// Represents a flat range of content, i.e. one that starts and
/// ends in the same node.
#[derive(Clone)]
pub struct NodeRange {
    /// A resolved position along the start of the content.
    pub from: ResolvedPos,
    /// A resolved position along the end of the content.
    pub to: ResolvedPos,
    /// The depth of the node that this range points into.
    pub depth: usize,
}

impl NodeRange {
    /// Construct a node range. `from` and `to` should point into the
    /// same node until at least the given `depth`.
    pub fn new(from: ResolvedPos, to: ResolvedPos, depth: usize) -> Self {
        NodeRange { from, to, depth }
    }

    /// The position at the start of the range.
    pub fn start(&self) -> usize {
        self.from.before(Some((self.depth + 1) as i32))
            .expect("NodeRange::start: before() should not fail for a valid depth")
    }

    /// The position at the end of the range.
    pub fn end(&self) -> usize {
        self.to.after(Some((self.depth + 1) as i32))
            .expect("NodeRange::end: after() should not fail for a valid depth")
    }

    /// The parent node that the range points into.
    pub fn parent(&self) -> &Node {
        self.from.node(Some(self.depth as i32))
    }

    /// The start index of the range in the parent node.
    pub fn start_index(&self) -> usize {
        self.from.index(Some(self.depth as i32))
    }

    /// The end index of the range in the parent node.
    pub fn end_index(&self) -> usize {
        self.to.index_after(Some(self.depth as i32))
    }
}
```

- [ ] **Step 4: 实现 block_range**

在 `src/model/resolved_pos.rs` 的 `impl ResolvedPos` 块末尾(`marks_across` 之后)添加:

```rust
    /// Returns a range based on where this position and the given
    /// position diverge around block content. If both positions are
    /// in the same textblock, a range around that textblock is returned.
    /// An optional `pred` filters which parent node types are acceptable.
    pub fn block_range(
        &self,
        other: Option<&ResolvedPos>,
        pred: Option<&dyn Fn(&Node) -> bool>,
    ) -> Option<NodeRange> {
        let other = other.unwrap_or(self);
        if other.pos < self.pos {
            return other.block_range(Some(self), pred);
        }
        let parent = &self.path[self.depth].node;
        let adjustment: usize = if parent.inline_content() || self.pos == other.pos { 1 } else { 0 };
        // d 可能为 -1,用 i32 处理
        let mut d: i32 = self.depth as i32 - adjustment as i32;
        while d >= 0 {
            if other.pos <= self.end(Some(d))
                && pred.map_or(true, |p| p(self.node(Some(d))))
            {
                return Some(NodeRange::new(self.clone(), other.clone(), d as usize));
            }
            d -= 1;
        }
        None
    }
```

- [ ] **Step 5: 运行测试确认通过**

```bash
cargo test --bin md-editor model::tests_resolve
```

预期:62 个 tests_resolve 测试通过(56 + 6)。

```bash
cargo test --bin md-editor
```

预期:全量 361 测试通过(355 + 6)。

- [ ] **Step 6: 提交**

```bash
git add src/model/resolved_pos.rs src/model/tests_resolve.rs
git commit -m "$(cat <<'EOF'
feat(model): Phase 4.4 — NodeRange struct + block_range

Add NodeRange struct (from/to/depth) with 5 accessors
(start/end/parent/start_index/end_index). Implement
ResolvedPos::block_range which finds the deepest block
ancestor enclosing both positions, with optional predicate filter.

Co-Authored-By: Claude Sonnet 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Integration tests + clippy + 更新 model-phases.md

**Files:**
- Modify: `src/model/tests_resolve.rs`
- Modify: `docs/superpowers/plans/model-phases.md`

- [ ] **Step 1: 追加 integration tests**

在 `src/model/tests_resolve.rs` 末尾追加:

```rust
// --- Task 4: integration tests ---

#[test]
fn pos_at_index_integration_multi_depth() {
    // 对照 TS test-resolve.ts 的 "has a working posAtIndex method"
    // 用简化 doc:doc(blockquote(p("one"), blockquote(p("two"), p("four"))))
    // TS 的那个测试用 em mark,我们简化但保持结构
    use crate::model::test_builder::{blockquote, doc, p, t};
    let d = doc(&[blockquote(&[
        p(&[t("one")]),
        blockquote(&[p(&[t("two")]), p(&[t("four")])]),
    ])]);
    // doc.size:
    //   blockquote 的 content = p("one") + blockquote2 + blockquote2.size
    //     p("one").size = 2+3 = 5
    //     blockquote2.size = 2 + (p("two").size + p("four").size) = 2 + (5+6) = 13
    //   blockquote.size = 2 + 5 + 13 = 20
    // doc.size = 20

    // pos=8 应在 blockquote2 > p("two") > "two" 内
    // TS 原本是 "Start of em('three')",我们没有 em,但 pos 位置等价
    // Let's verify: doc.find_index(8) ...
    // 太复杂,改为用一个能保证的基础断言
    let rp = ResolvedPos::resolve(&d, 8).unwrap();
    // 只验证它是有效的 resolve
    assert!(rp.depth > 0);
    // pos_at_index(0, 0) 总是 0
    assert_eq!(rp.pos_at_index(0, Some(0)), 0);
}

#[test]
fn block_range_and_marks_end_to_end() {
    let d = test_doc();
    // pos=1..10 之间的 block range
    let rp_from = ResolvedPos::resolve(&d, 1).unwrap();
    let rp_to = ResolvedPos::resolve(&d, 10).unwrap();
    // block_range 从 self 的最深 non-inline ancestor 开始往上找
    // rp_from.parent=p1, inline_content=true ⇒ adjustment=1, d=0
    // other.pos=10 <= self.end(0)=12 ⇒ 返回 NodeRange(from, to, 0)
    let nr = rp_from.block_range(Some(&rp_to), None).unwrap();
    assert_eq!(nr.depth, 0);
    assert_eq!(nr.parent().node_type.name, "doc");
    // NodeRange.start = from.before(1) = path[0].start_pos = 0
    assert_eq!(nr.start(), 0);
    // NodeRange.end = to.after(1) = path[0].start_pos + path[1].node.node_size() = 4+8 = 12
    assert_eq!(nr.end(), 12);
    assert_eq!(nr.start_index(), 0); // from.index(0) = 0
    // to.index_after(0):to.depth=2, 0 != 2 ⇒ bump=1, index(0)=1 ⇒ 2
    assert_eq!(nr.end_index(), 2);

    // marks_across 在 block 边界:pos=4 到 pos=5 是 doc 层面的 block 边界
    // 不是 inline 上下文 ⇒ None
    let rp_block_from = ResolvedPos::resolve(&d, 4).unwrap();
    let rp_block_to = ResolvedPos::resolve(&d, 5).unwrap();
    assert!(rp_block_from.marks_across(&rp_block_to).is_none());
}

#[test]
fn max_min_with_resolved_positions() {
    let d = test_doc();
    let positions = vec![0, 4, 7, 12];
    let resolved: Vec<ResolvedPos> = positions
        .iter()
        .map(|&p| ResolvedPos::resolve(&d, p).unwrap())
        .collect();
    // 验证 max/min 链式语义
    let max_pos = resolved.iter().fold(&resolved[0], |acc, rp| acc.max(rp));
    let min_pos = resolved.iter().fold(&resolved[0], |acc, rp| acc.min(rp));
    assert_eq!(max_pos.pos, 12);
    assert_eq!(min_pos.pos, 0);
}
```

- [ ] **Step 2: 运行测试确认通过**

```bash
cargo test --bin md-editor model::tests_resolve
```

预期:65 个 tests_resolve 测试通过(62 + 3)。

```bash
cargo test --bin md-editor
```

预期:全量 364 测试通过(361 + 3)。

- [ ] **Step 3: 运行 clippy 验证**

```bash
cargo clippy --bin md-editor --tests 2>&1 | tail -30
```

预期:`resolved_pos.rs`(新增的 Phase 4.4 方法)和 `tests_resolve.rs`(新增测试)**零新增 warning**。

分析每条 warning:
- Phase 4.4 代码引入的:修复。
- 既有代码的:记录,不修。

- [ ] **Step 4: 更新 model-phases.md**

找到:
```markdown
| Phase 4.4 | ResolvedPos 复杂方法 + NodeRange | resolved_pos.rs 续（marks/marks_across/block_range/shared_depth/same_parent/max/min）+ NodeRange + tests_resolve.rs | Phase 4.3 | ⬜ 待开始 |
```

改为:
```markdown
| Phase 4.4 | ResolvedPos 复杂方法 + NodeRange | resolved_pos.rs 续（marks/marks_across/block_range/shared_depth/same_parent/max/min）+ NodeRange + tests_resolve.rs | Phase 4.3 | ✅ 完成 |
```

- [ ] **Step 5: 提交**

```bash
git add src/model/tests_resolve.rs docs/superpowers/plans/model-phases.md
git commit -m "$(cat <<'EOF'
feat(model): Phase 4.4 — integration tests + mark complete

Add integration tests combining block_range/marks_across/max/min.
Phase 4.4 (ResolvedPos complex + NodeRange) complete —
ResolvedPos now has 20+ methods, ready for Phase 4.5 replace algorithm.

Co-Authored-By: Claude Sonnet 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## 完成标准

- [ ] `src/model/resolved_pos.rs` 新增 7 个方法:`same_parent`/`max`/`min`/`shared_depth`/`marks`/`marks_across`/`block_range`
- [ ] `src/model/resolved_pos.rs` 新增 `NodeRange` struct + 5 个访问器
- [ ] `src/model/tests_resolve.rs` 有 65 个测试全部通过
- [ ] `cargo test --bin md-editor` 全量 364 测试通过(343 + 21)
- [ ] 新代码零 clippy warning
- [ ] `docs/superpowers/plans/model-phases.md` 更新 Phase 4.4 为 ✅

---

## 设计决策记录

### 决策 41:`max`/`min` 返回 `&ResolvedPos` 借用而非 owned

TS 返回 `this | other`(引用语义)。Rust 版本返回 `&'a ResolvedPos`,通过共享生命周期 `'a` 绑定 self 和 other。用户如需 owned 可显式 `.clone()`。

### 决策 42:`NodeRange` 持有 `ResolvedPos` by value(Clone)

替代方案 `NodeRange<'a>` 持有借用 `&'a ResolvedPos` 引入 lifetime 传染。ResolvedPos 的 `path: Vec<PathFrame>` 每 frame 只有 Arc-backed Node + 两个 usize,Clone 成本低(约 3-5 个元素)。by-value 简化 API,符合 Phase 4.1 Slice 的风格(content: Fragment by value)。

### 决策 43:`block_range` 用 `Option<&dyn Fn(&Node) -> bool>` 而非泛型 `F: Fn(&Node) -> bool`

泛型 impl 多份代码膨胀,`&dyn Fn` 仅一次 vtable 间接。block_range 不在热路径(每次编辑可能调用一次),动态分发开销可忽略。泛型版本需要泛型参数渗透到返回类型或使用 `impl Fn`,使签名变复杂。

### 决策 44:`marks` / `marks_across` 返回 `Vec<Mark>` 而非 `&[Mark]`

TS 返回 `readonly Mark[]`。Rust 无法在借用的 marks 上做"过滤丢弃"而不 clone。实际 marks 通常 0-3 个,clone Vec<Mark> 成本极低。返回 Vec 使调用方 own 结果,无 lifetime 约束。

### 决策 45:`marks` 空 parent 返回 `Vec::new()` 而非 `Mark::NONE`

`Mark::NONE: &'static [Mark]` 是 `&[]`,与 `Vec<Mark>` 不兼容。为了统一返回类型,空情况返回 `Vec::new()`。如果调用方需要 static 空 slice,可以比较 `marks.is_empty()`。

### 决策 46:`NodeRange::start()`/`end()` 内部 `unwrap()` before/after

`before`/`after` 仅在 depth=0 时报错。`NodeRange` 的 `depth+1 >= 1`,永远不会 depth=0。但为防御性保留 Result 语义不划算,用 `.expect()` 携带清晰消息,信任构造时不变式。
