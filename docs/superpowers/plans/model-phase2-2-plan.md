# model Phase 2.2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 `src/model/schema.rs` 中实现 `NodeType` 和 `MarkType` struct，包含所有不依赖 Fragment/Node/Mark 的方法，依赖这三个类型的方法留 `_stub` 占位。

**Architecture:** 全部新增代码在 `src/model/schema.rs`（已有 Spec 类型）。先实现内部辅助函数 `compute_attrs`/`default_attrs`，再实现 NodeType（结构体 + 谓词 + mark 权限方法），最后实现 MarkType。`content_match` 和 `mark_set` 字段在构建时先设默认值，Phase 2.3 的 `Schema::new` 会通过 `Arc::get_mut` 填入正确值。

**Tech Stack:** Rust 2024 edition, `indexmap`, `serde_json`, Phase 2.1 的 `ContentMatch`

**TS 参考：** `reference/prosemirror-model/src/schema.ts`（NodeType 类和 MarkType 类）

---

## 文件结构

```
src/model/schema.rs     ← 仅修改：新增 compute_attrs, default_attrs, NodeType, MarkType
```

---

## 背景：已有类型

Phase 1 已在 `schema.rs` 中定义：`AttrSpec`、`AttrValidator`、`AttrValidatorFn`、`NodeSpec`、`MarkSpec`、`SchemaSpec`、`Whitespace`、`ExcludeSpec`、`Attrs`（= `Arc<HashMap<String, Value>>`）

Phase 2.1 已在 `content.rs` 中定义：`ContentMatch`（`parse`, `match_type`, `edge_count`, `compatible`, `inline_content`, `valid_end` 等）

---

## Task 1：内部辅助函数 compute_attrs / default_attrs

**Files:**
- Modify: `src/model/schema.rs`

### TS 对应逻辑

```typescript
function computeAttrs(attrs: {[name: string]: Attribute}, value: Attrs | null) {
  let built = Object.create(null)
  for (let name in attrs) {
    let given = value && value[name]
    if (given === undefined) {
      let attr = attrs[name]
      if (attr.hasDefault) given = attr.default
      else throw new RangeError("No value supplied for attribute " + name)
    }
    built[name] = given
  }
  return built
}
function defaultAttrs(attrs) {
  // returns null if any attr has no default, else returns the default map
}
```

- [ ] **Step 1: 写测试（在 `schema.rs` 的 tests 模块末尾追加）**

```rust
    // ---- compute_attrs / default_attrs ----
    use std::collections::HashMap as StdHashMap;

    #[test]
    fn compute_attrs_uses_spec_defaults() {
        let mut specs: IndexMap<String, AttrSpec> = IndexMap::new();
        specs.insert("level".to_string(), AttrSpec { default: Some(json!(1)), validate: None });
        let result = compute_attrs(&specs, None).unwrap();
        assert_eq!(result.get("level"), Some(&json!(1)));
    }

    #[test]
    fn compute_attrs_provided_values_override_defaults() {
        let mut specs: IndexMap<String, AttrSpec> = IndexMap::new();
        specs.insert("level".to_string(), AttrSpec { default: Some(json!(1)), validate: None });
        let mut vals: StdHashMap<String, serde_json::Value> = StdHashMap::new();
        vals.insert("level".to_string(), json!(3));
        let result = compute_attrs(&specs, Some(&vals)).unwrap();
        assert_eq!(result.get("level"), Some(&json!(3)));
    }

    #[test]
    fn compute_attrs_errors_on_missing_required() {
        let mut specs: IndexMap<String, AttrSpec> = IndexMap::new();
        specs.insert("href".to_string(), AttrSpec { default: None, validate: None });
        let err = compute_attrs(&specs, None).unwrap_err();
        assert!(err.0.contains("href"));
    }

    #[test]
    fn compute_attrs_empty_specs_returns_empty_map() {
        let specs: IndexMap<String, AttrSpec> = IndexMap::new();
        let result = compute_attrs(&specs, None).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn default_attrs_some_when_all_have_defaults() {
        let mut specs: IndexMap<String, AttrSpec> = IndexMap::new();
        specs.insert("level".to_string(), AttrSpec { default: Some(json!(1)), validate: None });
        assert!(default_attrs(&specs).is_some());
    }

    #[test]
    fn default_attrs_none_when_any_required() {
        let mut specs: IndexMap<String, AttrSpec> = IndexMap::new();
        specs.insert("href".to_string(), AttrSpec { default: None, validate: None });
        assert!(default_attrs(&specs).is_none());
    }
```

- [ ] **Step 2: 确认测试失败**

```bash
cargo test -p md-editor model::schema::tests::compute_attrs
```

Expected: FAIL（函数未定义）

- [ ] **Step 3: 实现（在 `schema.rs` 的 `SchemaSpec` 定义之后、`#[cfg(test)]` 之前插入）**

```rust
/// Merge provided attribute values with spec defaults.
///
/// For node types where all attrs have a default value (or which don't
/// have any attributes), build up a single reusable default attribute
/// object, and use it for all nodes that don't specify specific attributes.
///
/// Corresponds to `computeAttrs` in TypeScript.
pub(crate) fn compute_attrs(
    specs: &IndexMap<String, AttrSpec>,
    values: Option<&std::collections::HashMap<String, serde_json::Value>>,
) -> Result<Attrs, crate::model::errors::SchemaError> {
    use std::collections::HashMap;
    let mut built: HashMap<String, serde_json::Value> = HashMap::new();
    for (name, spec) in specs {
        let given = values.and_then(|v| v.get(name));
        let value = if let Some(v) = given {
            v.clone()
        } else if let Some(d) = &spec.default {
            d.clone()
        } else {
            return Err(crate::model::errors::SchemaError(format!(
                "No value supplied for attribute {}",
                name
            )));
        };
        built.insert(name.clone(), value);
    }
    Ok(Arc::new(built))
}

/// Pre-compute a reusable default Attrs map.
///
/// Returns `None` if any attribute has no default (i.e. is required).
/// Returns `Some(attrs)` with all defaults filled in otherwise.
///
/// Corresponds to `defaultAttrs` in TypeScript.
pub(crate) fn default_attrs(specs: &IndexMap<String, AttrSpec>) -> Option<Attrs> {
    compute_attrs(specs, None).ok()
}
```

- [ ] **Step 4: 运行测试，确认通过**

```bash
cargo test -p md-editor model::schema::tests::compute_attrs
cargo test -p md-editor model::schema::tests::default_attrs
```

Expected: 全部通过

- [ ] **Step 5: Commit**

```bash
git add src/model/schema.rs
git commit -m "feat(model): add compute_attrs and default_attrs helpers"
```

---

## Task 2：NodeType struct + 谓词方法

**Files:**
- Modify: `src/model/schema.rs`

### 设计决策

- `is_inline()`、`is_textblock()`、`is_leaf()`、`is_atom()` 作为方法（不存字段），因为它们可从 `is_block`、`inline_content`、`content_match`、`spec.atom` 计算。
- `is_block`、`is_text`、`inline_content`、`whitespace` 存为字段（Schema::new 构建时赋值）。
- 不保留 TS 的 `schema` 反向引用（设计决策 5：避免循环引用）。
- `content_match` 初始值为 `ContentMatch::empty()`，Phase 2.3 的 `Schema::new` 通过 `Arc::get_mut` 填入真实值。
- `mark_set` 初始值为 `None`（表示允许所有 mark），Phase 2.3 填入。

- [ ] **Step 1: 在 schema.rs 的 tests 模块中追加 NodeType 测试辅助和谓词测试**

先在 `use super::*;` 下方加：
```rust
    use crate::model::content::ContentMatch;
```

然后追加：

```rust
    // ---- NodeType 谓词测试 ----

    fn make_paragraph_nodetype() -> NodeType {
        let nt_map: IndexMap<String, bool> = [("text".to_string(), true)].into_iter().collect();
        let g: IndexMap<String, Vec<String>> = IndexMap::new();
        let r: IndexMap<String, bool> = IndexMap::new();
        let cm = ContentMatch::parse("text*", &nt_map, &g, &r).unwrap();
        NodeType {
            name: "paragraph".to_string(),
            is_block: true,
            is_text: false,
            inline_content: cm.inline_content,
            groups: vec!["block".to_string()],
            content_match: cm,
            mark_set: None,
            whitespace: Whitespace::Normal,
            spec: NodeSpec::default(),
            default_attrs: None,
        }
    }

    fn make_text_nodetype() -> NodeType {
        NodeType {
            name: "text".to_string(),
            is_block: false,
            is_text: true,
            inline_content: false,
            groups: vec![],
            content_match: ContentMatch::empty(),
            mark_set: None,
            whitespace: Whitespace::Normal,
            spec: NodeSpec::default(),
            default_attrs: None,
        }
    }

    fn make_image_nodetype() -> NodeType {
        NodeType {
            name: "image".to_string(),
            is_block: false,
            is_text: false,
            inline_content: false,
            groups: vec!["inline".to_string()],
            content_match: ContentMatch::empty(),
            mark_set: None,
            whitespace: Whitespace::Normal,
            spec: NodeSpec { atom: true, ..NodeSpec::default() },
            default_attrs: None,
        }
    }

    #[test]
    fn nodetype_paragraph_is_block_not_inline() {
        let p = make_paragraph_nodetype();
        assert!(p.is_block);
        assert!(!p.is_text);
        assert!(!p.is_inline());
    }

    #[test]
    fn nodetype_text_is_inline_and_text() {
        let t = make_text_nodetype();
        assert!(t.is_text);
        assert!(t.is_inline());
        assert!(!t.is_block);
    }

    #[test]
    fn nodetype_paragraph_is_textblock() {
        let p = make_paragraph_nodetype();
        assert!(p.is_textblock()); // block + inline_content
    }

    #[test]
    fn nodetype_image_is_leaf_and_atom() {
        let img = make_image_nodetype();
        assert!(img.is_leaf());
        assert!(img.is_atom());
    }

    #[test]
    fn nodetype_paragraph_is_not_leaf() {
        let p = make_paragraph_nodetype();
        assert!(!p.is_leaf());
        assert!(!p.is_atom());
    }

    #[test]
    fn nodetype_is_in_group() {
        let p = make_paragraph_nodetype();
        assert!(p.is_in_group("block"));
        assert!(!p.is_in_group("inline"));
    }

    #[test]
    fn nodetype_whitespace_normal_by_default() {
        let p = make_paragraph_nodetype();
        assert_eq!(p.whitespace(), Whitespace::Normal);
    }

    #[test]
    fn nodetype_whitespace_pre_when_code_true() {
        let code_node = NodeType {
            spec: NodeSpec { code: true, ..NodeSpec::default() },
            ..make_paragraph_nodetype()
        };
        assert_eq!(code_node.whitespace(), Whitespace::Pre);
    }

    #[test]
    fn nodetype_whitespace_spec_override() {
        let pre_node = NodeType {
            spec: NodeSpec { whitespace: Some(Whitespace::Pre), ..NodeSpec::default() },
            ..make_paragraph_nodetype()
        };
        assert_eq!(pre_node.whitespace(), Whitespace::Pre);
    }

    #[test]
    fn nodetype_has_required_attrs_false_when_all_defaulted() {
        let p = make_paragraph_nodetype();
        assert!(!p.has_required_attrs());
    }

    #[test]
    fn nodetype_has_required_attrs_true_when_any_required() {
        use indexmap::indexmap;
        let p = NodeType {
            spec: NodeSpec {
                attrs: indexmap! {
                    "href".to_string() => AttrSpec { default: None, validate: None }
                },
                ..NodeSpec::default()
            },
            ..make_paragraph_nodetype()
        };
        assert!(p.has_required_attrs());
    }
```

- [ ] **Step 2: 确认测试失败**

```bash
cargo test -p md-editor model::schema::tests::nodetype
```

Expected: FAIL（NodeType 未定义）

- [ ] **Step 3: 在 schema.rs 中 `compute_attrs` 之后插入 `NodeType` struct 和方法**

```rust
/// Node types are objects allocated once per `Schema` and used to
/// tag `Node` instances. They contain information about the node type,
/// such as its name and what kind of node it represents.
///
/// Corresponds to `NodeType` in TypeScript.
/// Note: `schema` back-reference removed (design decision 5 — avoids circular Arc).
#[derive(Clone)]
pub struct NodeType {
    /// The name the node type has in this schema.
    pub name: String,
    /// True if this is a block-level node (`!(spec.inline || name == "text")`).
    pub is_block: bool,
    /// True if this is the text node type.
    pub is_text: bool,
    /// True if this node type has inline content.
    /// Set during Schema construction from `content_match.inline_content`.
    pub inline_content: bool,
    /// The groups this node type belongs to (derived from `spec.group`).
    pub groups: Vec<String>,
    /// The starting match state of the node type's content expression.
    /// Initialised to `ContentMatch::empty()`; filled in by `Schema::new`.
    pub content_match: Arc<ContentMatch>,
    /// The set of marks allowed in this node. `None` means all marks are allowed.
    /// Filled in by `Schema::new`.
    pub mark_set: Option<Vec<Arc<MarkType>>>,
    /// The node type's whitespace handling mode.
    /// Derived from `spec.whitespace` or `spec.code` at construction.
    pub whitespace: Whitespace,
    /// The spec that this type is based on.
    pub spec: NodeSpec,
    /// Pre-computed default attributes (None if any attribute is required).
    pub(crate) default_attrs: Option<Attrs>,
}

impl NodeType {
    /// True if this is an inline type.
    ///
    /// True if this is an inline type.
    pub fn is_inline(&self) -> bool {
        !self.is_block
    }

    /// True if this is a textblock type, a block that contains inline content.
    ///
    /// True if this is a textblock type, a block that contains inline
    /// content.
    pub fn is_textblock(&self) -> bool {
        self.is_block && self.inline_content
    }

    /// True for node types that allow no content.
    ///
    /// True for node types that allow no content.
    pub fn is_leaf(&self) -> bool {
        self.content_match.edge_count() == 0 && self.content_match.valid_end
    }

    /// True when this node is an atom, i.e. when it does not have
    /// directly editable content.
    pub fn is_atom(&self) -> bool {
        self.is_leaf() || self.spec.atom
    }

    /// Return true when this node type is part of the given group.
    ///
    /// Return true when this node type is part of the given
    /// [group](#model.NodeSpec.group).
    pub fn is_in_group(&self, group: &str) -> bool {
        self.groups.iter().any(|g| g == group)
    }

    /// The node type's whitespace option.
    ///
    /// The node type's [whitespace](#model.NodeSpec.whitespace) option.
    pub fn whitespace(&self) -> Whitespace {
        self.spec.whitespace.unwrap_or(if self.spec.code {
            Whitespace::Pre
        } else {
            Whitespace::Normal
        })
    }

    /// Tells you whether this node type has any required attributes.
    ///
    /// Tells you whether this node type has any required attributes.
    pub fn has_required_attrs(&self) -> bool {
        self.spec.attrs.values().any(|a| a.is_required())
    }

    /// Indicates whether this node allows some of the same content as
    /// the given node type.
    pub fn compatible_content(&self, other: &NodeType) -> bool {
        std::ptr::eq(self, other) || self.content_match.compatible(&other.content_match)
    }

    /// Compute attributes, merging provided values with spec defaults.
    ///
    /// @internal
    pub(crate) fn compute_attrs_for(
        &self,
        attrs: Option<&std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<Attrs, crate::model::errors::SchemaError> {
        if attrs.is_none() {
            if let Some(defaults) = &self.default_attrs {
                return Ok(defaults.clone());
            }
        }
        compute_attrs(&self.spec.attrs, attrs)
    }

    /// Check whether the given mark type is allowed in this node.
    ///
    /// Check whether the given mark type is allowed in this node.
    pub fn allows_mark_type(&self, mark_type: &Arc<MarkType>) -> bool {
        self.mark_set
            .as_ref()
            .map_or(true, |set| set.iter().any(|m| Arc::ptr_eq(m, mark_type)))
    }

    // ---- Phase 3 stubs (依赖 Fragment / Node / Mark) ----

    /// Create a `Node` of this type.
    ///
    /// NOTE: stub — requires Node/Fragment/Mark (Phase 3).
    #[allow(unused)]
    pub fn create_stub(&self) -> ! {
        todo!("NodeType::create requires Node/Fragment/Mark — implement in Phase 3")
    }

    /// Like `create`, but check the given content against the node type's content restrictions.
    ///
    /// NOTE: stub — requires Fragment (Phase 3).
    #[allow(unused)]
    pub fn create_checked_stub(&self) -> ! {
        todo!("NodeType::create_checked requires Fragment — implement in Phase 3")
    }

    /// Like `create`, but fill in missing content automatically.
    ///
    /// NOTE: stub — requires Fragment (Phase 3).
    #[allow(unused)]
    pub fn create_and_fill_stub(&self) -> ! {
        todo!("NodeType::create_and_fill requires Fragment — implement in Phase 3")
    }

    /// Returns true if the given fragment is valid content for this node type.
    ///
    /// NOTE: stub — requires Fragment (Phase 3).
    #[allow(unused)]
    pub fn valid_content_stub(&self) -> ! {
        todo!("NodeType::valid_content requires Fragment — implement in Phase 3")
    }

    /// Test whether the given set of marks are allowed in this node.
    ///
    /// NOTE: stub — requires Mark (Phase 3).
    #[allow(unused)]
    pub fn allows_marks_stub(&self) -> ! {
        todo!("NodeType::allows_marks requires Mark — implement in Phase 3")
    }

    /// Removes the marks that are not allowed in this node from the given set.
    ///
    /// NOTE: stub — requires Mark (Phase 3).
    #[allow(unused)]
    pub fn allowed_marks_stub(&self) -> ! {
        todo!("NodeType::allowed_marks requires Mark — implement in Phase 3")
    }
}
```

**注意：** `MarkType` 在这一步还不存在，`mark_set: Option<Vec<Arc<MarkType>>>` 会导致编译错误。先把 `mark_set` 的类型暂时替换为 `mark_set: Option<Vec<Arc<()>>>` 并在 Task 4 完成后修正。

**实际操作：** 直接跳到 Task 4（MarkType）先定义一个空的占位 struct，让编译通过，然后再填充：

在插入 `NodeType` 之前，先插入一个前向声明占位（放在 `compute_attrs` 之前）：

```rust
// Forward declaration placeholder — full definition in Task 4.
/// Like nodes, marks (which are associated with nodes to signify
/// things like emphasis or being part of a link) are
/// tagged with type objects, which are instantiated once per `Schema`.
///
/// Corresponds to `MarkType` in TypeScript.
#[derive(Clone)]
pub struct MarkType {
    pub name: String,
    pub rank: usize,
    pub inclusive: bool,
    pub excluded: ExcludeSpec,
    pub spec: MarkSpec,
    pub(crate) default_attrs: Option<Attrs>,
}
```

这样 `NodeType.mark_set: Option<Vec<Arc<MarkType>>>` 可以编译。Task 4 会在此基础上添加方法（impl MarkType）。

- [ ] **Step 4: 运行测试，确认通过**

```bash
cargo test -p md-editor model::schema::tests::nodetype
```

Expected: 全部通过

- [ ] **Step 5: Commit**

```bash
git add src/model/schema.rs
git commit -m "feat(model): add NodeType struct with predicate methods and stubs"
```

---

## Task 3：NodeType mark 权限和兼容性测试

**Files:**
- Modify: `src/model/schema.rs`（只加测试，方法已在 Task 2 实现）

- [ ] **Step 1: 追加测试**

```rust
    #[test]
    fn nodetype_allows_mark_type_none_means_all() {
        let p = make_paragraph_nodetype();
        let mt = Arc::new(MarkType {
            name: "em".to_string(),
            rank: 0,
            inclusive: true,
            excluded: ExcludeSpec::SelfOnly,
            spec: MarkSpec::default(),
            default_attrs: None,
        });
        // mark_set = None → all marks allowed
        assert!(p.allows_mark_type(&mt));
    }

    #[test]
    fn nodetype_allows_mark_type_restricted_set() {
        let mt_em = Arc::new(MarkType {
            name: "em".to_string(),
            rank: 0,
            inclusive: true,
            excluded: ExcludeSpec::SelfOnly,
            spec: MarkSpec::default(),
            default_attrs: None,
        });
        let mt_strong = Arc::new(MarkType {
            name: "strong".to_string(),
            rank: 1,
            inclusive: true,
            excluded: ExcludeSpec::SelfOnly,
            spec: MarkSpec::default(),
            default_attrs: None,
        });
        // mark_set = Some([em]) → only em allowed
        let p = NodeType {
            mark_set: Some(vec![mt_em.clone()]),
            ..make_paragraph_nodetype()
        };
        assert!(p.allows_mark_type(&mt_em));
        assert!(!p.allows_mark_type(&mt_strong));
    }

    #[test]
    fn nodetype_compatible_content_same_node() {
        let p = make_paragraph_nodetype();
        assert!(p.compatible_content(&p));
    }

    #[test]
    fn nodetype_compute_attrs_for_uses_defaults() {
        let p = make_paragraph_nodetype(); // spec.attrs is empty → no required attrs
        let result = p.compute_attrs_for(None).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn nodetype_compute_attrs_for_returns_cached_defaults() {
        let p = make_paragraph_nodetype();
        // Two calls return equal maps
        let r1 = p.compute_attrs_for(None).unwrap();
        let r2 = p.compute_attrs_for(None).unwrap();
        assert_eq!(*r1, *r2);
    }
```

- [ ] **Step 2: 运行测试，确认通过**

```bash
cargo test -p md-editor model::schema::tests::nodetype
cargo test -p md-editor model::schema::tests::compute_attrs
```

Expected: 全部通过

- [ ] **Step 3: Commit**

```bash
git add src/model/schema.rs
git commit -m "feat(model): add NodeType mark/content method tests"
```

---

## Task 4：MarkType 方法 + stubs

**Files:**
- Modify: `src/model/schema.rs`

MarkType struct 在 Task 2 已作为占位存在。本 task 补全其 impl 块。

### TS 参考

```typescript
class MarkType {
  create(attrs: Attrs | null = null) { ... }   // needs Mark
  static compile(marks, schema) { ... }        // internal
  removeFromSet(set: readonly Mark[]) { ... }  // needs Mark
  isInSet(set: readonly Mark[]) { ... }        // needs Mark
  checkAttrs(attrs: Attrs) { ... }             // internal
  excludes(other: MarkType) {
    return this.excluded.indexOf(other) > -1
  }
}
```

- [ ] **Step 1: 追加 MarkType 测试**

```rust
    // ---- MarkType 测试 ----

    fn make_em_marktype() -> MarkType {
        MarkType {
            name: "em".to_string(),
            rank: 0,
            inclusive: true,
            excluded: ExcludeSpec::SelfOnly,
            spec: MarkSpec::default(),
            default_attrs: None,
        }
    }

    fn make_strong_marktype() -> MarkType {
        MarkType {
            name: "strong".to_string(),
            rank: 1,
            inclusive: true,
            excluded: ExcludeSpec::SelfOnly,
            spec: MarkSpec::default(),
            default_attrs: None,
        }
    }

    fn make_link_marktype() -> MarkType {
        MarkType {
            name: "link".to_string(),
            rank: 2,
            inclusive: false,
            // link excludes itself (links can't be nested)
            excluded: ExcludeSpec::SelfOnly,
            spec: MarkSpec::default(),
            default_attrs: None,
        }
    }

    #[test]
    fn marktype_excludes_self_only_same_name() {
        let em = make_em_marktype();
        assert!(em.excludes(&em));
    }

    #[test]
    fn marktype_excludes_self_only_different_name() {
        let em = make_em_marktype();
        let strong = make_strong_marktype();
        assert!(!em.excludes(&strong));
    }

    #[test]
    fn marktype_excludes_all() {
        let mt = MarkType {
            excluded: ExcludeSpec::All,
            ..make_em_marktype()
        };
        assert!(mt.excludes(&make_strong_marktype()));
        assert!(mt.excludes(&make_link_marktype()));
    }

    #[test]
    fn marktype_excludes_named_list() {
        let mt = MarkType {
            excluded: ExcludeSpec::Named(vec!["strong".to_string()]),
            ..make_em_marktype()
        };
        assert!(mt.excludes(&make_strong_marktype()));
        assert!(!mt.excludes(&make_link_marktype()));
    }

    #[test]
    fn marktype_has_required_attrs_false_when_empty() {
        let em = make_em_marktype();
        assert!(!em.has_required_attrs());
    }

    #[test]
    fn marktype_has_required_attrs_true_when_any_required() {
        use indexmap::indexmap;
        let mt = MarkType {
            spec: MarkSpec {
                attrs: indexmap! {
                    "href".to_string() => AttrSpec { default: None, validate: None }
                },
                ..MarkSpec::default()
            },
            ..make_em_marktype()
        };
        assert!(mt.has_required_attrs());
    }

    #[test]
    fn marktype_compute_attrs_uses_defaults() {
        let em = make_em_marktype();
        let result = em.compute_attrs_for(None).unwrap();
        assert!(result.is_empty()); // no attrs defined
    }
```

- [ ] **Step 2: 确认测试失败**

```bash
cargo test -p md-editor model::schema::tests::marktype
```

Expected: FAIL（MarkType 没有 `excludes`, `has_required_attrs`, `compute_attrs_for` 方法）

- [ ] **Step 3: 在 MarkType struct 之后添加 impl 块**

```rust
impl MarkType {
    /// Queries whether a given mark type is excluded by this one.
    ///
    /// Queries whether a given mark type is
    /// [excluded](#model.MarkSpec.excludes) by this one.
    pub fn excludes(&self, other: &MarkType) -> bool {
        self.excluded.excludes(&self.name, &other.name)
    }

    /// Tells you whether this mark type has any required attributes.
    pub fn has_required_attrs(&self) -> bool {
        self.spec.attrs.values().any(|a| a.is_required())
    }

    /// Compute attributes, merging provided values with spec defaults.
    ///
    /// @internal
    pub(crate) fn compute_attrs_for(
        &self,
        attrs: Option<&std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<Attrs, crate::model::errors::SchemaError> {
        if attrs.is_none() {
            if let Some(defaults) = &self.default_attrs {
                return Ok(defaults.clone());
            }
        }
        compute_attrs(&self.spec.attrs, attrs)
    }

    // ---- Phase 3 stubs (依赖 Mark) ----

    /// Create a mark of this type. `attrs` may be `null` or an object
    /// containing only some of the mark's attributes.
    ///
    /// NOTE: stub — requires Mark (Phase 3).
    #[allow(unused)]
    pub fn create_stub(&self) -> ! {
        todo!("MarkType::create requires Mark — implement in Phase 3")
    }

    /// When there is a mark of this type in the given set, a new set
    /// without it is returned. Otherwise, the input set is returned.
    ///
    /// NOTE: stub — requires Mark (Phase 3).
    #[allow(unused)]
    pub fn remove_from_set_stub(&self) -> ! {
        todo!("MarkType::remove_from_set requires Mark — implement in Phase 3")
    }

    /// Tests whether there is a mark of this type in the given set.
    ///
    /// NOTE: stub — requires Mark (Phase 3).
    #[allow(unused)]
    pub fn is_in_set_stub(&self) -> ! {
        todo!("MarkType::is_in_set requires Mark — implement in Phase 3")
    }
}
```

- [ ] **Step 4: 运行测试，确认通过**

```bash
cargo test -p md-editor model::schema::tests::marktype
```

Expected: 全部通过

- [ ] **Step 5: Commit**

```bash
git add src/model/schema.rs
git commit -m "feat(model): add MarkType methods (excludes, has_required_attrs, compute_attrs_for) and stubs"
```

---

## Task 5：全量验证 + 更新阶段计划

**Files:**
- Modify: `docs/superpowers/plans/model-phases.md`

- [ ] **Step 1: 运行全量测试**

```bash
cargo test -p md-editor
```

Expected（数量可能略有出入，但全部通过）：
```
running ~70 tests
test result: ok. N passed; 0 failed
```

具体分布：
- `model::errors::tests` — 5 个
- `model::compare_deep::tests` — 8 个
- `model::schema::tests` — 原有 11 个 + Phase 2.2 新增约 25 个 = ~36 个
- `model::content::tests` — 26 个

- [ ] **Step 2: Clippy 检查**

```bash
cargo clippy -p md-editor -- -D warnings
```

Expected: 无 error，无 warning

- [ ] **Step 3: 确认无 unsafe**

```bash
grep -rn "unsafe" src/model/
```

Expected: 空输出

- [ ] **Step 4: 更新阶段计划**

将 `docs/superpowers/plans/model-phases.md` 中：
```
| Phase 2.2 | NodeType + MarkType | schema.rs 续（NodeType、MarkType） | Phase 2.1 | ⬜ 待开始 |
```
改为：
```
| Phase 2.2 | NodeType + MarkType | schema.rs 续（NodeType、MarkType） | Phase 2.1 | ✅ 完成 |
```

- [ ] **Step 5: Commit**

```bash
git add docs/superpowers/plans/model-phases.md
git commit -m "docs: mark model Phase 2.2 as complete"
```

---

## 自检

### Spec 覆盖

| TS 方法/属性 | Rust 实现 | 位置 |
|---|---|---|
| `NodeType.isBlock` | `pub is_block: bool` 字段 | Task 2 |
| `NodeType.isText` | `pub is_text: bool` 字段 | Task 2 |
| `NodeType.isInline` | `fn is_inline()` 方法 | Task 2 |
| `NodeType.inlineContent` | `pub inline_content: bool` 字段 | Task 2 |
| `NodeType.isTextblock` | `fn is_textblock()` 方法 | Task 2 |
| `NodeType.isLeaf` | `fn is_leaf()` 方法 | Task 2 |
| `NodeType.isAtom` | `fn is_atom()` 方法 | Task 2 |
| `NodeType.isInGroup()` | `fn is_in_group()` 方法 | Task 2 |
| `NodeType.contentMatch` | `pub content_match: Arc<ContentMatch>` 字段 | Task 2 |
| `NodeType.markSet` | `pub mark_set: Option<Vec<Arc<MarkType>>>` 字段 | Task 2 |
| `NodeType.whitespace` | `fn whitespace()` 方法 | Task 2 |
| `NodeType.hasRequiredAttrs()` | `fn has_required_attrs()` 方法 | Task 2 |
| `NodeType.compatibleContent()` | `fn compatible_content()` 方法 | Task 2 |
| `NodeType.computeAttrs()` | `fn compute_attrs_for()` 方法 | Task 2 |
| `NodeType.allowsMarkType()` | `fn allows_mark_type()` 方法 | Task 2 |
| `NodeType.allowsMarks()` | `fn allows_marks_stub()` stub | Task 2 |
| `NodeType.allowedMarks()` | `fn allowed_marks_stub()` stub | Task 2 |
| `NodeType.create()` | `fn create_stub()` stub | Task 2 |
| `NodeType.createChecked()` | `fn create_checked_stub()` stub | Task 2 |
| `NodeType.createAndFill()` | `fn create_and_fill_stub()` stub | Task 2 |
| `NodeType.validContent()` | `fn valid_content_stub()` stub | Task 2 |
| `MarkType.excludes()` | `fn excludes()` 方法 | Task 4 |
| `MarkType.create()` | `fn create_stub()` stub | Task 4 |
| `MarkType.removeFromSet()` | `fn remove_from_set_stub()` stub | Task 4 |
| `MarkType.isInSet()` | `fn is_in_set_stub()` stub | Task 4 |

### 注意事项

- `NodeType.spec` 中 `whitespace` 字段已在 Phase 2.2 直接存为 `Whitespace` 字段（不需要再从 spec 推断），但 `whitespace()` 方法仍从 `spec` 计算，与 TS 保持一致。
- `NodeType::default_attrs` 是 `pub(crate)` 因为只有 Schema::new（Phase 2.3）需要读它。
- `MarkType` 的 `excluded: ExcludeSpec` 在 Phase 2.3 的 `Schema::new` 中通过 `Arc::get_mut` 填入正确值（从 spec.excludes 字符串解析）。
