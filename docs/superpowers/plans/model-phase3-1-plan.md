# model Phase 3.1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 `Mark` struct 及全部方法，同时补全 Phase 2.2 遗留的 MarkType stubs（`create`、`remove_from_set`、`is_in_set`），并建立对应的 `tests_mark.rs` 测试文件。

**Architecture:** 新增 `src/model/mark.rs`（Mark struct + 所有方法）；在 `src/model/schema.rs` 中将 `MarkType` 的三个 `_stub` 方法替换为真实实现。Mark 依赖 MarkType（Phase 2.2 ✅）、Schema（Phase 2.3 ✅）、compare_deep（Phase 1 ✅）。无 Fragment/Node 依赖，可独立完成。

**Tech Stack:** Rust 2024 edition, `serde_json`, `indexmap`, Phase 1 的 `compare_deep`

**TS 参考：** `reference/prosemirror-model/src/mark.ts`

---

## 文件结构

```
src/model/mark.rs           ← 新建：Mark struct + 所有方法
src/model/tests_mark.rs     ← 新建：对应 test-mark.ts 的 Rust 测试
src/model/mod.rs            ← 修改：声明 mark 和 tests_mark 模块
src/model/schema.rs         ← 修改：将 MarkType 的三个 _stub 替换为真实实现
```

---

## 背景

已有（不修改）：
- `MarkType` struct（`name`, `rank`, `inclusive`, `excluded: ExcludeSpec`, `spec: MarkSpec`, `default_attrs`）
- `MarkType::excludes(&self, other: &MarkType) -> bool`（通过 ExcludeSpec）
- `compute_attrs(specs, values) -> Result<Attrs, SchemaError>`
- `Schema::new(spec) -> Result<Arc<Schema>, SchemaError>`

Mark::eq 的逻辑：
- 指针相等（同一实例）→ true
- 同类型（MarkType 地址相等）AND 相同 attrs（HashMap 相等）→ true
- 否则 false

## 关键设计

```rust
// mark.rs

pub struct Mark {
    pub mark_type: Arc<MarkType>,
    pub attrs: Attrs,      // Arc<HashMap<String, Value>>
}

impl Mark {
    pub const NONE: &'static [Self] = &[];

    // 实例方法
    pub fn eq(&self, other: &Mark) -> bool
    pub fn add_to_set(&self, set: &[Mark]) -> Vec<Mark>
    pub fn remove_from_set(&self, set: &[Mark]) -> Vec<Mark>
    pub fn is_in_set(&self, set: &[Mark]) -> bool
    pub fn to_json(&self) -> serde_json::Value

    // 关联函数
    pub fn from_json(schema: &Schema, json: &serde_json::Value) -> Result<Self, RangeError>
    pub fn same_set(a: &[Mark], b: &[Mark]) -> bool
    pub fn set_from(marks: Option<&[Mark]>) -> Vec<Mark>
}

impl PartialEq for Mark { fn eq(&self, other: &Self) -> bool { self.eq(other) } }
impl Eq for Mark {}
impl Clone for Mark { fn clone(&self) -> Self { Mark { mark_type: self.mark_type.clone(), attrs: self.attrs.clone() } } }
```

---

## Task 1：mark.rs 骨架 + mod.rs 声明

**Files:**
- Create: `src/model/mark.rs`
- Modify: `src/model/mod.rs`

- [ ] **Step 1: 创建 mark.rs 骨架**

```rust
// src/model/mark.rs
//
// Mark：附加在节点上的样式信息（如加粗、链接等）。
// 对应 TypeScript 的 mark.ts。
//
use std::sync::Arc;
use std::collections::HashMap;
use serde_json::Value;

use crate::model::schema::{MarkType, Schema, Attrs};
use crate::model::errors::RangeError;
use crate::model::compare_deep::compare_deep;

/// A mark is a piece of information that can be attached to a node,
/// such as it being emphasized, in code font, or a link. It has a
/// type and optionally a set of attributes that provide further
/// information (such as the target of the link). Marks are created
/// through a `Schema`, which controls which types exist and which
/// attributes they have.
#[derive(Clone)]
pub struct Mark {
    /// The type of this mark.
    pub mark_type: Arc<MarkType>,
    /// The attributes associated with this mark.
    pub attrs: Attrs,
}

impl Mark {
    /// The empty set of marks.
    pub const NONE: &'static [Self] = &[];
}

impl PartialEq for Mark {
    fn eq(&self, other: &Self) -> bool {
        self.eq_mark(other)
    }
}

impl Eq for Mark {}
```

- [ ] **Step 2: 在 mod.rs 声明模块**

```rust
// src/model/mod.rs（在已有 pub mod schema; 之后添加）
pub mod mark;
```

- [ ] **Step 3: 验证编译**

```bash
cargo check
```

Expected: 无报错（`eq_mark` 未定义会有 error，下一步修复）

实际上骨架中 `self.eq_mark(other)` 还不存在，先写为 `false`：

```rust
impl PartialEq for Mark {
    fn eq(&self, other: &Self) -> bool {
        false  // placeholder, replaced in Task 2
    }
}
```

- [ ] **Step 4: Commit**

```bash
git add src/model/mark.rs src/model/mod.rs
git commit -m "feat(model): add Mark struct skeleton"
```

---

## Task 2：Mark::eq + Mark::NONE + tests_mark.rs（第一批测试）

**Files:**
- Modify: `src/model/mark.rs`
- Create: `src/model/tests_mark.rs`
- Modify: `src/model/mod.rs`

- [ ] **Step 1: 创建 tests_mark.rs 测试辅助**

```rust
// src/model/tests_mark.rs
use std::sync::Arc;
use indexmap::indexmap;
use serde_json::json;

use crate::model::mark::Mark;
use crate::model::schema::{Schema, SchemaSpec, NodeSpec, MarkSpec, AttrSpec, Attrs};
use crate::model::errors::RangeError;

/// 构建测试用 Schema（含 em, strong, link, code, remark 等 mark 类型）
fn test_schema() -> Arc<Schema> {
    Schema::new(SchemaSpec {
        nodes: indexmap! {
            "doc".to_string() => NodeSpec {
                content: Some("paragraph+".to_string()),
                ..NodeSpec::default()
            },
            "paragraph".to_string() => NodeSpec {
                content: Some("text*".to_string()),
                ..NodeSpec::default()
            },
            "text".to_string() => NodeSpec { inline: true, ..NodeSpec::default() },
        },
        marks: indexmap! {
            "em".to_string() => MarkSpec::default(),
            "strong".to_string() => MarkSpec::default(),
            "link".to_string() => MarkSpec {
                attrs: indexmap! {
                    "href".to_string() => AttrSpec { default: None, validate: None },
                    "title".to_string() => AttrSpec { default: Some(json!(null)), validate: None },
                },
                ..MarkSpec::default()
            },
            "code".to_string() => MarkSpec::default(),
            "remark".to_string() => MarkSpec {
                attrs: indexmap! {
                    "id".to_string() => AttrSpec { default: None, validate: None },
                },
                excludes: Some("".to_string()), // excludes nothing (non-exclusive)
                inclusive: false,
                ..MarkSpec::default()
            },
        },
        top_node: None,
    }).unwrap()
}

/// 创建 em mark（无 attrs）
fn mk_em(s: &Arc<Schema>) -> Mark {
    s.marks["em"].create(
        &s.marks["em"], None
    ).unwrap()
}

fn mk_strong(s: &Arc<Schema>) -> Mark {
    s.marks["strong"].create(
        &s.marks["strong"], None
    ).unwrap()
}

fn mk_code(s: &Arc<Schema>) -> Mark {
    s.marks["code"].create(
        &s.marks["code"], None
    ).unwrap()
}

fn mk_link(s: &Arc<Schema>, href: &str, title: Option<&str>) -> Mark {
    use std::collections::HashMap;
    let mut attrs: HashMap<String, serde_json::Value> = HashMap::new();
    attrs.insert("href".to_string(), json!(href));
    if let Some(t) = title {
        attrs.insert("title".to_string(), json!(t));
    }
    s.marks["link"].create(
        &s.marks["link"], Some(&attrs)
    ).unwrap()
}

fn mk_remark(s: &Arc<Schema>, id: i64) -> Mark {
    use std::collections::HashMap;
    let mut attrs: HashMap<String, serde_json::Value> = HashMap::new();
    attrs.insert("id".to_string(), json!(id));
    s.marks["remark"].create(
        &s.marks["remark"], Some(&attrs)
    ).unwrap()
}

// ---- eq 测试 ----

#[test]
fn mark_eq_same_instance_via_clone() {
    let s = test_schema();
    let em = mk_em(&s);
    assert!(em.eq_mark(&em.clone()));
}

#[test]
fn mark_eq_identical_links() {
    let s = test_schema();
    assert!(mk_link(&s, "http://foo", None).eq_mark(&mk_link(&s, "http://foo", None)));
}

#[test]
fn mark_eq_different_links() {
    let s = test_schema();
    assert!(!mk_link(&s, "http://foo", None).eq_mark(&mk_link(&s, "http://bar", None)));
}

#[test]
fn mark_eq_links_different_titles() {
    let s = test_schema();
    assert!(!mk_link(&s, "http://foo", Some("A")).eq_mark(&mk_link(&s, "http://foo", Some("B"))));
}

// ---- same_set 测试 ----

#[test]
fn same_set_empty() {
    assert!(Mark::same_set(&[], &[]));
}

#[test]
fn same_set_identical() {
    let s = test_schema();
    let em = mk_em(&s);
    let strong = mk_strong(&s);
    assert!(Mark::same_set(&[em.clone(), strong.clone()], &[em, strong]));
}

#[test]
fn same_set_different() {
    let s = test_schema();
    assert!(!Mark::same_set(&[mk_em(&s), mk_strong(&s)], &[mk_em(&s), mk_code(&s)]));
}

#[test]
fn same_set_different_size() {
    let s = test_schema();
    assert!(!Mark::same_set(
        &[mk_em(&s), mk_strong(&s)],
        &[mk_em(&s), mk_strong(&s), mk_code(&s)],
    ));
}

#[test]
fn same_set_identical_links() {
    let s = test_schema();
    assert!(Mark::same_set(
        &[mk_link(&s, "http://foo", None), mk_code(&s)],
        &[mk_link(&s, "http://foo", None), mk_code(&s)],
    ));
}

#[test]
fn same_set_different_links() {
    let s = test_schema();
    assert!(!Mark::same_set(
        &[mk_link(&s, "http://foo", None), mk_code(&s)],
        &[mk_link(&s, "http://bar", None), mk_code(&s)],
    ));
}
```

- [ ] **Step 2: 在 mod.rs 中声明 tests_mark 模块**

```rust
// src/model/mod.rs（追加）
#[cfg(test)]
mod tests_mark;
```

- [ ] **Step 3: 运行测试，确认失败**

```bash
cargo test -p md-editor tests_mark
```

Expected: FAIL（`eq_mark` 未定义，`Mark::same_set` 未定义，`MarkType::create` 未实现）

- [ ] **Step 4: 在 mark.rs 中实现 eq_mark + PartialEq + same_set**

```rust
impl Mark {
    // ... NONE 常量已有 ...

    /// Test whether this mark has the same type and attributes as another mark.
    ///
    /// Test whether this mark has the same type and attributes as
    /// another mark.
    pub fn eq_mark(&self, other: &Mark) -> bool {
        std::ptr::eq(self as *const _, other as *const _)
            || (Arc::ptr_eq(&self.mark_type, &other.mark_type)
                && *self.attrs == *other.attrs)
    }

    /// Test whether two sets of marks are identical.
    ///
    /// Test whether two sets of marks are identical.
    pub fn same_set(a: &[Mark], b: &[Mark]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        a.iter().zip(b.iter()).all(|(x, y)| x.eq_mark(y))
    }
}

impl PartialEq for Mark {
    fn eq(&self, other: &Self) -> bool {
        self.eq_mark(other)
    }
}
```

- [ ] **Step 5: 在 schema.rs 中实现 MarkType::create（替换 create_stub）**

找到 `MarkType` impl 中的 `create_stub` 方法，替换为：

```rust
    /// Create a mark of this type. `attrs` may be `None` or a map containing
    /// only some of the mark's attributes. The others, if they have defaults,
    /// will be added.
    ///
    /// Create a mark of this type. `attrs` may be `null` or an object
    /// containing only some of the mark's attributes. The others, if
    /// they have defaults, will be added.
    pub fn create(
        self: &Arc<Self>,
        attrs: Option<&std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<crate::model::mark::Mark, crate::model::errors::SchemaError> {
        let attrs = compute_attrs(&self.spec.attrs, attrs)?;
        Ok(crate::model::mark::Mark {
            mark_type: self.clone(),
            attrs,
        })
    }
```

**注意：** 删除原来的 `create_stub` 方法（包括注释），用上面替换。同时删除 schema.rs 顶部 `mod.rs` 注释中关于 `create_stub` 的说明（如果有）。

- [ ] **Step 6: 运行测试，确认 same_set 和 eq 相关测试通过**

```bash
cargo test -p md-editor tests_mark
```

Expected: eq 和 same_set 测试通过，add_to_set/remove_from_set 测试还未实现

- [ ] **Step 7: Commit**

```bash
git add src/model/mark.rs src/model/tests_mark.rs src/model/mod.rs src/model/schema.rs
git commit -m "feat(model): add Mark::eq, same_set and MarkType::create"
```

---

## Task 3：Mark::add_to_set + 测试

**Files:**
- Modify: `src/model/mark.rs`
- Modify: `src/model/tests_mark.rs`

### add_to_set 算法说明

按 rank 顺序维护 mark 集合：
1. 遍历集合中每个 mark `other`：
   - 若 `self.eq_mark(other)` → 集合已包含，返回原集合的克隆
   - 若 `self.mark_type.excludes(&other.mark_type)` → 跳过 other（被排除）
   - 若 `other.mark_type.excludes(&self.mark_type)` → self 无法加入，返回原集合克隆
   - 若 `!placed && other.mark_type.rank > self.mark_type.rank` → 在 other 之前插入 self
2. 若未插入，追加到末尾

- [ ] **Step 1: 追加 add_to_set 测试到 tests_mark.rs**

```rust
// tests_mark.rs 末尾追加：

fn custom_schema() -> Arc<Schema> {
    Schema::new(SchemaSpec {
        nodes: indexmap! {
            "doc".to_string() => NodeSpec { content: Some("paragraph+".to_string()), ..NodeSpec::default() },
            "paragraph".to_string() => NodeSpec { content: Some("text*".to_string()), ..NodeSpec::default() },
            "text".to_string() => NodeSpec { inline: true, ..NodeSpec::default() },
        },
        marks: indexmap! {
            "remark".to_string() => MarkSpec {
                attrs: indexmap! { "id".to_string() => AttrSpec { default: None, validate: None } },
                excludes: Some("".to_string()), // excludes nothing
                inclusive: false,
                ..MarkSpec::default()
            },
            "user".to_string() => MarkSpec {
                attrs: indexmap! { "id".to_string() => AttrSpec { default: None, validate: None } },
                excludes: Some("_".to_string()), // excludes all
                ..MarkSpec::default()
            },
            "strong".to_string() => MarkSpec {
                excludes: Some("em-group".to_string()),
                ..MarkSpec::default()
            },
            "em".to_string() => MarkSpec {
                group: Some("em-group".to_string()),
                ..MarkSpec::default()
            },
        },
        top_node: None,
    }).unwrap()
}

fn mk_custom_mark(s: &Arc<Schema>, name: &str, id: i64) -> Mark {
    use std::collections::HashMap;
    let mut attrs: HashMap<String, serde_json::Value> = HashMap::new();
    attrs.insert("id".to_string(), json!(id));
    s.marks[name].create(&s.marks[name], Some(&attrs)).unwrap()
}

fn mk_no_attr_mark(s: &Arc<Schema>, name: &str) -> Mark {
    s.marks[name].create(&s.marks[name], None).unwrap()
}

// ---- add_to_set 测试 ----

#[test]
fn add_to_set_empty_set() {
    let s = test_schema();
    let em = mk_em(&s);
    let result = em.add_to_set(&[]);
    assert!(Mark::same_set(&result, &[em]));
}

#[test]
fn add_to_set_already_present() {
    let s = test_schema();
    let em = mk_em(&s);
    let result = em.add_to_set(&[em.clone()]);
    assert!(Mark::same_set(&result, &[em]));
}

#[test]
fn add_to_set_lower_rank_before_others() {
    let s = test_schema();
    // em has rank 0, strong has rank 1
    let result = mk_em(&s).add_to_set(&[mk_strong(&s)]);
    assert!(Mark::same_set(&result, &[mk_em(&s), mk_strong(&s)]));
}

#[test]
fn add_to_set_higher_rank_after_others() {
    let s = test_schema();
    let result = mk_strong(&s).add_to_set(&[mk_em(&s)]);
    assert!(Mark::same_set(&result, &[mk_em(&s), mk_strong(&s)]));
}

#[test]
fn add_to_set_replaces_excluded_mark() {
    let s = test_schema();
    // link replaces another link (same type, different attrs)
    let result = mk_link(&s, "http://bar", None).add_to_set(&[mk_link(&s, "http://foo", None), mk_em(&s)]);
    assert!(Mark::same_set(&result, &[mk_link(&s, "http://bar", None), mk_em(&s)]));
}

#[test]
fn add_to_set_no_op_existing_link() {
    let s = test_schema();
    let result = mk_link(&s, "http://foo", None).add_to_set(&[mk_em(&s), mk_link(&s, "http://foo", None)]);
    assert!(Mark::same_set(&result, &[mk_em(&s), mk_link(&s, "http://foo", None)]));
}

#[test]
fn add_to_set_code_at_end() {
    let s = test_schema();
    let result = mk_code(&s).add_to_set(&[mk_em(&s), mk_strong(&s), mk_link(&s, "http://foo", None)]);
    assert!(Mark::same_set(&result, &[mk_em(&s), mk_strong(&s), mk_link(&s, "http://foo", None), mk_code(&s)]));
}

#[test]
fn add_to_set_middle_rank() {
    let s = test_schema();
    // strong (rank 1) between em (rank 0) and code (rank 3)
    let result = mk_strong(&s).add_to_set(&[mk_em(&s), mk_code(&s)]);
    assert!(Mark::same_set(&result, &[mk_em(&s), mk_strong(&s), mk_code(&s)]));
}

#[test]
fn add_to_set_nonexclusive_allows_multiple() {
    let cs = custom_schema();
    let r1 = mk_custom_mark(&cs, "remark", 1);
    let r2 = mk_custom_mark(&cs, "remark", 2);
    let result = r2.add_to_set(&[r1.clone()]);
    assert!(Mark::same_set(&result, &[r1, r2]));
}

#[test]
fn add_to_set_no_duplicate_nonexclusive() {
    let cs = custom_schema();
    let r1 = mk_custom_mark(&cs, "remark", 1);
    let result = r1.clone().add_to_set(&[r1.clone()]);
    assert!(Mark::same_set(&result, &[r1]));
}

#[test]
fn add_to_set_globally_excluding_clears_others() {
    let cs = custom_schema();
    let r1 = mk_custom_mark(&cs, "remark", 1);
    let user1 = mk_custom_mark(&cs, "user", 1);
    let em = mk_no_attr_mark(&cs, "em");
    let result = user1.clone().add_to_set(&[r1, em]);
    assert!(Mark::same_set(&result, &[user1]));
}

#[test]
fn add_to_set_cannot_add_to_globally_excluding() {
    let cs = custom_schema();
    let user1 = mk_custom_mark(&cs, "user", 1);
    let em = mk_no_attr_mark(&cs, "em");
    let result = em.add_to_set(&[user1.clone()]);
    assert!(Mark::same_set(&result, &[user1]));
}

#[test]
fn add_to_set_globally_excluding_overwrites_itself() {
    let cs = custom_schema();
    let user1 = mk_custom_mark(&cs, "user", 1);
    let user2 = mk_custom_mark(&cs, "user", 2);
    let result = user2.clone().add_to_set(&[user1]);
    assert!(Mark::same_set(&result, &[user2]));
}

#[test]
fn add_to_set_blocked_by_excludes() {
    let cs = custom_schema();
    let r1 = mk_custom_mark(&cs, "remark", 1);
    let strong = mk_no_attr_mark(&cs, "strong");
    let em = mk_no_attr_mark(&cs, "em");
    // em is in em-group, strong excludes em-group → em cannot be added
    let result = em.add_to_set(&[r1.clone(), strong.clone()]);
    assert!(Mark::same_set(&result, &[r1, strong]));
}

#[test]
fn add_to_set_removes_excluded_by_new() {
    let cs = custom_schema();
    let r1 = mk_custom_mark(&cs, "remark", 1);
    let strong = mk_no_attr_mark(&cs, "strong");
    let em = mk_no_attr_mark(&cs, "em");
    // strong excludes em-group → adding strong removes em
    let result = strong.clone().add_to_set(&[r1.clone(), em]);
    assert!(Mark::same_set(&result, &[r1, strong]));
}
```

- [ ] **Step 2: 运行测试，确认失败**

```bash
cargo test -p md-editor tests_mark::add_to_set
```

Expected: FAIL（`add_to_set` 未定义）

- [ ] **Step 3: 在 mark.rs 中实现 add_to_set**

```rust
impl Mark {
    // ... 已有方法 ...

    /// Given a set of marks, create a new set which contains this one as
    /// well, in the right position. If this mark is already in the set,
    /// the set itself is returned. If any marks that are set to be
    /// exclusive with this mark are present, those are replaced by this one.
    pub fn add_to_set(&self, set: &[Mark]) -> Vec<Mark> {
        let mut copy: Option<Vec<Mark>> = None;
        let mut placed = false;

        for (i, other) in set.iter().enumerate() {
            if self.eq_mark(other) {
                return set.to_vec();
            }
            if self.mark_type.excludes(other.mark_type.as_ref()) {
                // This mark excludes `other` — skip other in the result
                if copy.is_none() {
                    copy = Some(set[..i].to_vec());
                }
            } else if other.mark_type.excludes(self.mark_type.as_ref()) {
                // `other` excludes this mark — cannot add
                return set.to_vec();
            } else {
                if !placed && other.mark_type.rank > self.mark_type.rank {
                    if copy.is_none() {
                        copy = Some(set[..i].to_vec());
                    }
                    copy.as_mut().unwrap().push(self.clone());
                    placed = true;
                }
                if let Some(ref mut c) = copy {
                    c.push(other.clone());
                }
            }
        }

        let mut result = copy.unwrap_or_else(|| set.to_vec());
        if !placed {
            result.push(self.clone());
        }
        result
    }
}
```

- [ ] **Step 4: 运行测试，确认通过**

```bash
cargo test -p md-editor tests_mark::add_to_set
```

Expected: 14 个 add_to_set 测试全通过

- [ ] **Step 5: Commit**

```bash
git add src/model/mark.rs src/model/tests_mark.rs
git commit -m "feat(model): add Mark::add_to_set with 14 tests"
```

---

## Task 4：Mark::remove_from_set + is_in_set + to_json + from_json + set_from

**Files:**
- Modify: `src/model/mark.rs`
- Modify: `src/model/tests_mark.rs`
- Modify: `src/model/schema.rs`（替换 MarkType::remove_from_set_stub 和 is_in_set_stub）

- [ ] **Step 1: 追加测试**

```rust
// tests_mark.rs 末尾追加：

// ---- remove_from_set 测试 ----

#[test]
fn remove_from_set_empty() {
    let s = test_schema();
    assert!(Mark::same_set(&mk_em(&s).remove_from_set(&[]), &[]));
}

#[test]
fn remove_from_set_removes_last() {
    let s = test_schema();
    let em = mk_em(&s);
    assert!(Mark::same_set(&em.remove_from_set(&[em.clone()]), &[]));
}

#[test]
fn remove_from_set_noop_not_present() {
    let s = test_schema();
    let em = mk_em(&s);
    let strong = mk_strong(&s);
    assert!(Mark::same_set(&strong.remove_from_set(&[em.clone()]), &[em]));
}

#[test]
fn remove_from_set_removes_by_attrs() {
    let s = test_schema();
    let link = mk_link(&s, "http://foo", None);
    assert!(Mark::same_set(&link.remove_from_set(&[mk_link(&s, "http://foo", None)]), &[]));
}

#[test]
fn remove_from_set_noop_different_attrs() {
    let s = test_schema();
    let link_title = mk_link(&s, "http://foo", Some("title"));
    let link_no_title = mk_link(&s, "http://foo", None);
    assert!(Mark::same_set(
        &link_title.remove_from_set(&[link_no_title.clone()]),
        &[link_no_title],
    ));
}

// ---- to_json / from_json 测试 ----

#[test]
fn mark_to_json_no_attrs() {
    let s = test_schema();
    let em = mk_em(&s);
    let json = em.to_json();
    assert_eq!(json["type"], json!("em"));
    assert!(json.get("attrs").is_none() || json["attrs"].as_object().map_or(true, |m| m.is_empty()));
}

#[test]
fn mark_to_json_with_attrs() {
    let s = test_schema();
    let link = mk_link(&s, "http://foo", None);
    let json = link.to_json();
    assert_eq!(json["type"], json!("link"));
    assert_eq!(json["attrs"]["href"], json!("http://foo"));
}

#[test]
fn mark_from_json_roundtrip() {
    let s = test_schema();
    let link = mk_link(&s, "http://foo", None);
    let json = link.to_json();
    let restored = Mark::from_json(&s, &json).unwrap();
    assert!(link.eq_mark(&restored));
}

#[test]
fn mark_from_json_invalid_type() {
    let s = test_schema();
    let json = json!({ "type": "nonexistent" });
    assert!(Mark::from_json(&s, &json).is_err());
}

// ---- set_from 测试 ----

#[test]
fn set_from_none_returns_empty() {
    assert_eq!(Mark::set_from(None).len(), 0);
}

#[test]
fn set_from_empty_slice_returns_empty() {
    assert_eq!(Mark::set_from(Some(&[])).len(), 0);
}

#[test]
fn set_from_sorts_by_rank() {
    let s = test_schema();
    // strong rank=1, em rank=0 → sorted: [em, strong]
    let unsorted = vec![mk_strong(&s), mk_em(&s)];
    let sorted = Mark::set_from(Some(&unsorted));
    assert_eq!(sorted[0].mark_type.name, "em");
    assert_eq!(sorted[1].mark_type.name, "strong");
}
```

- [ ] **Step 2: 确认测试失败**

```bash
cargo test -p md-editor tests_mark
```

Expected: 新增测试 FAIL

- [ ] **Step 3: 在 mark.rs 中实现剩余方法**

```rust
impl Mark {
    // ... 已有方法 ...

    /// Remove this mark from the given set, returning a new set. If this
    /// mark is not in the set, the set itself is returned.
    pub fn remove_from_set(&self, set: &[Mark]) -> Vec<Mark> {
        for (i, m) in set.iter().enumerate() {
            if self.eq_mark(m) {
                let mut result = set[..i].to_vec();
                result.extend_from_slice(&set[i + 1..]);
                return result;
            }
        }
        set.to_vec()
    }

    /// Test whether this mark is in the given set of marks.
    ///
    /// Test whether this mark is in the given set of marks.
    pub fn is_in_set(&self, set: &[Mark]) -> bool {
        set.iter().any(|m| self.eq_mark(m))
    }

    /// Convert this mark to a JSON-serializeable representation.
    ///
    /// Convert this mark to a JSON-serializeable representation.
    pub fn to_json(&self) -> serde_json::Value {
        let mut obj = serde_json::json!({ "type": self.mark_type.name });
        if !self.attrs.is_empty() {
            let attrs_val: serde_json::Value = serde_json::Value::Object(
                self.attrs
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            );
            obj["attrs"] = attrs_val;
        }
        obj
    }

    /// Deserialize a mark from JSON.
    ///
    /// Deserialize a mark from JSON.
    pub fn from_json(schema: &Schema, json: &serde_json::Value) -> Result<Self, RangeError> {
        let type_name = json
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RangeError("Invalid input for Mark.fromJSON".to_string()))?;
        let mark_type = schema
            .marks
            .get(type_name)
            .ok_or_else(|| {
                RangeError(format!(
                    "There is no mark type {} in this schema",
                    type_name
                ))
            })?;
        let attrs: Option<std::collections::HashMap<String, serde_json::Value>> = json
            .get("attrs")
            .and_then(|v| v.as_object())
            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
        mark_type
            .create(mark_type, attrs.as_ref())
            .map_err(|e| RangeError(e.0))
    }

    /// Create a properly sorted mark set from `None`, a single mark, or an
    /// unsorted slice of marks.
    ///
    /// Create a properly sorted mark set from null, a single mark, or an
    /// unsorted array of marks.
    pub fn set_from(marks: Option<&[Mark]>) -> Vec<Mark> {
        match marks {
            None | Some([]) => vec![],
            Some(marks) => {
                let mut copy = marks.to_vec();
                copy.sort_by_key(|m| m.mark_type.rank);
                copy
            }
        }
    }
}
```

- [ ] **Step 4: 在 schema.rs 中替换 MarkType::remove_from_set_stub 和 is_in_set_stub**

找到这两个 stub 方法，替换为：

```rust
    /// When there is a mark of this type in the given set, a new set
    /// without it is returned. Otherwise, the input set is returned.
    ///
    /// When there is a mark of this type in the given set, a new set
    /// without it is returned. Otherwise, the input set is returned.
    pub fn remove_from_set(
        &self,
        set: &[crate::model::mark::Mark],
    ) -> Vec<crate::model::mark::Mark> {
        set.iter()
            .filter(|m| !std::ptr::eq(m.mark_type.as_ref(), self))
            .cloned()
            .collect()
    }

    /// Tests whether there is a mark of this type in the given set.
    ///
    /// Tests whether there is a mark of this type in the given set.
    pub fn is_in_set<'a>(
        &self,
        set: &'a [crate::model::mark::Mark],
    ) -> Option<&'a crate::model::mark::Mark> {
        set.iter().find(|m| std::ptr::eq(m.mark_type.as_ref(), self))
    }
```

- [ ] **Step 5: 运行全量测试，确认通过**

```bash
cargo test -p md-editor tests_mark
```

Expected: 全部 tests_mark 测试通过

- [ ] **Step 6: Commit**

```bash
git add src/model/mark.rs src/model/tests_mark.rs src/model/schema.rs
git commit -m "feat(model): add Mark::remove_from_set, is_in_set, to_json, from_json, set_from and MarkType methods"
```

---

## Task 5：全量验证 + 更新阶段计划

**Files:**
- Modify: `docs/superpowers/plans/model-phases.md`

- [ ] **Step 1: 全量测试**

```bash
cargo test -p md-editor
```

Expected: 全部通过（Phase 1-2.3 的 ~105 个 + Phase 3.1 新增的 ~40 个 = ~145 个）

- [ ] **Step 2: Clippy**

```bash
cargo clippy -p md-editor -- -D warnings
```

Expected: 无 error，无 warning

- [ ] **Step 3: 无 unsafe**

```bash
grep -rn "unsafe" src/model/
```

Expected: 空

- [ ] **Step 4: 更新阶段计划**

将 `docs/superpowers/plans/model-phases.md` 中 Phase 3.1 的状态从 `⬜ 待开始` 改为 `✅ 完成`。

- [ ] **Step 5: Commit**

```bash
git add docs/superpowers/plans/model-phases.md
git commit -m "docs: mark model Phase 3.1 as complete"
```

---

## 自检

### Spec 覆盖

| TS 方法/属性 | Rust 实现 | 任务 |
|---|---|---|
| `Mark.type` | `pub mark_type: Arc<MarkType>` | Task 1 |
| `Mark.attrs` | `pub attrs: Attrs` | Task 1 |
| `Mark.eq(other)` | `fn eq_mark(&self, other: &Mark) -> bool` | Task 2 |
| `Mark.addToSet(set)` | `fn add_to_set(&self, set: &[Mark]) -> Vec<Mark>` | Task 3 |
| `Mark.removeFromSet(set)` | `fn remove_from_set(&self, set: &[Mark]) -> Vec<Mark>` | Task 4 |
| `Mark.isInSet(set)` | `fn is_in_set(&self, set: &[Mark]) -> bool` | Task 4 |
| `Mark.toJSON()` | `fn to_json(&self) -> Value` | Task 4 |
| `Mark.fromJSON(schema, json)` | `fn from_json(schema: &Schema, json: &Value) -> Result<Self, RangeError>` | Task 4 |
| `Mark.sameSet(a, b)` | `fn same_set(a: &[Mark], b: &[Mark]) -> bool` | Task 2 |
| `Mark.setFrom(marks)` | `fn set_from(marks: Option<&[Mark]>) -> Vec<Mark>` | Task 4 |
| `Mark.none` | `const NONE: &'static [Self] = &[]` | Task 1 |
| `MarkType.create(attrs)` | `fn create(self: &Arc<Self>, attrs) -> Result<Mark, SchemaError>` | Task 2 |
| `MarkType.removeFromSet(set)` | `fn remove_from_set(&self, set: &[Mark]) -> Vec<Mark>` | Task 4 |
| `MarkType.isInSet(set)` | `fn is_in_set<'a>(&self, set) -> Option<&'a Mark>` | Task 4 |

### 注意事项

- `Mark::eq_mark` 用 `std::ptr::eq` 检查实例相等（同一内存地址），而不是用 `Arc::ptr_eq`，因为 Mark 是值类型（不在 Arc 里）。对于 `MarkType::excludes` 中的类型比较，用 `std::ptr_eq(m.mark_type.as_ref(), self)`（比较 Arc 背后的 MarkType 指针）。
- `MarkType::create(self: &Arc<Self>, ...)` 使用 `self: &Arc<Self>` 语法，调用方必须持有 `Arc<MarkType>`（从 Schema 取出的总是 Arc）。
- `Mark::from_json` 内部调用 `mark_type.create(mark_type, ...)` — 第一个 `mark_type` 是 `&Arc<MarkType>`（自动解引用），第二个是 attrs 参数。
