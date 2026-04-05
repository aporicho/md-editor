# Phase 3.4-c: find_wrapping + Node Method Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 `ContentMatch::find_wrapping`、`Node::check`/`can_replace`/`can_replace_with`/`can_append`，补写 4 个 check 测试和 3 个 find_wrapping 测试，完成 prosemirror-model Phase 3.4-c。

**Architecture:** 所有新方法均为纯函数（无副作用），不需要 `schema` 参数（与 TS 实现对称）。`find_wrapping` 采用 BFS + 内部缓存（wrap_cache Mutex<HashMap>）。`check_content`/`check_attrs` 作为辅助方法添加到 NodeType/MarkType。

**Tech Stack:** Rust, `std::sync::{Arc, Mutex}`, `indexmap::IndexMap`, `serde_json::Value`

---

## 文件列表

| 操作 | 文件 | 说明 |
|------|------|------|
| Modify | `src/model/schema.rs` | 添加 `json_type_name` 辅助函数；添加 `NodeType::check_content`、`NodeType::check_attrs`、`MarkType::check_attrs` |
| Modify | `src/model/test_builder.rs` | 为 image.src 添加 string 类型验证器（供 check 测试使用） |
| Modify | `src/model/content.rs` | 替换 `find_wrapping_stub` → `find_wrapping`（先 todo!()，再实现）；添加模块级 `compute_wrapping` 辅助函数 |
| Modify | `src/model/node.rs` | 替换 `check_stub`/`can_replace_stub`/`can_replace_with_stub`/`can_append_stub` → 真实方法签名+实现 |
| Modify | `src/model/tests_node.rs` | 补写 4 个 check 测试 |
| Modify | `src/model/tests_content.rs` | 补写 3 个 find_wrapping 测试 |

---

## 关键已有 API（subagent 参考）

```
// content.rs
pub struct ContentMatch {
    pub valid_end: bool,
    edges: OnceLock<Vec<(Weak<NodeType>, Arc<ContentMatch>)>>,
    wrap_cache: Mutex<HashMap<String, Option<Vec<String>>>>,
}
// edges 访问：self.edges.get().map(|v| v.as_slice()).unwrap_or(&[])

// schema.rs
impl NodeType {
    pub is_block: bool
    pub is_text: bool
    pub fn is_inline(&self) -> bool { !self.is_block }
    pub fn is_leaf(&self) -> bool { content_match.edge_count()==0 && content_match.valid_end }
    pub fn has_required_attrs(&self) -> bool { ... }
    pub fn compatible_content(&self, other: &NodeType) -> bool { ... }
    pub fn allows_marks(&self, marks: &[Mark]) -> bool { ... }
    pub fn valid_content(&self, content: &Fragment) -> bool { ... }
    pub content_match: Arc<ContentMatch>
    pub spec: NodeSpec     // spec.attrs: IndexMap<String, AttrSpec>
    pub name: String
}

// node.rs
impl Node {
    pub fn content_match_at(&self, index: usize) -> Option<Arc<ContentMatch>>
    pub fn child_count(&self) -> usize
    pub content: Fragment
    pub node_type: Arc<NodeType>
    pub marks: Vec<Mark>
    pub attrs: Attrs  // Arc<HashMap<String, Value>>
}

// fragment.rs
impl Fragment {
    pub fn child(&self, index: usize) -> Result<&Node, RangeError>
    pub fn child_count(&self) -> usize
    pub fn to_string_repr(&self) -> String
}

// mark.rs
impl Mark {
    pub fn same_set(a: &[Mark], b: &[Mark]) -> bool
    pub fn add_to_set(&self, set: &[Mark]) -> Vec<Mark>
    pub mark_type: Arc<MarkType>
    pub attrs: Attrs
}

// schema.rs
pub enum AttrValidator {
    Types(Vec<String>),
    Func(AttrValidatorFn),
}
impl AttrSpec {
    pub validate: Option<AttrValidator>,
}
```

---

## Task 1: schema.rs 辅助方法 + test_builder.rs 验证器

**Files:**
- Modify: `src/model/schema.rs`
- Modify: `src/model/test_builder.rs`

- [ ] **Step 1: 在 schema.rs 添加 json_type_name 辅助函数**

在 `src/model/schema.rs` 中，在 `impl AttrSpec` 块结束之后（约第 121 行），添加以下私有函数：

```rust
/// 返回 serde_json Value 的 JavaScript-style 类型名，用于属性验证错误信息。
fn json_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
```

- [ ] **Step 2: 在 schema.rs 添加私有辅助函数 validate_attrs，避免 NodeType/MarkType 重复**

在 `json_type_name` 函数之后（约第 121 行）紧接着添加：

```rust
/// 验证 attrs 中每个字段是否符合 spec_attrs 的验证规则。
/// 供 NodeType::check_attrs 和 MarkType::check_attrs 共用，避免重复代码。
fn validate_attrs(
    spec_attrs: &IndexMap<String, AttrSpec>,
    attrs: &Attrs,
    type_name: &str,
) -> Result<(), crate::model::errors::RangeError> {
    for (attr_name, spec) in spec_attrs {
        let value = attrs.get(attr_name).unwrap_or(&Value::Null);
        match &spec.validate {
            Some(AttrValidator::Types(types)) => {
                let actual = json_type_name(value);
                if !types.iter().any(|t| t == actual) {
                    return Err(crate::model::errors::RangeError(format!(
                        "Expected value of type {} for attribute {} on type {}, got {}",
                        types.join(","),
                        attr_name,
                        type_name,
                        actual
                    )));
                }
            }
            Some(AttrValidator::Func(f)) => {
                if let Err(msg) = f(value) {
                    return Err(crate::model::errors::RangeError(msg));
                }
            }
            None => {}
        }
    }
    Ok(())
}
```

- [ ] **Step 3: 在 NodeType impl 块添加 check_content 和 check_attrs**

在 `src/model/schema.rs` 的 `impl NodeType` 块中，在 `valid_content` 方法之后（约第 514 行），添加：

```rust
/// 验证给定 fragment 是否为该节点类型的合法内容。
/// 若非法，返回 Err(RangeError("Invalid content for node {name}: ..."))。
pub fn check_content(
    &self,
    content: &crate::model::fragment::Fragment,
) -> Result<(), crate::model::errors::RangeError> {
    if !self.valid_content(content) {
        return Err(crate::model::errors::RangeError(format!(
            "Invalid content for node {}: {}",
            self.name,
            content.to_string_repr().chars().take(50).collect::<String>()
        )));
    }
    Ok(())
}

/// 验证属性值是否符合本节点类型的 AttrSpec 验证规则。
/// 错误格式：`Expected value of type {types} for attribute {attr} on type {name}, got {actual}`
pub fn check_attrs(
    &self,
    attrs: &crate::model::schema::Attrs,
) -> Result<(), crate::model::errors::RangeError> {
    validate_attrs(&self.spec.attrs, attrs, &self.name)
}
```

- [ ] **Step 4: 在 MarkType impl 块添加 check_attrs（复用 validate_attrs）**

在 `src/model/schema.rs` 的 `impl MarkType` 块中（约第 540 行之后），在 `has_required_attrs` 方法之后添加：

```rust
/// 验证 mark 的属性值是否合法。
pub fn check_attrs(
    &self,
    attrs: &crate::model::schema::Attrs,
) -> Result<(), crate::model::errors::RangeError> {
    validate_attrs(&self.spec.attrs, attrs, &self.name)
}
```

- [ ] **Step 5: 在 test_builder.rs 为 image.src 添加 string 类型验证器**

找到 `src/model/test_builder.rs` 中 image 节点的 attrs 定义，将 `src` 的 AttrSpec 改为带验证器：

在文件顶部的 use 语句中添加 `AttrValidator`（找到已有的 use 行）：

```rust
use crate::model::schema::{
    AttrSpec, AttrValidator, Attrs, MarkSpec, NodeSpec, Schema, SchemaSpec,
};
```

然后找到 image 的 src AttrSpec（当前 `validate: None`），将其改为：

```rust
"src".to_string() => AttrSpec {
    default: Some(json!("")),
    validate: Some(AttrValidator::Types(vec!["string".to_string()])),
},
```

**注意：** 只修改 `src` 的验证器，`alt` 和 `title` 保持 `validate: None`。

- [ ] **Step 6: 验证编译**

运行：`cargo build 2>&1 | tail -5`

预期：0 errors（可能有 warnings）

- [ ] **Step 7: Commit**

```bash
cd /Users/aporicho/Desktop/md-editor
git add src/model/schema.rs src/model/test_builder.rs
git commit -m "feat(model): add check_content/check_attrs helpers and image.src validator for Phase 3.4-c"
```

---

## Task 2: 替换 stubs 为真实签名（content.rs + node.rs）

**Files:**
- Modify: `src/model/content.rs`（约第 786 行）
- Modify: `src/model/node.rs`（约第 444-481 行）

- [ ] **Step 1: content.rs — 替换 find_wrapping_stub**

找到 `src/model/content.rs` 中 `pub fn find_wrapping_stub` 方法，将其（包括 `#[allow(unused)]` 属性行）整体替换为：

```rust
/// Find a set of wrapping node type names that would allow a node of the given type
/// to appear at this position.
///
/// Returns `Some([])` when the target fits directly (no wrapping needed),
/// `Some([wrapper1, wrapper2, ...])` when wrapping is needed,
/// or `None` when no such wrapping exists.
/// Result is cached in `wrap_cache` by target name.
pub fn find_wrapping(
    self: &Arc<Self>,
    target: &str,
) -> Option<Vec<String>> {
    todo!("find_wrapping: implement in Phase 3.4-c")
}
```

- [ ] **Step 2: node.rs — 替换 4 个 stubs**

找到 `src/model/node.rs` 中的 4 个 stub 方法，将其整体替换为带正确签名的 todo!() 实现：

```rust
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
```

保留 `slice_stub`, `replace_stub`, `resolve_stub` 不变。

- [ ] **Step 3: 验证编译**

运行：`cargo build 2>&1 | tail -5`

预期：0 errors

---

## Task 3: 写 TDD 测试（红阶段）

**Files:**
- Modify: `src/model/tests_node.rs`
- Modify: `src/model/tests_content.rs`

- [ ] **Step 1: 在 tests_node.rs 末尾添加 4 个 check 测试**

在 `src/model/tests_node.rs` 的末尾（最后一个测试之后、文件结束前）添加：

```rust
// ---- check tests ----

#[test]
fn check_notices_invalid_content() {
    // doc directly containing list_item (not in block group) → invalid
    use crate::model::fragment::Fragment;
    let s = test_schema();
    // li() builder creates list_item. We manually build doc with list_item child.
    let li_node = li(&[p(&[t("x")])]);
    let frag = Fragment::from_node(li_node);
    let doc_node = s.nodes["doc"].create(None, Some(frag), vec![]).unwrap();
    let result = doc_node.check();
    assert!(result.is_err(), "expected Err for invalid content");
    let err = result.unwrap_err();
    assert!(
        err.0.contains("Invalid content for node doc"),
        "unexpected error: {}",
        err.0
    );
}

#[test]
fn check_notices_marks_in_wrong_places() {
    // paragraph node with em mark directly on the node (not on inline children)
    // placed inside doc — doc doesn't allow marks on its children
    use crate::model::fragment::Fragment;
    let s = test_schema();
    let em_mark = s.marks["em"].create(&s.marks["em"], None).unwrap();
    // Create paragraph with no content but em mark on the node itself
    let para = s.nodes["paragraph"].create(None, None, vec![em_mark]).unwrap();
    let doc_node = s.nodes["doc"].create(None, Some(Fragment::from_node(para)), vec![]).unwrap();
    let result = doc_node.check();
    assert!(result.is_err(), "expected Err for marks in wrong place");
    let err = result.unwrap_err();
    assert!(
        err.0.contains("Invalid content for node doc"),
        "unexpected error: {}",
        err.0
    );
}

#[test]
fn check_notices_incorrect_mark_sets() {
    // text node with duplicate em marks [em, em] — should fail mark set validation
    let s = test_schema();
    let em1 = s.marks["em"].create(&s.marks["em"], None).unwrap();
    let em2 = s.marks["em"].create(&s.marks["em"], None).unwrap();
    let text_node = s.text("a".to_string(), vec![em1, em2]);
    let result = text_node.check();
    assert!(result.is_err(), "expected Err for duplicate marks");
    let err = result.unwrap_err();
    assert!(
        err.0.contains("Invalid collection of marks"),
        "unexpected error: {}",
        err.0
    );
}

#[test]
fn check_notices_wrong_attribute_types() {
    // image with src=true (boolean) — should fail string type validation
    use std::collections::HashMap;
    use serde_json::json;
    let s = test_schema();
    // Create image with src=true (wrong type: boolean instead of string)
    let mut attrs: HashMap<String, serde_json::Value> = HashMap::new();
    attrs.insert("src".to_string(), json!(true));
    attrs.insert("alt".to_string(), json!(""));
    attrs.insert("title".to_string(), json!(""));
    let img_node = s.nodes["image"].create(Some(&attrs), None, vec![]).unwrap();
    let result = img_node.check();
    assert!(result.is_err(), "expected Err for wrong attribute type");
    let err = result.unwrap_err();
    assert!(
        err.0.contains("Expected value of type"),
        "unexpected error: {}",
        err.0
    );
    assert!(
        err.0.contains("src"),
        "error should mention attribute name: {}",
        err.0
    );
}
```

- [ ] **Step 2: 在 tests_content.rs 的 mod tests 末尾添加 3 个 find_wrapping 测试**

在 `src/model/tests_content.rs` 的 `mod tests` 块末尾（`fill3_refuses_to_complete_overflown_count` 之后，`}` 之前）添加：

```rust
// -----------------------------------------------------------------------
// find_wrapping() tests (3)
// -----------------------------------------------------------------------

/// 从 doc 的 content match ("block+") 找 paragraph 的包裹：不需要包裹，返回 Some([])。
#[test]
fn find_wrapping_direct_fit() {
    let schema = test_schema();
    let doc_cm = &schema.nodes["doc"].content_match;
    // paragraph 在 block group 中，doc 的 content match 直接接受 paragraph
    let result = doc_cm.find_wrapping("paragraph");
    assert_eq!(result, Some(vec![]), "paragraph fits directly in doc, no wrapping needed");
}

/// 从 doc 的 content match ("block+") 找 list_item 的包裹：需要 bullet_list 包裹。
#[test]
fn find_wrapping_one_level() {
    let schema = test_schema();
    let doc_cm = &schema.nodes["doc"].content_match;
    // list_item 不在 block group 中，但 bullet_list（block group 成员）接受 list_item
    let result = doc_cm.find_wrapping("list_item");
    assert_eq!(
        result,
        Some(vec!["bullet_list".to_string()]),
        "list_item needs bullet_list as wrapper"
    );
}

/// 从 paragraph 的 content match ("inline*") 找 bullet_list 的包裹：不可能，返回 None。
#[test]
fn find_wrapping_impossible() {
    let schema = test_schema();
    let para_cm = &schema.nodes["paragraph"].content_match;
    // paragraph 只接受 inline 节点；所有 inline 节点都是叶节点，无法包裹 block 节点
    let result = para_cm.find_wrapping("bullet_list");
    assert!(result.is_none(), "bullet_list cannot be wrapped inside paragraph's inline context");
}
```

- [ ] **Step 3: 运行测试（红阶段）**

运行：`cargo test tests_content::tests::find_wrapping 2>&1 | head -20`

预期：编译成功，3 个测试因 todo!() 而 panic

运行：`cargo test tests_node::check_ 2>&1 | head -20`

预期：编译成功，4 个测试因 todo!() 而 panic

- [ ] **Step 4: Commit**

```bash
cd /Users/aporicho/Desktop/md-editor
git add src/model/tests_node.rs src/model/tests_content.rs
git commit -m "test(model): Phase 3.4-c — add 4 check tests and 3 find_wrapping tests (red)"
```

---

## Task 4: 实现 Node::check + can_replace + can_replace_with + can_append

**Files:**
- Modify: `src/model/node.rs`（约第 444-481 行，替换 4 个 todo!() 实现）

- [ ] **Step 1: 实现 check**

找到 `pub fn check(&self) -> Result<..., ...> { todo!(...) }`，将 todo!() 替换为：

```rust
pub fn check(&self) -> Result<(), crate::model::errors::RangeError> {
    self.node_type.check_content(&self.content)?;
    self.node_type.check_attrs(&self.attrs)?;
    // 验证 marks：构建规范化 mark set 并与实际 marks 比较
    let mut copy: Vec<crate::model::mark::Mark> = vec![];
    for mark in &self.marks {
        mark.mark_type.check_attrs(&mark.attrs)?;
        copy = mark.add_to_set(&copy);
    }
    if !crate::model::mark::Mark::same_set(&copy, &self.marks) {
        return Err(crate::model::errors::RangeError(format!(
            "Invalid collection of marks for node {}: {}",
            self.node_type.name,
            self.marks
                .iter()
                .map(|m| m.mark_type.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    // 递归检查子节点
    // 注意：i 始终在 0..child_count() 范围内，child() 不可能返回 Err，用 expect 表达此不变式
    for i in 0..self.content.child_count() {
        self.content
            .child(i)
            .expect("i < child_count() is always valid")
            .check()?;
    }
    Ok(())
}
```

- [ ] **Step 2: 实现 can_replace**

找到 `pub fn can_replace(&self, ...) -> bool { todo!(...) }`，替换为：

```rust
pub fn can_replace(
    &self,
    from: usize,
    to: usize,
    replacement: &crate::model::fragment::Fragment,
    start: usize,
    end: usize,
) -> bool {
    // 从 from 位置开始匹配 replacement[start..end]
    let one = self
        .content_match_at(from)
        .and_then(|m| m.match_fragment(replacement, start, end));
    // 再从匹配后的状态继续匹配剩余的原有内容 [to..]
    let two = one
        .as_ref()
        .and_then(|m| m.match_fragment(&self.content, to, self.child_count()));
    // 对应 TS: if (!two || !two.validEnd) return false
    if !two.is_some_and(|m| m.valid_end) {
        return false;
    }
    // 检查 replacement 的每个子节点的 marks 是否被允许
    for i in start..end {
        if let Ok(child) = replacement.child(i) {
            if !self.node_type.allows_marks(&child.marks) {
                return false;
            }
        }
    }
    true
}
```

- [ ] **Step 3: 实现 can_replace_with**

找到 `pub fn can_replace_with(&self, ...) -> bool { todo!(...) }`，替换为：

```rust
pub fn can_replace_with(
    &self,
    from: usize,
    to: usize,
    type_name: &str,
    marks: &[crate::model::mark::Mark],
) -> bool {
    // 若指定了 marks，先检查这些 marks 是否被本节点类型允许
    if !marks.is_empty() && !self.node_type.allows_marks(marks) {
        return false;
    }
    // 从 from 位置匹配 type_name
    let start = self
        .content_match_at(from)
        .and_then(|m| m.match_type(type_name));
    // 再从匹配后继续匹配剩余内容 [to..]
    let end = start
        .as_ref()
        .and_then(|m| m.match_fragment(&self.content, to, self.child_count()));
    end.map_or(false, |m| m.valid_end)
}
```

- [ ] **Step 4: 实现 can_append**

找到 `pub fn can_append(&self, other: &Node) -> bool { todo!(...) }`，替换为：

```rust
pub fn can_append(&self, other: &Node) -> bool {
    if other.content.size > 0 {
        // other 有内容：检查能否在末尾插入 other 的所有内容
        self.can_replace(
            self.child_count(),
            self.child_count(),
            &other.content,
            0,
            other.content.child_count(),
        )
    } else {
        // other 为空：检查两个节点的 content match 是否相容（共享至少一个允许的类型）
        self.node_type.compatible_content(&other.node_type)
    }
}
```

- [ ] **Step 5: 运行 check 测试**

运行：`cargo test check_ 2>&1 | tail -10`

预期：4 个 check 测试全部 PASS

若测试失败，常见原因：
- `check_notices_marks_in_wrong_places`：确认 doc 的 mark_set = Some([])（block 节点默认无 marks）
- `check_notices_incorrect_mark_sets`：确认 `s.text("a", vec![em1, em2])` 创建 marks=[em, em]，且 add_to_set 返回 [em]（去重）

- [ ] **Step 6: Commit**

```bash
cd /Users/aporicho/Desktop/md-editor
git add src/model/node.rs
git commit -m "feat(model): implement Node::check, can_replace, can_replace_with, can_append"
```

---

## Task 5: 实现 ContentMatch::find_wrapping

**Files:**
- Modify: `src/model/content.rs`

- [ ] **Step 1: 添加模块级 compute_wrapping 函数**

在 `src/model/content.rs` 中，在 `fill_before_search` 函数定义之后（约第 622 行之后，在 `pub struct ContentMatch` 之前），添加：

```rust
/// BFS 辅助函数：计算从 match_state 出发，让 target 类型能出现所需的最短包裹链。
///
/// - 返回 `Some(vec![])` 若 match_state 直接接受 target（无需包裹）。
/// - 返回 `Some(vec!["type1", "type2", ...])` 若需要包裹。
/// - 返回 `None` 若无法包裹。
///
/// BFS 对应 TS 的 computeWrapping 方法。
fn compute_wrapping(
    match_state: &Arc<ContentMatch>,
    target: &str,
) -> Option<Vec<String>> {
    struct Active {
        match_state: Arc<ContentMatch>,
        type_name: Option<String>,
        via: Option<usize>,
    }

    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut active: Vec<Active> = vec![Active {
        match_state: match_state.clone(),
        type_name: None,
        via: None,
    }];
    let mut head = 0usize;

    while head < active.len() {
        let current_match = active[head].match_state.clone();
        let has_type = active[head].type_name.is_some(); // 已进入至少一层包裹

        // 检查当前 match_state 是否直接接受 target
        if current_match.match_type(target).is_some() {
            // 沿 via 链回溯，收集包裹类型名，再 reverse
            let mut result: Vec<String> = Vec::new();
            let mut idx = head;
            while let Some(type_name) = &active[idx].type_name {
                result.push(type_name.clone());
                // via 指向父 Active 的索引
                match active[idx].via {
                    Some(via_idx) => idx = via_idx,
                    None => break,
                }
            }
            result.reverse();
            return Some(result);
        }

        // 探索当前 match_state 的出边
        let edges = current_match.edges.get().map(|v| v.as_slice()).unwrap_or(&[]);
        for (weak_nt, next_state) in edges {
            if let Some(nt) = weak_nt.upgrade() {
                // 跳过：叶节点 / 有必填属性 / 已见过 / 违反可关闭约束
                // (!current.type || next.validEnd): 若已在包裹内，要求本层能正常结束
                if !nt.is_leaf()
                    && !nt.has_required_attrs()
                    && !seen.contains(nt.name.as_str())
                    && (!has_type || next_state.valid_end)
                {
                    seen.insert(nt.name.clone());
                    let child_match = nt.content_match.clone();
                    active.push(Active {
                        match_state: child_match,
                        type_name: Some(nt.name.clone()),
                        via: Some(head),
                    });
                }
            }
        }

        head += 1;
    }
    None
}
```

- [ ] **Step 2: 替换 find_wrapping 的 todo!() 实现**

找到 `impl ContentMatch` 块中的 `find_wrapping` 方法（当前 todo!()），替换为：

```rust
pub fn find_wrapping(
    self: &Arc<Self>,
    target: &str,
) -> Option<Vec<String>> {
    // 先查缓存
    {
        let cache = self.wrap_cache.lock().unwrap();
        if let Some(cached) = cache.get(target) {
            return cached.clone();
        }
    }
    // 计算并写入缓存
    let result = compute_wrapping(self, target);
    {
        let mut cache = self.wrap_cache.lock().unwrap();
        cache.insert(target.to_string(), result.clone());
    }
    result
}
```

- [ ] **Step 3: 运行 find_wrapping 测试**

运行：`cargo test find_wrapping 2>&1 | tail -10`

预期：3 个 find_wrapping 测试全部 PASS

若 `find_wrapping_one_level` 失败，检查：
- `bullet_list` 在 test_schema 的 block group 中（`group: Some("block".to_string())`）
- `bullet_list` 不是叶节点（content = "list_item+"，有边）
- `bullet_list.content_match.match_type("list_item")` 返回 Some

若 `find_wrapping_direct_fit` 失败，检查：
- `paragraph` 在 block group 中 → doc 的 content match ("block+") 有 paragraph 边
- `doc_cm.match_type("paragraph")` = Some → compute_wrapping 第一轮即返回 Some([])

- [ ] **Step 4: 运行全量测试**

运行：`cargo test 2>&1 | tail -5`

预期：约 259 个测试全部 PASS（249 原有 + 4 check + 3 find_wrapping + 已有 2 个 schema tests from Task 1 compile pass）

- [ ] **Step 5: Commit**

```bash
cd /Users/aporicho/Desktop/md-editor
git add src/model/content.rs
git commit -m "feat(model): Phase 3.4-c — implement ContentMatch::find_wrapping"
```

---

## Task 6: 最终验证

**Files:** 仅运行命令

- [ ] **Step 1: 全量测试**

运行：`cargo test 2>&1 | tail -5`

预期：
```
test result: ok. 256 passed; 0 failed; 0 ignored; ...
```

（精确数量取决于 Task 1 中 check_attrs 是否触发已有测试的改动，但必须 0 failed）

- [ ] **Step 2: 确认 todo!() 仅剩 Phase 4 stub**

运行：`cargo build 2>&1 | grep "todo"` 或 `grep -n "todo!" src/model/node.rs`

预期：只剩 `slice_stub`, `replace_stub`, `resolve_stub` 为 todo!()，无 Phase 3.4-c 相关 todo!()。

---

## 自查

### Spec 覆盖

| 要求 | 实现位置 | 状态 |
|------|----------|------|
| `ContentMatch::find_wrapping` (BFS + wrap_cache) | Task 5 | ✅ |
| `NodeType::check_content` | Task 1 | ✅ |
| `NodeType::check_attrs` + `MarkType::check_attrs` | Task 1 | ✅ |
| `Node::check` | Task 4 | ✅ |
| `Node::can_replace` | Task 4 | ✅ |
| `Node::can_replace_with` | Task 4 | ✅ |
| `Node::can_append` | Task 4 | ✅ |
| 4 check 测试 (tests_node.rs) | Task 3 | ✅ |
| 3 find_wrapping 测试 (tests_content.rs) | Task 3 | ✅ |

### 类型一致性

- `check()` 签名：`&self -> Result<(), RangeError>`（无 schema 参数，与 TS 对称）
- `can_replace` 签名：`(&self, from, to, replacement: &Fragment, start, end) -> bool`（无 schema）
- `find_wrapping` 签名：`(self: &Arc<Self>, target: &str) -> Option<Vec<String>>`
- 所有测试中调用方式与实现签名一致

### 设计决策说明

`check`/`can_replace`/`can_replace_with`/`can_append` 均**不需要** `schema` 参数——Node 直接持有 `Arc<NodeType>`，方法通过 `content_match_at` 和 `allows_marks` 工作，均不需要 schema 查找。

`find_wrapping` 同样**不需要** `schema` 参数——`ContentMatch::edges` 已改为 `Weak<NodeType>`（Decision 28 重构），直接 `upgrade()` 获取 NodeType，无需外部传入 schema。

这与 TypeScript 原实现完全对称。
