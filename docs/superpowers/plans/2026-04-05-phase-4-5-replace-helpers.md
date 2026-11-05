# Phase 4.5 — replace 内部辅助函数 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 `replace.rs` 中 replace 算法所需的所有私有辅助函数，为 Phase 4.6（replace_two_way / replace_three_way）和 Phase 4.7（replace / replace_outer）做准备。

**Architecture:** 6 个私有函数 + 1 个公开入口函数（`replace`），函数之间有调用依赖。`close` 被几乎所有高层函数调用；`add_node` / `add_range` 是构建节点数组的工具；`check_join` / `joinable` 校验节点类型兼容性；`prepare_slice_for_replace` 准备 slice 供 replace 算法消费。本阶段无独立测试（由 Phase 4.7 集成测试覆盖），但每个函数须通过 `cargo test`（无编译错误、无回归）。

**Tech Stack:** Rust, prosemirror-model (Rust port)

---

## 文件结构

| 文件 | 变更类型 | 职责 |
|---|---|---|
| `src/model/replace.rs` | 修改 | 追加 6 个私有辅助函数 + `replace` 公开入口 stub |
| `src/model/errors.rs` | 只读 | 使用 `ReplaceError`、`RangeError` |
| `src/model/resolved_pos.rs` | 只读 | 使用 `ResolvedPos` 的访问器 |
| `src/model/node.rs` | 只读 | 使用 `Node` 的方法 |
| `src/model/fragment.rs` | 只读 | 使用 `Fragment::from_array` 等 |
| `src/model/schema.rs` | 只读 | 使用 `NodeType::check_content`、`compatible_content` |

---

## 函数调用依赖图

```
replace (公开入口, Phase 4.7 补全)
  └─ replaceOuter (Phase 4.7)
       ├─ close ← Task 1
       ├─ replaceTwoWay (Phase 4.6)
       │    ├─ addRange ← Task 3
       │    │    └─ addNode ← Task 2
       │    ├─ joinable ← Task 4
       │    │    └─ checkJoin ← Task 4
       │    └─ close
       ├─ replaceThreeWay (Phase 4.6)
       │    ├─ addRange / addNode
       │    ├─ joinable / checkJoin
       │    └─ close
       └─ prepareSliceForReplace ← Task 5
```

---

## 参考源码映射

| TS 函数 (replace.ts) | 行号 | Rust 函数名 | Task |
|---|---|---|---|
| `close(node, content)` | 182-185 | `close(node, content)` | Task 1 |
| `addNode(child, target)` | 157-163 | `add_node(child, target)` | Task 2 |
| `addRange($start, $end, depth, target)` | 165-180 | `add_range(start, end, depth, target)` | Task 3 |
| `checkJoin(main, sub)` | 146-149 | `check_join(main, sub)` | Task 4 |
| `joinable($before, $after, depth)` | 151-155 | `joinable(before, after, depth)` | Task 4 |
| `prepareSliceForReplace(slice, $along)` | 218-225 | `prepare_slice_for_replace(slice, along)` | Task 5 |

---

### Task 1: `close` 函数

**Files:**
- Modify: `src/model/replace.rs:232` (文件末尾追加)

- [ ] **Step 1: 实现 `close`**

在 `src/model/replace.rs` 文件末尾追加：

```rust
use crate::model::errors::ReplaceError;
use crate::model::resolved_pos::ResolvedPos;
```

然后追加函数：

```rust
/// Internal: validate content against the node's type, then copy with new content.
///
/// Corresponds to TS `close(node, content)` in replace.ts:182-185.
/// Panics via `check_content` if content is invalid for the node type.
fn close(node: &Node, content: Fragment) -> Result<Node, ReplaceError> {
    node.node_type.check_content(&content).map_err(|e| ReplaceError(e.0))?;
    Ok(node.copy(Some(content)))
}
```

**设计说明：** TS 的 `close` 直接调用 `checkContent` 可能抛出异常。Rust 版本返回 `Result<Node, ReplaceError>`，由调用方用 `?` 传播。`check_content` 返回 `Err(RangeError)` 时转为 `ReplaceError`。

- [ ] **Step 2: 运行 `cargo test` 确认无编译错误无回归**

Run: `cargo test 2>&1 | tail -5`
Expected: `test result: ok. 366 passed; 0 failed`

- [ ] **Step 3: Commit**

```bash
git add src/model/replace.rs
git commit -m "feat(model): Phase 4.5 — close helper function"
```

---

### Task 2: `add_node` 函数

**Files:**
- Modify: `src/model/replace.rs` (在 `close` 之后追加)

- [ ] **Step 1: 实现 `add_node`**

在 `close` 函数之后追加：

```rust
/// Internal: push `child` onto `target`, merging adjacent text nodes with
/// identical markup.
///
/// Corresponds to TS `addNode(child, target)` in replace.ts:157-163.
fn add_node(child: Node, target: &mut Vec<Node>) {
    let last = target.len().wrapping_sub(1); // wrapping to avoid underflow panic; checked below
    if !target.is_empty() && child.is_text() && child.same_markup(&target[last]) {
        // 合并相邻文本节点
        let merged_text = format!(
            "{}{}",
            target[last].text.as_deref().unwrap_or(""),
            child.text.as_deref().unwrap_or("")
        );
        target[last] = target[last].with_text(merged_text);
    } else {
        target.push(child);
    }
}
```

**设计说明：** TS 中 `(child as TextNode).withText(target[last].text! + child.text!)` 直接拼接。Rust 版本用 `format!` + `with_text`。`with_text` 是 `pub(crate)` 的，replace.rs 在同一 crate 内可以访问。

- [ ] **Step 2: 运行 `cargo test` 确认无编译错误无回归**

Run: `cargo test 2>&1 | tail -5`
Expected: `test result: ok. 366 passed; 0 failed`

- [ ] **Step 3: Commit**

```bash
git add src/model/replace.rs
git commit -m "feat(model): Phase 4.5 — add_node helper function"
```

---

### Task 3: `add_range` 函数

**Files:**
- Modify: `src/model/replace.rs` (在 `add_node` 之后追加)

- [ ] **Step 1: 实现 `add_range`**

在 `add_node` 函数之后追加：

```rust
/// Internal: collect child nodes from a resolved-position range into `target`.
///
/// Collects children of the node at `depth` between the positions indicated
/// by `start` and `end`. Handles partial text nodes at boundaries via
/// `node_after` / `node_before`.
///
/// Corresponds to TS `addRange($start, $end, depth, target)` in replace.ts:165-180.
fn add_range(
    start: Option<&ResolvedPos>,
    end: Option<&ResolvedPos>,
    depth: usize,
    target: &mut Vec<Node>,
) {
    let node = end.or(start).expect("add_range: at least one of start/end must be Some")
        .node(Some(depth as i32));
    let mut start_index = 0usize;
    let end_index = match end {
        Some(e) => e.index(Some(depth as i32)),
        None => node.child_count(),
    };
    if let Some(s) = start {
        start_index = s.index(Some(depth as i32));
        if s.depth > depth {
            start_index += 1;
        } else if s.text_offset() > 0 {
            add_node(s.node_after().expect("add_range: node_after must exist when text_offset > 0"), target);
            start_index += 1;
        }
    }
    for i in start_index..end_index {
        add_node(
            node.child(i).expect("add_range: index within child_count").clone(),
            target,
        );
    }
    if let Some(e) = end {
        if e.depth == depth && e.text_offset() > 0 {
            add_node(e.node_before().expect("add_range: node_before must exist when text_offset > 0"), target);
        }
    }
}
```

**设计说明：**
- TS 中 `($end || $start)!.node(depth)` 对应 Rust 的 `end.or(start).expect(...)`.
- `depth` 参数传给 `ResolvedPos::node(Some(depth as i32))` / `index(Some(depth as i32))`。
- `node.child(i)` 返回 `Result<&Node, RangeError>`，此处 `i` 在 `start_index..end_index` 范围内，保证有效，用 `expect`。
- TS 的 `$start.nodeAfter!` 对应 `s.node_after().expect(...)`，因为 `text_offset() > 0` 保证文本节点存在。

- [ ] **Step 2: 运行 `cargo test` 确认无编译错误无回归**

Run: `cargo test 2>&1 | tail -5`
Expected: `test result: ok. 366 passed; 0 failed`

- [ ] **Step 3: Commit**

```bash
git add src/model/replace.rs
git commit -m "feat(model): Phase 4.5 — add_range helper function"
```

---

### Task 4: `check_join` + `joinable` 函数

**Files:**
- Modify: `src/model/replace.rs` (在 `add_range` 之后追加)

- [ ] **Step 1: 实现 `check_join` 和 `joinable`**

在 `add_range` 函数之后追加：

```rust
/// Internal: check that `sub`'s content type is compatible with `main`'s.
///
/// Corresponds to TS `checkJoin(main, sub)` in replace.ts:146-149.
fn check_join(main: &Node, sub: &Node) -> Result<(), ReplaceError> {
    if !sub.node_type.compatible_content(&main.node_type) {
        return Err(ReplaceError(format!(
            "Cannot join {} onto {}",
            sub.node_type.name, main.node_type.name
        )));
    }
    Ok(())
}

/// Internal: verify that the nodes at `depth + 1` in `before` and `after`
/// are joinable, returning a clone of the node from `before`.
///
/// Corresponds to TS `joinable($before, $after, depth)` in replace.ts:151-155.
fn joinable(
    before: &ResolvedPos,
    after: &ResolvedPos,
    depth: usize,
) -> Result<Node, ReplaceError> {
    let node = before.node(Some(depth as i32)).clone();
    check_join(&node, after.node(Some(depth as i32)))?;
    Ok(node)
}
```

**设计说明：**
- TS 的 `joinable` 返回 node 或抛异常。Rust 版本返回 `Result<Node, ReplaceError>`。
- `check_join` 使用 `NodeType::compatible_content`，该方法检查两个节点类型的 content match 是否相容。
- Phase 4.6 的 `replace_two_way` / `replace_three_way` 会调用 `joinable` 的返回值判断是否为 truthy（TS 中 `if (openStart)` 检查返回值是否存在），在 Rust 中需要变体处理（Phase 4.6 计划中会处理此细节）。

- [ ] **Step 2: 运行 `cargo test` 确认无编译错误无回归**

Run: `cargo test 2>&1 | tail -5`
Expected: `test result: ok. 366 passed; 0 failed`

- [ ] **Step 3: Commit**

```bash
git add src/model/replace.rs
git commit -m "feat(model): Phase 4.5 — check_join + joinable helper functions"
```

---

### Task 5: `prepare_slice_for_replace` 函数

**Files:**
- Modify: `src/model/replace.rs` (在 `joinable` 之后追加)

- [ ] **Step 1: 实现 `prepare_slice_for_replace`**

在 `joinable` 函数之后追加：

```rust
/// Internal: wrap the slice content in ancestor copies so that it can be
/// resolved into `ResolvedPos` pairs (`start`, `end`) for the replace
/// algorithm.
///
/// Corresponds to TS `prepareSliceForReplace(slice, $along)` in replace.ts:218-225.
fn prepare_slice_for_replace(
    slice: &Slice,
    along: &ResolvedPos,
) -> Result<(ResolvedPos, ResolvedPos), RangeError> {
    let extra = along.depth - slice.open_start;
    let parent = along.node(Some(extra as i32));
    let mut node = parent.copy(Some(slice.content.clone()));
    for i in (0..extra).rev() {
        node = along.node(Some(i as i32)).copy(Some(Fragment::from_node(node)));
    }
    let start = ResolvedPos::resolve_no_cache(
        &node,
        slice.open_start + extra,
    )?;
    let end = ResolvedPos::resolve_no_cache(
        &node,
        node.content.size - slice.open_end - extra,
    )?;
    Ok((start, end))
}
```

**设计说明：**
- TS 中返回 `{start, end}` 对象，Rust 返回元组 `(ResolvedPos, ResolvedPos)`。
- `along.node(extra)` 获取 slice 内容对应的祖先节点。
- 循环从 `extra - 1` 到 `0` 反向遍历，逐层用祖先节点包装（`Fragment::from_node(node)` 将单节点包装为 Fragment）。
- 用 `ResolvedPos::resolve_no_cache` 解析包装后节点中的起止位置。

- [ ] **Step 2: 运行 `cargo test` 确认无编译错误无回归**

Run: `cargo test 2>&1 | tail -5`
Expected: `test result: ok. 366 passed; 0 failed`

- [ ] **Step 3: Commit**

```bash
git add src/model/replace.rs
git commit -m "feat(model): Phase 4.5 — prepare_slice_for_replace helper function"
```

---

### Task 6: 更新阶段状态

**Files:**
- Modify: `docs/superpowers/plans/model-phases.md`

- [ ] **Step 1: 更新 Phase 4.5 状态为 ✅ 完成**

在 `model-phases.md` 中将 Phase 4.5 行的状态从 `⬜ 待开始` 改为 `✅ 完成`：

```
| Phase 4.5 | replace 内部辅助函数 | replace.rs 续（remove_range/insert_into/add_node/add_range/close/check_join/prepare_slice_for_replace 等私有函数） | Phase 4.1 + Phase 4.4 | ✅ 完成 |
```

- [ ] **Step 2: Commit**

```bash
git add docs/superpowers/plans/model-phases.md
git commit -m "feat(model): Phase 4.5 — mark complete"
```

---

## 完成后状态

- `replace.rs` 包含：Slice (Phase 4.1) + remove_range + insert_into (Phase 4.1) + close + add_node + add_range + check_join + joinable + prepare_slice_for_replace (Phase 4.5)
- `cargo test` 仍然 366 passed / 0 failed（无新测试，无回归）
- Phase 4.6 可以直接开始实现 `replace_two_way` / `replace_three_way`
