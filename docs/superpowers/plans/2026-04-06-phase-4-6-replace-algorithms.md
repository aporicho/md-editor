# Phase 4.6 — replace 算法积木块 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 `replace_two_way` 和 `replace_three_way` 两个递归函数，它们是 Phase 4.7 `replace_outer` 的核心构建块。

**Architecture:** 两个互相关联的递归函数。`replace_two_way` 处理无 slice 的简单删除场景（从 `$from` 到 `$to` 合并）；`replace_three_way` 处理有 slice 插入的场景（`$from` → `$start` → `$end` → `$to` 四段合并）。两者都调用 Phase 4.5 的辅助函数（`add_range`、`add_node`、`close`、`joinable`、`check_join`）。

**Tech Stack:** Rust, prosemirror-model (Rust port)

---

## 文件结构

| 文件 | 变更类型 | 职责 |
|---|---|---|
| `src/model/replace.rs` | 修改 | 追加 `replace_two_way` 和 `replace_three_way` |

---

## TS → Rust 关键适配

### `joinable` 的 truthy/falsy 模式

TS 中 `joinable` 返回 `Node`（truthy）或抛异常。在 `replaceThreeWay` 中用短路求值：

```typescript
let openStart = $from.depth > depth && joinable($from, $start, depth + 1)
// openStart: false | Node
```

Rust 适配：返回 `Option<Node>`，`joinable` 的 `Err` 通过 `?` 传播：

```rust
let open_start: Option<Node> = if from.depth > depth {
    Some(joinable(from, start, depth + 1)?)
} else {
    None
};
```

### Fragment 构造

TS 的 `new Fragment(content)` 直接构造（不合并文本）。Rust 用 `Fragment::from_array(&content)`——由于 `add_node` 已处理合并，`from_array` 不会再合并，但会正确计算 `size`。

---

## 参考源码映射

| TS 函数 (replace.ts) | 行号 | Rust 函数名 | Task |
|---|---|---|---|
| `replaceTwoWay($from, $to, depth)` | 207-216 | `replace_two_way(from, to, depth)` | Task 1 |
| `replaceThreeWay($from, $start, $end, $to, depth)` | 187-205 | `replace_three_way(from, start, end, to, depth)` | Task 2 |

---

### Task 1: `replace_two_way` 函数

**Files:**
- Modify: `src/model/replace.rs` (在 `prepare_slice_for_replace` 之后追加)

- [ ] **Step 1: 实现 `replace_two_way`**

在 `src/model/replace.rs` 末尾追加：

```rust
/// Internal: build a new fragment by merging content from `$from` to `$to`
/// at the given `depth`, recursing into matching open nodes.
///
/// Used when there is no slice content to insert — just joining the
/// remaining content on both sides.
///
/// Corresponds to TS `replaceTwoWay($from, $to, depth)` in replace.ts:207-216.
fn replace_two_way(
    from: &ResolvedPos,
    to: &ResolvedPos,
    depth: usize,
) -> Result<Fragment, ReplaceError> {
    let mut content: Vec<Node> = Vec::new();
    add_range(None, Some(from), depth, &mut content);
    if from.depth > depth {
        let type_node = joinable(from, to, depth + 1)?;
        add_node(
            close(&type_node, replace_two_way(from, to, depth + 1)?)?,
            &mut content,
        );
    }
    add_range(Some(to), None, depth, &mut content);
    Ok(Fragment::from_array(&content))
}
```

**逻辑说明：**
1. 收集 `from` 之前的所有兄弟节点（`add_range(None, from, depth, ...)`）
2. 如果 `from` 深度 > 当前深度，说明有开放节点需要递归合并：
   - `joinable` 验证两侧节点兼容性，返回 `from` 侧的节点
   - 递归 `replace_two_way` 处理更深层
   - `close` 用递归结果替换节点内容
3. 收集 `to` 之后的所有兄弟节点（`add_range(to, None, depth, ...)`）
4. 用 `Fragment::from_array` 将收集的节点包装为 Fragment

- [ ] **Step 2: 运行 `cargo test` 确认无编译错误无回归**

Run: `cargo test 2>&1 | tail -5`
Expected: `test result: ok. 366 passed; 0 failed`

- [ ] **Step 3: Commit**

```bash
git add src/model/replace.rs
git commit -m "feat(model): Phase 4.6 — replace_two_way recursive function"
```

---

### Task 2: `replace_three_way` 函数

**Files:**
- Modify: `src/model/replace.rs` (在 `replace_two_way` 之后追加)

- [ ] **Step 1: 实现 `replace_three_way`**

在 `replace_two_way` 函数之后追加：

```rust
/// Internal: build a new fragment by merging content around an inserted slice.
///
/// Four resolved positions define the boundaries:
/// - `from` / `to`: positions in the original document
/// - `start` / `end`: positions in the prepared slice (from `prepare_slice_for_replace`)
///
/// The function collects: [before from] + [open join or range] + [after to].
///
/// Corresponds to TS `replaceThreeWay($from, $start, $end, $to, depth)` in replace.ts:187-205.
fn replace_three_way(
    from: &ResolvedPos,
    start: &ResolvedPos,
    end: &ResolvedPos,
    to: &ResolvedPos,
    depth: usize,
) -> Result<Fragment, ReplaceError> {
    let open_start: Option<Node> = if from.depth > depth {
        Some(joinable(from, start, depth + 1)?)
    } else {
        None
    };
    let open_end: Option<Node> = if to.depth > depth {
        Some(joinable(end, to, depth + 1)?)
    } else {
        None
    };

    let mut content: Vec<Node> = Vec::new();
    add_range(None, Some(from), depth, &mut content);

    let both_open_same_index = open_start.is_some()
        && open_end.is_some()
        && start.index(Some(depth as i32)) == end.index(Some(depth as i32));

    if both_open_same_index {
        let os = open_start.as_ref().unwrap();
        let oe = open_end.as_ref().unwrap();
        check_join(os, oe)?;
        add_node(
            close(os, replace_three_way(from, start, end, to, depth + 1)?)?,
            &mut content,
        );
    } else {
        if let Some(ref os) = open_start {
            add_node(
                close(os, replace_two_way(from, start, depth + 1)?)?,
                &mut content,
            );
        }
        add_range(Some(start), Some(end), depth, &mut content);
        if let Some(ref oe) = open_end {
            add_node(
                close(oe, replace_two_way(end, to, depth + 1)?)?,
                &mut content,
            );
        }
    }

    add_range(Some(to), None, depth, &mut content);
    Ok(Fragment::from_array(&content))
}
```

**逻辑说明：**
1. 判断两侧是否有开放节点（`from.depth > depth` / `to.depth > depth`）
2. 收集 `from` 之前的兄弟节点
3. **关键分支：**
   - 如果两侧都开放且 `start`/`end` 指向同一子节点 → 递归 `replace_three_way`（保持四段合并）
   - 否则 → 分别用 `replace_two_way` 处理左侧开放和右侧开放，中间用 `add_range` 收集 slice 内容
4. 收集 `to` 之后的兄弟节点

- [ ] **Step 2: 运行 `cargo test` 确认无编译错误无回归**

Run: `cargo test 2>&1 | tail -5`
Expected: `test result: ok. 366 passed; 0 failed`

- [ ] **Step 3: Commit**

```bash
git add src/model/replace.rs
git commit -m "feat(model): Phase 4.6 — replace_three_way recursive function"
```

---

### Task 3: 更新阶段状态

**Files:**
- Modify: `docs/superpowers/plans/model-phases.md`

- [ ] **Step 1: 更新 Phase 4.6 状态为 ✅ 完成**

将 Phase 4.6 行的状态从 `⬜ 待开始` 改为 `✅ 完成`：

```
| Phase 4.6 | replace 算法积木块 | replace.rs 续（replace_two_way/replace_three_way） | Phase 4.5 | ✅ 完成 |
```

- [ ] **Step 2: Commit**

```bash
git add docs/superpowers/plans/model-phases.md
git commit -m "feat(model): Phase 4.6 — mark complete"
```

---

## 完成后状态

- `replace.rs` 包含所有 Phase 4.1–4.6 的内容：Slice + 8 个辅助函数 + 2 个递归算法
- `cargo test` 仍然 366 passed / 0 failed
- Phase 4.7 可以直接开始实现 `replace_outer`、`replace` 公开入口 + `Node::replace`/`Node::slice`
