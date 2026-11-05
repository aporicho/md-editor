# Phase 3.4-b: fill_before + create_and_fill Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 `ContentMatch::fill_before` 和 `NodeType::create_and_fill`，并补全 24 个 fillBefore 测试（TS 的 `test-content.ts` 中 `describe("fillBefore")` 块，共 17 个 `fill` + 7 个 `fill3` = 24 个测试）。

**Architecture:** `fill_before` 通过 DFS 遍历 ContentMatch 的 edges，找出需要插入的节点类型列表，然后对每个类型调用 `create_and_fill` 创建节点。两个函数均需要 `schema: &Arc<Schema>` 参数，因为 ContentMatch edges 只存类型名字符串，运行时需通过 schema.nodes 查 NodeType 属性。

**Tech Stack:** Rust, `std::sync::Arc`, `indexmap::IndexMap`, `serde_json::Value`

---

## 文件列表

| 操作 | 文件 | 说明 |
|------|------|------|
| Modify | `src/model/test_builder.rs` | 添加 `code_block` 节点类型到 test_schema，添加 `pre()` 构建函数 |
| Modify | `src/model/content.rs` | 删除 `fill_before_stub`；添加模块级 `fill_before_search`；添加 `ContentMatch::fill_before` |
| Modify | `src/model/schema.rs` | 删除 `create_and_fill_stub`；添加 `NodeType::create_and_fill` |
| Create | `src/model/tests_content.rs` | 24 个 fillBefore 测试（17 fill + 7 fill3）|
| Modify | `src/model/mod.rs` | 声明 `#[cfg(test)] mod tests_content;` |

---

## 关键数据结构与已有 API

```
// ContentMatch 结构（content.rs）
pub struct ContentMatch {
    pub valid_end: bool,
    pub inline_content: bool,
    edges: std::sync::OnceLock<Vec<(String, Arc<ContentMatch>)>>,
    wrap_cache: std::sync::Mutex<...>,
}

// 已有 API（可直接调用）：
// ContentMatch::match_fragment(self: &Arc<Self>, frag: &Fragment, start: usize, end: usize) -> Option<Arc<ContentMatch>>
// ContentMatch::parse(expr, node_types, groups, has_required) -> Result<Arc<ContentMatch>>
// Fragment::empty() -> Fragment
// Fragment::from_array(array: &[Node]) -> Fragment
// Fragment::append(&self, other: &Fragment) -> Fragment
// Fragment::child_count(&self) -> usize
// Fragment::eq(&self, other: &Fragment) -> bool  // 注意：非 PartialEq trait，是自定义方法
// NodeType::is_text: bool   // 字段
// NodeType::is_inline(&self) -> bool
// NodeType::has_required_attrs(&self) -> bool
// NodeType::create(self: &Arc<Self>, attrs, content, marks) -> Result<Node, SchemaError>
// NodeType::compute_attrs_for(attrs) -> Result<Attrs, SchemaError>
// Node::new(node_type, attrs, content, marks) -> Node
// Node.content: Fragment  // 公开字段
// Node.content_match: Arc<ContentMatch>  // 公开字段（在 NodeType 上）
// NodeType.content_match: Arc<ContentMatch>  // 公开字段
// Schema.nodes: IndexMap<String, Arc<NodeType>>  // 公开字段
```

---

## Task 1: 添加 code_block 到 test_builder.rs

**Files:**
- Modify: `src/model/test_builder.rs`

某些 fillBefore 测试用 `code_block` 节点（TS 的 `pre()`）。当前 test_schema 缺少该类型，需要补充。

- [ ] **Step 1: 在 test_schema 中添加 code_block**

在 `src/model/test_builder.rs` 的 `test_schema()` 函数中，在 `"text".to_string() => NodeSpec {...}` 条目之前，添加 code_block 定义：

```rust
"code_block".to_string() => NodeSpec {
    content: Some("text*".to_string()),
    group: Some("block".to_string()),
    ..NodeSpec::default()
},
```

具体定位：在 test_schema() 的 nodes indexmap 中，在 `"hard_break"` 之前添加（保持 block 节点和 inline 节点的分组顺序）。

修改后的相关部分（在 `"horizontal_rule"` 之后，`"image"` 之前插入）：

```rust
"horizontal_rule".to_string() => NodeSpec {
    group: Some("block".to_string()),
    ..NodeSpec::default()
},
"code_block".to_string() => NodeSpec {
    content: Some("text*".to_string()),
    group: Some("block".to_string()),
    ..NodeSpec::default()
},
"image".to_string() => NodeSpec {
```

- [ ] **Step 2: 添加 pre() 构建函数**

在 `src/model/test_builder.rs` 的 `// Block node builders` 区域，在 `hr()` 函数之后添加：

```rust
/// 创建 code_block 节点（对应 TS test-builder 的 pre()）。
pub fn pre(children: &[Node]) -> Node {
    make_node("code_block", empty_attrs(), children)
}
```

- [ ] **Step 3: 验证 test_builder 编译正常**

运行：`cargo test test_builder::tests::schema_builds_successfully 2>&1`

预期：PASS（schema 包含 code_block 节点）

- [ ] **Step 4: Commit**

```bash
git add src/model/test_builder.rs
git commit -m "test(model): add code_block to test_schema and pre() builder for Phase 3.4-b tests"
```

---

## Task 2: 添加编译用 stub 签名（正确签名 + todo!）

**Files:**
- Modify: `src/model/content.rs`
- Modify: `src/model/schema.rs`

在写测试前，先添加正确签名的占位方法，确保测试能编译（运行时 panic）。

- [ ] **Step 1: 在 content.rs 替换 fill_before_stub**

找到 `src/model/content.rs` 中的 `fill_before_stub` 方法（在 `ContentMatch` 的 impl 块内），将其替换为正确签名的 stub：

```rust
/// Try to match the given fragment, and if that fails, see if it can
/// be made to match by inserting nodes in front of it.
///
/// Returns a fragment of inserted nodes (may be empty). When `to_end` is true,
/// only returns if the resulting match goes to the end of the content expression.
pub fn fill_before(
    self: &Arc<Self>,
    schema: &Arc<crate::model::schema::Schema>,
    after: &crate::model::fragment::Fragment,
    to_end: bool,
    start_index: usize,
) -> Option<crate::model::fragment::Fragment> {
    todo!("fill_before: implement in Phase 3.4-b")
}
```

同时删除旧的 `find_wrapping_stub` 中的注释中对 `fill_before_stub` 的引用（若存在）。

- [ ] **Step 2: 在 schema.rs 替换 create_and_fill_stub**

找到 `src/model/schema.rs` 中的 `create_and_fill_stub` 方法（在 `impl NodeType` 块内），将其替换为：

```rust
/// Like `create`, but fill in missing content automatically.
///
/// If no fitting content can be constructed, returns `None`.
pub fn create_and_fill(
    self: &Arc<Self>,
    schema: &Arc<Schema>,
    attrs: Option<&std::collections::HashMap<String, serde_json::Value>>,
    marks: Vec<crate::model::mark::Mark>,
) -> Option<crate::model::node::Node> {
    todo!("create_and_fill: implement in Phase 3.4-b")
}
```

- [ ] **Step 3: 验证编译通过**

运行：`cargo build 2>&1 | head -20`

预期：编译成功（无错误，可能有 warnings）

---

## Task 3: 创建 tests_content.rs + 声明 mod（TDD 红阶段）

**Files:**
- Create: `src/model/tests_content.rs`
- Modify: `src/model/mod.rs`

测试文件包含 `get()`、`fill()`、`fill3()` 辅助函数以及 24 个测试。

- [ ] **Step 1: 在 mod.rs 添加测试模块声明**

在 `src/model/mod.rs` 末尾（`tests_node` 之后）添加：

```rust
#[cfg(test)]
mod tests_content;
```

- [ ] **Step 2: 创建 src/model/tests_content.rs**

创建文件，内容如下（完整代码）：

```rust
// src/model/tests_content.rs
//
// fillBefore tests ported from prosemirror-model/test/test-content.ts
// describe("fillBefore") block: 17 fill + 7 fill3 = 24 tests
//
#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use indexmap::IndexMap;

    use crate::model::content::ContentMatch;
    use crate::model::fragment::Fragment;
    use crate::model::test_builder::{
        test_schema, br, doc, h1, hr, img, p, pre,
    };

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Parse a content expression against the test_schema node types.
    fn get(expr: &str) -> Arc<ContentMatch> {
        let schema = test_schema();
        let is_inline: IndexMap<String, bool> = schema
            .nodes
            .iter()
            .map(|(n, nt)| (n.clone(), nt.is_inline()))
            .collect();
        let groups: IndexMap<String, Vec<String>> = {
            let mut m: IndexMap<String, Vec<String>> = IndexMap::new();
            for (name, nt) in &schema.nodes {
                for g in &nt.groups {
                    m.entry(g.clone()).or_insert_with(Vec::new).push(name.clone());
                }
            }
            m
        };
        let has_required: IndexMap<String, bool> = schema
            .nodes
            .iter()
            .map(|(n, nt)| (n.clone(), nt.has_required_attrs()))
            .collect();
        ContentMatch::parse(expr, &is_inline, &groups, &has_required)
            .unwrap_or_else(|e| panic!("content parse failed for '{}': {:?}", expr, e))
    }

    /// Port of TS fill():
    /// get(expr).matchFragment(before_content)!.fillBefore(after_content, true)
    /// then compare result to `expected` (or assert None).
    fn fill(expr: &str, before_content: Fragment, after_content: Fragment, expected: Option<Fragment>) {
        let schema = test_schema();
        let cm = get(expr);
        let matched = cm
            .match_fragment(&before_content, 0, before_content.child_count())
            .unwrap_or_else(|| panic!("fill: matchFragment failed for before_content in '{}'", expr));
        let filled = matched.fill_before(schema, &after_content, true, 0);
        match expected {
            Some(exp) => {
                let filled = filled
                    .unwrap_or_else(|| panic!("fill: expected Some but got None for '{}'", expr));
                assert!(
                    filled.eq(&exp),
                    "fill '{}': result mismatch — got size {}, expected size {}",
                    expr,
                    filled.size,
                    exp.size
                );
            }
            None => assert!(
                filled.is_none(),
                "fill '{}': expected None but got Some",
                expr
            ),
        }
    }

    /// Port of TS fill3():
    /// a = get(expr).matchFragment(before)!.fillBefore(mid, false)
    /// b = a && get(expr).matchFragment(before.append(a).append(mid))!.fillBefore(after, true)
    fn fill3(
        expr: &str,
        before: Fragment,
        mid: Fragment,
        after: Fragment,
        expected_left: Option<Fragment>,
        expected_right: Option<Fragment>,
    ) {
        let schema = test_schema();
        let cm = get(expr);
        let a = cm
            .match_fragment(&before, 0, before.child_count())
            .unwrap_or_else(|| panic!("fill3: matchFragment failed for `before` in '{}'", expr))
            .fill_before(schema, &mid, false, 0);
        let b = a.as_ref().and_then(|fill_a| {
            let combined = before.append(fill_a).append(&mid);
            cm.match_fragment(&combined, 0, combined.child_count())
                .unwrap_or_else(|| panic!("fill3: matchFragment failed for combined in '{}'", expr))
                .fill_before(schema, &after, true, 0)
        });
        match expected_left {
            Some(left) => {
                let a = a.unwrap_or_else(|| panic!("fill3 '{}': a should be Some", expr));
                assert!(
                    a.eq(&left),
                    "fill3 '{}': a mismatch — got size {}, expected size {}",
                    expr,
                    a.size,
                    left.size
                );
                let right =
                    expected_right.unwrap_or_else(|| panic!("fill3 '{}': expected_right is None when expected_left is Some", expr));
                let b = b.unwrap_or_else(|| panic!("fill3 '{}': b should be Some", expr));
                assert!(
                    b.eq(&right),
                    "fill3 '{}': b mismatch — got size {}, expected size {}",
                    expr,
                    b.size,
                    right.size
                );
            }
            None => assert!(b.is_none(), "fill3 '{}': b should be None", expr),
        }
    }

    // -----------------------------------------------------------------------
    // fill() tests (17)
    // -----------------------------------------------------------------------

    #[test]
    fn fill_returns_empty_when_things_match() {
        fill(
            "paragraph horizontal_rule paragraph",
            doc(&[p(&[]), hr()]).content,
            doc(&[p(&[])]).content,
            Some(doc(&[]).content),
        )
    }

    #[test]
    fn fill_adds_a_node_when_necessary() {
        fill(
            "paragraph horizontal_rule paragraph",
            doc(&[p(&[])]).content,
            doc(&[p(&[])]).content,
            Some(doc(&[hr()]).content),
        )
    }

    #[test]
    fn fill_accepts_asterisk_across_the_bound() {
        fill(
            "hard_break*",
            p(&[br()]).content,
            p(&[br()]).content,
            Some(p(&[]).content),
        )
    }

    #[test]
    fn fill_accepts_asterisk_only_on_the_left() {
        fill(
            "hard_break*",
            p(&[br()]).content,
            p(&[]).content,
            Some(p(&[]).content),
        )
    }

    #[test]
    fn fill_accepts_asterisk_only_on_the_right() {
        fill(
            "hard_break*",
            p(&[]).content,
            p(&[br()]).content,
            Some(p(&[]).content),
        )
    }

    #[test]
    fn fill_accepts_asterisk_with_no_elements() {
        fill(
            "hard_break*",
            p(&[]).content,
            p(&[]).content,
            Some(p(&[]).content),
        )
    }

    #[test]
    fn fill_accepts_plus_across_the_bound() {
        fill(
            "hard_break+",
            p(&[br()]).content,
            p(&[br()]).content,
            Some(p(&[]).content),
        )
    }

    #[test]
    fn fill_adds_element_for_content_less_plus() {
        fill(
            "hard_break+",
            p(&[]).content,
            p(&[]).content,
            Some(p(&[br()]).content),
        )
    }

    #[test]
    fn fill_fails_for_mismatched_plus() {
        fill(
            "hard_break+",
            p(&[]).content,
            p(&[img()]).content,
            None,
        )
    }

    #[test]
    fn fill_accepts_asterisk_with_content_on_both_sides() {
        fill(
            "heading* paragraph*",
            doc(&[h1(&[])]).content,
            doc(&[p(&[])]).content,
            Some(doc(&[]).content),
        )
    }

    #[test]
    fn fill_accepts_asterisk_with_no_content_after() {
        fill(
            "heading* paragraph*",
            doc(&[h1(&[])]).content,
            doc(&[]).content,
            Some(doc(&[]).content),
        )
    }

    #[test]
    fn fill_accepts_plus_with_content_on_both_sides() {
        fill(
            "heading+ paragraph+",
            doc(&[h1(&[])]).content,
            doc(&[p(&[])]).content,
            Some(doc(&[]).content),
        )
    }

    #[test]
    fn fill_accepts_plus_with_no_content_after() {
        fill(
            "heading+ paragraph+",
            doc(&[h1(&[])]).content,
            doc(&[]).content,
            Some(doc(&[p(&[])]).content),
        )
    }

    #[test]
    fn fill_adds_elements_to_match_a_count() {
        fill(
            "hard_break{3}",
            p(&[br()]).content,
            p(&[br()]).content,
            Some(p(&[br()]).content),
        )
    }

    #[test]
    fn fill_fails_when_too_many_elements() {
        fill(
            "hard_break{3}",
            p(&[br(), br()]).content,
            p(&[br(), br()]).content,
            None,
        )
    }

    #[test]
    fn fill_adds_elements_for_two_counted_groups() {
        fill(
            "code_block{2} paragraph{2}",
            doc(&[pre(&[])]).content,
            doc(&[p(&[])]).content,
            Some(doc(&[pre(&[]), p(&[])]).content),
        )
    }

    #[test]
    fn fill_does_not_include_optional_elements() {
        fill(
            "heading paragraph? horizontal_rule",
            doc(&[h1(&[])]).content,
            doc(&[]).content,
            Some(doc(&[hr()]).content),
        )
    }

    // -----------------------------------------------------------------------
    // fill3() tests (7)
    // -----------------------------------------------------------------------

    #[test]
    fn fill3_completes_a_sequence() {
        fill3(
            "paragraph horizontal_rule paragraph horizontal_rule paragraph",
            doc(&[p(&[])]).content,
            doc(&[p(&[])]).content,
            doc(&[p(&[])]).content,
            Some(doc(&[hr()]).content),
            Some(doc(&[hr()]).content),
        )
    }

    #[test]
    fn fill3_accepts_plus_across_two_bounds() {
        fill3(
            "code_block+ paragraph+",
            doc(&[pre(&[])]).content,
            doc(&[pre(&[])]).content,
            doc(&[p(&[])]).content,
            Some(doc(&[]).content),
            Some(doc(&[]).content),
        )
    }

    #[test]
    fn fill3_fills_a_plus_from_empty_input() {
        fill3(
            "code_block+ paragraph+",
            doc(&[]).content,
            doc(&[]).content,
            doc(&[]).content,
            Some(doc(&[]).content),
            Some(doc(&[pre(&[]), p(&[])]).content),
        )
    }

    #[test]
    fn fill3_completes_a_count() {
        fill3(
            "code_block{3} paragraph{3}",
            doc(&[pre(&[])]).content,
            doc(&[p(&[])]).content,
            doc(&[]).content,
            Some(doc(&[pre(&[]), pre(&[])]).content),
            Some(doc(&[p(&[]), p(&[])]).content),
        )
    }

    #[test]
    fn fill3_fails_on_non_matching_elements() {
        fill3(
            "paragraph*",
            doc(&[p(&[])]).content,
            doc(&[pre(&[])]).content,
            doc(&[p(&[])]).content,
            None,
            None,
        )
    }

    #[test]
    fn fill3_completes_a_plus_across_two_bounds() {
        fill3(
            "paragraph{4}",
            doc(&[p(&[])]).content,
            doc(&[p(&[])]).content,
            doc(&[p(&[])]).content,
            Some(doc(&[]).content),
            Some(doc(&[p(&[])]).content),
        )
    }

    #[test]
    fn fill3_refuses_to_complete_overflown_count() {
        fill3(
            "paragraph{2}",
            doc(&[p(&[])]).content,
            doc(&[p(&[])]).content,
            doc(&[p(&[])]).content,
            None,
            None,
        )
    }
}
```

- [ ] **Step 3: 验证测试编译并 panic（红阶段）**

运行：`cargo test tests_content 2>&1 | head -40`

预期：编译成功，但测试因 `todo!()` panic（"not yet implemented"）

- [ ] **Step 4: Commit**

```bash
git add src/model/tests_content.rs src/model/mod.rs src/model/content.rs src/model/schema.rs
git commit -m "test(model): Phase 3.4-b — add tests_content.rs with 24 fillBefore tests (red)"
```

---

## Task 4: 实现 NodeType::create_and_fill（schema.rs）

**Files:**
- Modify: `src/model/schema.rs:489-495`（替换 `create_and_fill_stub`）

`create_and_fill` 逻辑（TS 参考去掉 content 参数简化版）：
1. 计算 attrs（调用 `compute_attrs_for`）
2. 以空 Fragment 为起点，匹配 content_match（从索引 0 到 0，返回 content_match 本身）
3. 对 matched 状态调用 `fill_before(schema, Fragment::empty(), to_end=true, start_index=0)`
4. 返回 `Node::new(self, attrs, empty.append(after), marks)`

- [ ] **Step 1: 替换 create_and_fill 实现**

找到 `src/model/schema.rs` 中的 `create_and_fill` 方法（刚才在 Task 2 添加的 stub），将其完整替换为：

```rust
/// Like `create`, but fill in missing content automatically.
///
/// Starts with empty content and uses fill_before to produce the minimal
/// required content. Returns None if no valid content can be constructed
/// (e.g., type requires attributes without defaults).
pub fn create_and_fill(
    self: &Arc<Self>,
    schema: &Arc<Schema>,
    attrs: Option<&std::collections::HashMap<String, serde_json::Value>>,
    marks: Vec<crate::model::mark::Mark>,
) -> Option<crate::model::node::Node> {
    if self.is_text {
        return None;
    }
    let attrs = self.compute_attrs_for(attrs).ok()?;
    let content = crate::model::fragment::Fragment::empty();
    // match_fragment(empty, 0, 0) always returns Some(content_match) — no iterations needed
    let matched = self.content_match.match_fragment(&content, 0, 0)?;
    let after = matched.fill_before(
        schema,
        &crate::model::fragment::Fragment::empty(),
        true,
        0,
    )?;
    Some(crate::model::node::Node::new(
        self.clone(),
        attrs,
        content.append(&after),
        marks,
    ))
}
```

- [ ] **Step 2: 验证编译通过**

运行：`cargo build 2>&1 | head -20`

预期：编译成功

---

## Task 5: 实现 ContentMatch::fill_before（content.rs）

**Files:**
- Modify: `src/model/content.rs`

添加模块级辅助函数 `fill_before_search`（不在 `impl` 块内），然后实现 `fill_before` 方法。

**算法说明（对应 TS fillBefore）：**
```
fillBefore(after, toEnd=false, startIndex=0):
  seen = [this]
  search(match, types):
    finished = match.matchFragment(after, startIndex)
    if finished && (!toEnd || finished.validEnd):
      return Fragment.from(types.map(tp => tp.createAndFill()))
    for each (type_name, next) in match.edges:
      if !type.isText && !type.hasRequiredAttrs() && next NOT in seen:
        seen.push(next)
        found = search(next, types + [type_name])
        if found: return found
    return null
  return search(this, [])
```

- [ ] **Step 1: 在 content.rs 顶部添加必要的 use**

在 `src/model/content.rs` 文件顶部（`#[allow(unused_imports)]` 区域之后，`use crate::model::errors` 之后），确认以下 use 语句存在或添加：

```rust
// 已有的：
use std::sync::Arc;
// 无需额外添加：crate::model::schema、fragment、node 通过完整路径引用
```

（`fill_before_search` 和 `fill_before` 将使用完整路径 `crate::model::schema::Schema` 等来避免循环 use 问题。）

- [ ] **Step 2: 添加 fill_before_search 模块级函数**

在 `src/model/content.rs` 中，在 `impl ContentMatch {` 块之前（约第 590 行），添加以下模块级函数：

```rust
/// DFS 辅助函数，搜索需要在 `after` 前插入的节点类型列表。
///
/// - `match_state`: 当前 ContentMatch 状态
/// - `types`: 当前积累的待插入类型名列表
/// - `seen`: 已访问的 ContentMatch 状态（防止循环）
/// - `schema`: 用于查询 NodeType 属性
/// - `after`: 需要在结果之后出现的 Fragment
/// - `to_end`: 若为 true，要求最终能到达 valid_end 状态
/// - `start_index`: after Fragment 中的起始索引
fn fill_before_search(
    match_state: &Arc<ContentMatch>,
    types: &[String],
    seen: &mut Vec<Arc<ContentMatch>>,
    schema: &Arc<crate::model::schema::Schema>,
    after: &crate::model::fragment::Fragment,
    to_end: bool,
    start_index: usize,
) -> Option<crate::model::fragment::Fragment> {
    // 尝试从当前状态匹配 after[start_index..]
    if let Some(finished) = match_state.match_fragment(after, start_index, after.child_count()) {
        // 若不需要 to_end，或已到达 valid_end，则当前 types 就是插入列表
        if !to_end || finished.valid_end {
            // 对每个 type_name 调用 create_and_fill；任何一个失败则整体失败
            let nodes: Option<Vec<crate::model::node::Node>> = types
                .iter()
                .map(|type_name| {
                    let nt = schema.nodes.get(type_name)?;
                    nt.create_and_fill(schema, None, vec![])
                })
                .collect();
            if let Some(nodes) = nodes {
                return Some(crate::model::fragment::Fragment::from_array(&nodes));
            }
        }
    }

    // 遍历当前状态的 outgoing edges，寻找可插入的类型
    let edges = match_state.edges.get().map(|v| v.as_slice()).unwrap_or(&[]);
    for (type_name, next_state) in edges {
        if let Some(nt) = schema.nodes.get(type_name) {
            // 跳过 text 节点和有必填属性的节点（无法自动创建）
            if !nt.is_text
                && !nt.has_required_attrs()
                && !seen.iter().any(|s| Arc::ptr_eq(s, next_state))
            {
                seen.push(next_state.clone());
                let mut new_types = types.to_vec();
                new_types.push(type_name.clone());
                if let Some(found) = fill_before_search(
                    next_state,
                    &new_types,
                    seen,
                    schema,
                    after,
                    to_end,
                    start_index,
                ) {
                    return Some(found);
                }
            }
        }
    }
    None
}
```

- [ ] **Step 3: 在 impl ContentMatch 中替换 fill_before stub**

找到 `src/model/content.rs` 中 `impl ContentMatch` 块内的 `fill_before` stub（Task 2 添加的），将其替换为真实实现：

```rust
/// Try to match the given fragment, and if that fails, see if it can
/// be made to match by inserting nodes in front of it.
///
/// Returns a fragment of inserted nodes (may be empty if nothing needs
/// to be inserted). When `to_end` is true, only returns a fragment if
/// the resulting match goes to the end of the content expression.
pub fn fill_before(
    self: &Arc<Self>,
    schema: &Arc<crate::model::schema::Schema>,
    after: &crate::model::fragment::Fragment,
    to_end: bool,
    start_index: usize,
) -> Option<crate::model::fragment::Fragment> {
    let mut seen: Vec<Arc<ContentMatch>> = vec![self.clone()];
    fill_before_search(self, &[], &mut seen, schema, after, to_end, start_index)
}
```

- [ ] **Step 4: 运行测试（绿阶段）**

运行：`cargo test tests_content 2>&1`

预期：所有 24 个 tests_content 测试 PASS

若有测试失败，检查对应测试用例：
- 比较 result.size vs expected.size 来定位问题
- 确认 `code_block` 在 test_schema 的 `block` group 中（影响 "code_block+ paragraph+" 测试）
- 确认 `fill_before_search` 的 `!to_end || finished.valid_end` 条件正确

- [ ] **Step 5: 运行全量测试**

运行：`cargo test 2>&1 | tail -10`

预期：所有现有测试（225 个）+ 新增 24 个 = 约 249 个全部 PASS

- [ ] **Step 6: Commit**

```bash
git add src/model/content.rs src/model/schema.rs
git commit -m "feat(model): Phase 3.4-b — implement fill_before and create_and_fill"
```

---

## Task 6: 最终验证

**Files:** None（只运行命令）

- [ ] **Step 1: 全量测试通过**

运行：`cargo test 2>&1 | tail -5`

预期：
```
test result: ok. 249 passed; 0 failed; 0 ignored; ...
```

（精确数字取决于现有测试数，但必须 0 failed）

- [ ] **Step 2: 无新增 warnings（可选）**

运行：`cargo build 2>&1 | grep "^warning" | wc -l`

对比 Phase 3.4-a 完成时的 warning 数，确认没有显著增加。

---

## 自查备忘

**实现时的易错点：**

1. `fill_before_search` 中的条件：`if !to_end || finished.valid_end`
   - 当 `to_end = false` 时，`!to_end = true`，条件为 `true`，只要 `match_fragment` 返回 Some 就立即返回（不管 valid_end）
   - 当 `to_end = true` 时，必须 `valid_end = true` 才返回

2. `seen` 追踪用 `Arc::ptr_eq`（指针相等，不是内容相等）

3. `create_and_fill` 中 `match_fragment(&empty, 0, 0)` 返回 `Some(content_match)` 自身（迭代 0 次，返回 `self.clone()`）

4. `Fragment::eq` 是自定义方法（不是 PartialEq trait），测试中用 `assert!(a.eq(&b))`

5. `code_block` 需要在 test_schema 的 `"block"` group 中，这样 `fill_before` 在处理 `"code_block+ paragraph+"` 时能找到该类型

6. `fill3` helper 中 `before.append(fill_a)` 的类型：`fill_a` 是 `&Fragment`（来自 `a.as_ref()`），`Fragment::append` 接受 `&Fragment` ✓
