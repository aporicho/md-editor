# model Phase 2.3 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 `src/model/schema.rs` 中实现 `Schema` struct 及 `Schema::new` 两遍构建，使得可以从 `SchemaSpec` 构建出完整的 Schema，其中每个 `NodeType` 的 `content_match`、`inline_content`、`mark_set` 以及每个 `MarkType` 的 `excluded` 均被正确填入。

**Architecture:** 全部代码在 `src/model/schema.rs`。第一遍：创建所有 `Arc<NodeType>` 和 `Arc<MarkType>`（refcount=1，占位字段）。第二遍：`Arc::get_mut` 安全填入 `content_match`/`inline_content`/`mark_set`/`excluded`（refcount 仍为 1，因此不需要 unsafe）。辅助函数 `gather_marks`/`resolve_mark_set`/`resolve_excluded` 私有，仅供 `Schema::new` 使用。

**Tech Stack:** Rust 2024 edition, `indexmap`, `serde_json`, Phase 2.1 的 `ContentMatch::parse`, Phase 2.2 的 `NodeType`/`MarkType`/`compute_attrs`/`default_attrs`

**TS 参考：** `reference/prosemirror-model/src/schema.ts`（`Schema` 构造器 + `gatherMarks`）

---

## 文件结构

```
src/model/schema.rs     ← 仅修改：新增私有辅助函数 + Schema struct + impl
```

已有（不需要修改）：
- `AttrSpec`, `NodeSpec`, `MarkSpec`, `SchemaSpec`, `Whitespace`, `ExcludeSpec`, `Attrs`
- `compute_attrs`, `default_attrs`
- `NodeType`, `MarkType`（含所有方法）

---

## 关键设计

### Schema 两遍构建原理

```
第一遍：
  for each node in spec.nodes:
    Arc::new(NodeType { content_match: ContentMatch::empty(), mark_set: None, inline_content: false, ... })
    refcount = 1

  for each mark in spec.marks:
    Arc::new(MarkType { excluded: ExcludeSpec::SelfOnly, ... })
    refcount = 1

第二遍（Arc::get_mut 安全，refcount 仍 = 1）：
  for each node:
    let nt = Arc::get_mut(node_types[name]).unwrap();
    nt.content_match = ContentMatch::parse(expr, &is_inline_map, &groups_map, &has_required_map)?;
    nt.inline_content = nt.content_match.inline_content;
    nt.mark_set = resolve_mark_set(ns.marks, &mark_types, nt.inline_content)?;

  for each mark:
    let mt = Arc::get_mut(mark_types[name]).unwrap();
    mt.excluded = resolve_excluded(ms.excludes, name, &mark_types)?;
```

### ContentMatch::parse 的参数

ContentMatch::parse 需要：
- `node_types: &IndexMap<String, bool>` — 名称 → is_inline（`!is_block`）
- `groups: &IndexMap<String, Vec<String>>` — 组名 → 成员名列表
- `has_required_attrs: &IndexMap<String, bool>` — 名称 → 是否有必填 attr

这些从第一遍创建的 `Arc<NodeType>` 中提取。

### resolve_mark_set 逻辑（对应 TS 的 markSet 赋值）

```
marks_spec = None, inline_content = true  → None（所有 mark 允许）
marks_spec = None, inline_content = false → Some(vec![])（无 mark）
marks_spec = Some("_")                    → None（所有 mark 允许）
marks_spec = Some("")                     → Some(vec![])（无 mark）
marks_spec = Some("em strong")            → Some(gather_marks(["em","strong"], &mark_types))
```

### resolve_excluded 逻辑（对应 TS 的 excluded 赋值）

```
excludes_spec = None         → ExcludeSpec::SelfOnly
excludes_spec = Some("_")    → ExcludeSpec::All
excludes_spec = Some("")     → ExcludeSpec::Named(vec![])
excludes_spec = Some("em formatting") →
    ExcludeSpec::Named(resolve each token to type names, expanding group names)
```

---

## Task 1：Schema struct + 验证辅助 + 第一遍构建

**Files:**
- Modify: `src/model/schema.rs`

**TS 参考：** `Schema` 构造器前半段（创建 nodes/marks map，校验）

- [ ] **Step 1: 在 tests 模块末尾追加 Schema 基础测试**

在 marktype 测试之后追加（仍在 `mod tests { }` 内）：

```rust
    // ---- Schema 基础测试 ----
    use crate::model::schema::Schema;

    fn minimal_schema_spec() -> SchemaSpec {
        use indexmap::indexmap;
        SchemaSpec {
            nodes: indexmap! {
                "doc".to_string() => NodeSpec {
                    content: Some("paragraph+".to_string()),
                    ..NodeSpec::default()
                },
                "paragraph".to_string() => NodeSpec {
                    content: Some("text*".to_string()),
                    group: Some("block".to_string()),
                    ..NodeSpec::default()
                },
                "text".to_string() => NodeSpec {
                    inline: true,
                    ..NodeSpec::default()
                },
            },
            marks: IndexMap::new(),
            top_node: None,
        }
    }

    #[test]
    fn schema_new_builds_successfully() {
        let s = Schema::new(minimal_schema_spec()).unwrap();
        assert!(s.nodes.contains_key("doc"));
        assert!(s.nodes.contains_key("paragraph"));
        assert!(s.nodes.contains_key("text"));
    }

    #[test]
    fn schema_top_node_type_is_doc() {
        let s = Schema::new(minimal_schema_spec()).unwrap();
        assert_eq!(s.top_node_type.name, "doc");
    }

    #[test]
    fn schema_error_missing_top_node() {
        let mut spec = minimal_schema_spec();
        spec.top_node = Some("missing".to_string());
        assert!(Schema::new(spec).is_err());
    }

    #[test]
    fn schema_error_missing_text_node() {
        use indexmap::indexmap;
        let spec = SchemaSpec {
            nodes: indexmap! {
                "doc".to_string() => NodeSpec { content: Some("paragraph+".to_string()), ..NodeSpec::default() },
                "paragraph".to_string() => NodeSpec::default(),
            },
            marks: IndexMap::new(),
            top_node: None,
        };
        let err = Schema::new(spec).unwrap_err();
        assert!(err.0.contains("text"));
    }

    #[test]
    fn schema_error_text_node_has_attrs() {
        use indexmap::{indexmap, IndexMap};
        let mut spec = minimal_schema_spec();
        if let Some(ns) = spec.nodes.get_mut("text") {
            ns.attrs = indexmap! {
                "x".to_string() => AttrSpec { default: None, validate: None }
            };
        }
        assert!(Schema::new(spec).is_err());
    }

    #[test]
    fn schema_error_name_both_node_and_mark() {
        use indexmap::indexmap;
        let mut spec = minimal_schema_spec();
        spec.marks.insert("paragraph".to_string(), MarkSpec::default());
        assert!(Schema::new(spec).is_err());
    }
```

- [ ] **Step 2: 确认测试失败**

```bash
cargo test -p md-editor model::schema::tests::schema
```

Expected: FAIL（Schema 未定义）

- [ ] **Step 3: 在 MarkType impl 之后、`#[cfg(test)]` 之前插入 Schema struct + new() 骨架（仅第一遍，暂不填充 content_match）**

```rust
/// A document schema. Holds node type and mark type objects for the nodes and marks
/// that may occur in conforming documents, and provides functionality for
/// creating and deserializing such documents.
///
/// Corresponds to `Schema` in TypeScript.
pub struct Schema {
    /// An object mapping the schema's node names to node type objects.
    pub nodes: IndexMap<String, Arc<NodeType>>,
    /// A map from mark names to mark type objects.
    pub marks: IndexMap<String, Arc<MarkType>>,
    /// The type of the default top node for this schema.
    pub top_node_type: Arc<NodeType>,
    /// The linebreak replacement node defined in this schema, if any.
    pub linebreak_replacement: Option<Arc<NodeType>>,
    /// The spec on which the schema is based.
    pub spec: SchemaSpec,
}

impl Schema {
    /// Construct a schema from a schema specification.
    ///
    /// Construct a schema from a schema [specification](#model.SchemaSpec).
    pub fn new(spec: SchemaSpec) -> Result<Arc<Self>, crate::model::errors::SchemaError> {
        use crate::model::errors::SchemaError;
        use crate::model::content::ContentMatch;

        let top_name = spec.top_node.as_deref().unwrap_or("doc");

        // --- Validation ---
        if !spec.nodes.contains_key(top_name) {
            return Err(SchemaError(format!(
                "Schema is missing its top node type ('{}')",
                top_name
            )));
        }
        if !spec.nodes.contains_key("text") {
            return Err(SchemaError("Every schema needs a 'text' type".to_string()));
        }
        if spec.nodes.get("text").map_or(false, |ns| !ns.attrs.is_empty()) {
            return Err(SchemaError(
                "The text node type should not have attributes".to_string(),
            ));
        }
        for name in spec.marks.keys() {
            if spec.nodes.contains_key(name.as_str()) {
                return Err(SchemaError(format!(
                    "{} can not be both a node and a mark",
                    name
                )));
            }
        }

        // --- First pass: create NodeType Arcs (refcount = 1) ---
        let mut node_types: IndexMap<String, Arc<NodeType>> = IndexMap::new();
        for (name, ns) in &spec.nodes {
            let is_block = !(ns.inline || name == "text");
            let groups: Vec<String> = ns
                .group
                .as_deref()
                .map(|g| g.split_whitespace().map(|s| s.to_string()).collect())
                .unwrap_or_default();
            let whitespace = ns.whitespace.unwrap_or(if ns.code {
                Whitespace::Pre
            } else {
                Whitespace::Normal
            });
            let da = default_attrs(&ns.attrs);
            node_types.insert(
                name.clone(),
                Arc::new(NodeType {
                    name: name.clone(),
                    is_block,
                    is_text: name == "text",
                    inline_content: false,
                    groups,
                    content_match: ContentMatch::empty(),
                    mark_set: None,
                    whitespace,
                    spec: ns.clone(),
                    default_attrs: da,
                }),
            );
        }

        // --- First pass: create MarkType Arcs (refcount = 1) ---
        let mut mark_types: IndexMap<String, Arc<MarkType>> = IndexMap::new();
        for (rank, (name, ms)) in spec.marks.iter().enumerate() {
            let da = default_attrs(&ms.attrs);
            mark_types.insert(
                name.clone(),
                Arc::new(MarkType {
                    name: name.clone(),
                    rank,
                    inclusive: ms.inclusive,
                    excluded: ExcludeSpec::SelfOnly,
                    spec: ms.clone(),
                    default_attrs: da,
                }),
            );
        }

        // --- Build helper maps for ContentMatch::parse ---
        let is_inline_map: IndexMap<String, bool> = node_types
            .iter()
            .map(|(n, nt)| (n.clone(), nt.is_inline()))
            .collect();
        let groups_map: IndexMap<String, Vec<String>> = {
            let mut map: IndexMap<String, Vec<String>> = IndexMap::new();
            for (name, nt) in &node_types {
                for g in &nt.groups {
                    map.entry(g.clone()).or_default().push(name.clone());
                }
            }
            map
        };
        let has_required_map: IndexMap<String, bool> = node_types
            .iter()
            .map(|(n, nt)| (n.clone(), nt.has_required_attrs()))
            .collect();

        // --- Second pass: fill NodeType (content_match, inline_content, mark_set) ---
        let mut linebreak_replacement: Option<Arc<NodeType>> = None;
        for (name, ns) in &spec.nodes {
            let nt = Arc::get_mut(node_types.get_mut(name).unwrap()).unwrap();
            let expr = ns.content.as_deref().unwrap_or("");
            let cm = ContentMatch::parse(expr, &is_inline_map, &groups_map, &has_required_map)
                .map_err(|e| SchemaError(e.0))?;
            nt.inline_content = cm.inline_content;
            nt.content_match = cm;
            nt.mark_set =
                resolve_mark_set(ns.marks.as_deref(), &mark_types, nt.inline_content)?;
            if ns.linebreak_replacement {
                if linebreak_replacement.is_some() {
                    return Err(SchemaError("Multiple linebreak nodes defined".to_string()));
                }
                if !nt.is_inline() || !nt.is_leaf() {
                    return Err(SchemaError(
                        "Linebreak replacement nodes must be inline leaf nodes".to_string(),
                    ));
                }
                linebreak_replacement = Some(node_types[name].clone());
            }
        }

        // --- Second pass: fill MarkType.excluded ---
        for (name, ms) in &spec.marks {
            let excluded = resolve_excluded(ms.excludes.as_deref(), name, &mark_types)?;
            Arc::get_mut(mark_types.get_mut(name).unwrap()).unwrap().excluded = excluded;
        }

        let top_node_type = node_types[top_name].clone();
        Ok(Arc::new(Schema {
            nodes: node_types,
            marks: mark_types,
            top_node_type,
            linebreak_replacement,
            spec,
        }))
    }
}

/// Gather MarkType instances for a list of mark name/group tokens.
///
/// Each token is either a direct mark type name or a group name that expands
/// to all marks whose spec.group contains that name.
fn gather_marks(
    tokens: &[&str],
    mark_types: &IndexMap<String, Arc<MarkType>>,
) -> Result<Vec<Arc<MarkType>>, crate::model::errors::SchemaError> {
    use crate::model::errors::SchemaError;
    let mut found: Vec<Arc<MarkType>> = vec![];
    for &token in tokens {
        if let Some(mt) = mark_types.get(token) {
            found.push(mt.clone());
        } else {
            let mut any = false;
            for mt in mark_types.values() {
                if mt
                    .spec
                    .group
                    .as_deref()
                    .is_some_and(|g| g.split_whitespace().any(|s| s == token))
                {
                    found.push(mt.clone());
                    any = true;
                }
            }
            if !any {
                return Err(SchemaError(format!("Unknown mark type: '{}'", token)));
            }
        }
    }
    Ok(found)
}

/// Resolve a NodeSpec.marks expression to a mark set.
///
/// - None + inline_content=true  → None (all marks allowed)
/// - None + inline_content=false → Some([]) (no marks)
/// - "_"                         → None (all marks allowed)
/// - ""                          → Some([]) (no marks)
/// - "em strong …"               → Some(gathered list)
fn resolve_mark_set(
    marks_expr: Option<&str>,
    mark_types: &IndexMap<String, Arc<MarkType>>,
    inline_content: bool,
) -> Result<Option<Vec<Arc<MarkType>>>, crate::model::errors::SchemaError> {
    match marks_expr {
        Some("_") => Ok(None),
        Some("") => Ok(Some(vec![])),
        Some(s) => {
            let tokens: Vec<&str> = s.split_whitespace().collect();
            Ok(Some(gather_marks(&tokens, mark_types)?))
        }
        None => {
            if inline_content {
                Ok(None)
            } else {
                Ok(Some(vec![]))
            }
        }
    }
}

/// Resolve a MarkSpec.excludes expression to an ExcludeSpec.
///
/// - None      → SelfOnly (default: a mark excludes marks of the same type)
/// - "_"       → All
/// - ""        → Named([]) (excludes nothing)
/// - "em …"    → Named(resolved type names, expanding group names)
fn resolve_excluded(
    excludes_expr: Option<&str>,
    _self_name: &str,
    mark_types: &IndexMap<String, Arc<MarkType>>,
) -> Result<ExcludeSpec, crate::model::errors::SchemaError> {
    use crate::model::errors::SchemaError;
    match excludes_expr {
        None => Ok(ExcludeSpec::SelfOnly),
        Some("_") => Ok(ExcludeSpec::All),
        Some("") => Ok(ExcludeSpec::Named(vec![])),
        Some(s) => {
            let mut names: Vec<String> = vec![];
            for token in s.split_whitespace() {
                if mark_types.contains_key(token) {
                    if !names.contains(&token.to_string()) {
                        names.push(token.to_string());
                    }
                } else {
                    let mut any = false;
                    for (mname, mt) in mark_types {
                        if mt
                            .spec
                            .group
                            .as_deref()
                            .is_some_and(|g| g.split_whitespace().any(|gr| gr == token))
                        {
                            if !names.contains(mname) {
                                names.push(mname.clone());
                            }
                            any = true;
                        }
                    }
                    if !any {
                        return Err(SchemaError(format!("Unknown mark type: '{}'", token)));
                    }
                }
            }
            Ok(ExcludeSpec::Named(names))
        }
    }
}
```

- [ ] **Step 4: 运行 Schema 基础测试，确认通过**

```bash
cargo test -p md-editor model::schema::tests::schema
```

Expected: 6 个基础 schema 测试全通过

- [ ] **Step 5: Commit**

```bash
git add src/model/schema.rs
git commit -m "feat(model): add Schema struct, Schema::new two-pass construction, and helper functions"
```

---

## Task 2：Schema 内容和 mark 集成测试

**Files:**
- Modify: `src/model/schema.rs`（仅追加测试）

- [ ] **Step 1: 追加 content_match / inline_content / mark_set 集成测试**

在 schema 基础测试之后追加：

```rust
    #[test]
    fn schema_paragraph_content_match_not_empty() {
        let s = Schema::new(minimal_schema_spec()).unwrap();
        let p = &s.nodes["paragraph"];
        // paragraph has "text*" content → content_match has at least one edge
        // (or valid_end=true with edge for text)
        assert!(p.content_match.valid_end); // text* can match zero texts
    }

    #[test]
    fn schema_paragraph_is_textblock() {
        let s = Schema::new(minimal_schema_spec()).unwrap();
        let p = &s.nodes["paragraph"];
        assert!(p.is_block);
        assert!(p.inline_content);
        assert!(p.is_textblock());
    }

    #[test]
    fn schema_text_is_leaf_inline() {
        let s = Schema::new(minimal_schema_spec()).unwrap();
        let t = &s.nodes["text"];
        assert!(t.is_text);
        assert!(t.is_leaf());
        assert!(t.is_inline());
    }

    #[test]
    fn schema_block_node_has_no_marks_by_default() {
        let s = Schema::new(minimal_schema_spec()).unwrap();
        let doc = &s.nodes["doc"];
        // doc has block content (paragraph+), not inline → mark_set = Some([])
        assert_eq!(doc.mark_set.as_ref().map(|v| v.len()), Some(0));
    }

    #[test]
    fn schema_inline_node_allows_all_marks_by_default() {
        let s = Schema::new(minimal_schema_spec()).unwrap();
        let p = &s.nodes["paragraph"];
        // paragraph has inline content and no marks= spec → mark_set = None (all allowed)
        assert!(p.mark_set.is_none());
    }

    fn schema_with_marks_spec() -> SchemaSpec {
        use indexmap::indexmap;
        SchemaSpec {
            nodes: indexmap! {
                "doc".to_string() => NodeSpec {
                    content: Some("paragraph+".to_string()),
                    ..NodeSpec::default()
                },
                "paragraph".to_string() => NodeSpec {
                    content: Some("text*".to_string()),
                    marks: Some("em strong".to_string()),
                    group: Some("block".to_string()),
                    ..NodeSpec::default()
                },
                "text".to_string() => NodeSpec { inline: true, ..NodeSpec::default() },
            },
            marks: indexmap! {
                "em".to_string() => MarkSpec::default(),
                "strong".to_string() => MarkSpec::default(),
            },
            top_node: None,
        }
    }

    #[test]
    fn schema_builds_with_marks() {
        let s = Schema::new(schema_with_marks_spec()).unwrap();
        assert!(s.marks.contains_key("em"));
        assert!(s.marks.contains_key("strong"));
    }

    #[test]
    fn schema_marks_get_rank() {
        let s = Schema::new(schema_with_marks_spec()).unwrap();
        assert_eq!(s.marks["em"].rank, 0);
        assert_eq!(s.marks["strong"].rank, 1);
    }

    #[test]
    fn schema_mark_set_limited_to_specified() {
        let s = Schema::new(schema_with_marks_spec()).unwrap();
        let p = &s.nodes["paragraph"];
        let mark_set = p.mark_set.as_ref().unwrap();
        assert_eq!(mark_set.len(), 2);
        assert!(mark_set.iter().any(|m| m.name == "em"));
        assert!(mark_set.iter().any(|m| m.name == "strong"));
    }

    #[test]
    fn schema_mark_excludes_self_only_by_default() {
        let s = Schema::new(schema_with_marks_spec()).unwrap();
        let em = &s.marks["em"];
        assert!(em.excludes(em));
        assert!(!em.excludes(&s.marks["strong"]));
    }

    #[test]
    fn schema_mark_excludes_all() {
        use indexmap::indexmap;
        let spec = SchemaSpec {
            nodes: minimal_schema_spec().nodes,
            marks: indexmap! {
                "code".to_string() => MarkSpec {
                    excludes: Some("_".to_string()),
                    ..MarkSpec::default()
                },
                "em".to_string() => MarkSpec::default(),
            },
            top_node: None,
        };
        let s = Schema::new(spec).unwrap();
        let code = &s.marks["code"];
        let em = &s.marks["em"];
        // code excludes all → excludes em
        assert!(code.excludes(em));
        assert!(code.excludes(code));
    }

    #[test]
    fn schema_mark_set_all_with_underscore() {
        use indexmap::indexmap;
        let spec = SchemaSpec {
            nodes: {
                let mut nodes = minimal_schema_spec().nodes;
                // paragraph allows all marks via "_"
                nodes.get_mut("paragraph").unwrap().marks = Some("_".to_string());
                nodes
            },
            marks: indexmap! {
                "em".to_string() => MarkSpec::default(),
            },
            top_node: None,
        };
        let s = Schema::new(spec).unwrap();
        let p = &s.nodes["paragraph"];
        // "_" → None (all marks)
        assert!(p.mark_set.is_none());
    }

    #[test]
    fn schema_error_bad_content_expression() {
        use indexmap::indexmap;
        let spec = SchemaSpec {
            nodes: indexmap! {
                "doc".to_string() => NodeSpec {
                    content: Some("nonexistent_type+".to_string()),
                    ..NodeSpec::default()
                },
                "text".to_string() => NodeSpec { inline: true, ..NodeSpec::default() },
            },
            marks: IndexMap::new(),
            top_node: None,
        };
        assert!(Schema::new(spec).is_err());
    }
```

- [ ] **Step 2: 运行测试，确认通过**

```bash
cargo test -p md-editor model::schema::tests::schema
```

Expected: 6 + 14 = 20 个 schema 测试全通过

- [ ] **Step 3: Commit**

```bash
git add src/model/schema.rs
git commit -m "feat(model): add Schema content/mark integration tests"
```

---

## Task 3：Schema 公开查询方法 + stubs

**Files:**
- Modify: `src/model/schema.rs`

TS 对应：`Schema.nodeType(name)`、`Schema.node(...)`、`Schema.text(...)`、`Schema.mark(...)`、`Schema.nodeFromJSON`、`Schema.markFromJSON`

- [ ] **Step 1: 追加方法测试**

```rust
    #[test]
    fn schema_node_type_lookup_ok() {
        let s = Schema::new(minimal_schema_spec()).unwrap();
        let nt = s.node_type("paragraph").unwrap();
        assert_eq!(nt.name, "paragraph");
    }

    #[test]
    fn schema_node_type_lookup_err() {
        let s = Schema::new(minimal_schema_spec()).unwrap();
        assert!(s.node_type("nonexistent").is_err());
    }

    #[test]
    fn schema_mark_type_lookup_ok() {
        let s = Schema::new(schema_with_marks_spec()).unwrap();
        let mt = s.mark_type("em").unwrap();
        assert_eq!(mt.name, "em");
    }

    #[test]
    fn schema_mark_type_lookup_err() {
        let s = Schema::new(schema_with_marks_spec()).unwrap();
        assert!(s.mark_type("nonexistent").is_err());
    }
```

- [ ] **Step 2: 确认测试失败**

```bash
cargo test -p md-editor model::schema::tests::schema_node_type_lookup
```

Expected: FAIL（方法未定义）

- [ ] **Step 3: 在 `Schema` impl 块中添加公开方法（在 `new` 之后）**

```rust
    /// Look up a node type by name.
    ///
    /// @internal
    pub fn node_type(&self, name: &str) -> Result<&Arc<NodeType>, crate::model::errors::SchemaError> {
        self.nodes.get(name).ok_or_else(|| {
            crate::model::errors::SchemaError(format!("Unknown node type: {}", name))
        })
    }

    /// Look up a mark type by name.
    ///
    /// @internal
    pub fn mark_type(&self, name: &str) -> Result<&Arc<MarkType>, crate::model::errors::SchemaError> {
        self.marks.get(name).ok_or_else(|| {
            crate::model::errors::SchemaError(format!("Unknown mark type: {}", name))
        })
    }

    // ---- Phase 3 stubs (依赖 Node / Fragment / Mark) ----

    /// Create a node in this schema. The `type` may be a string or a `NodeType` instance.
    ///
    /// NOTE: stub — requires Node/Fragment/Mark (Phase 3).
    #[allow(unused)]
    pub fn node_stub(&self) -> ! {
        todo!("Schema::node requires Node/Fragment/Mark — implement in Phase 3")
    }

    /// Create a text node in the schema. Empty text nodes are not allowed.
    ///
    /// NOTE: stub — requires Node (Phase 3).
    #[allow(unused)]
    pub fn text_stub(&self) -> ! {
        todo!("Schema::text requires Node — implement in Phase 3")
    }

    /// Create a mark with the given type and attributes.
    ///
    /// NOTE: stub — requires Mark (Phase 3).
    #[allow(unused)]
    pub fn mark_stub(&self) -> ! {
        todo!("Schema::mark requires Mark — implement in Phase 3")
    }

    /// Deserialize a node from its JSON representation.
    ///
    /// NOTE: stub — requires Node (Phase 3).
    #[allow(unused)]
    pub fn node_from_json_stub(&self) -> ! {
        todo!("Schema::node_from_json requires Node — implement in Phase 3")
    }

    /// Deserialize a mark from its JSON representation.
    ///
    /// NOTE: stub — requires Mark (Phase 3).
    #[allow(unused)]
    pub fn mark_from_json_stub(&self) -> ! {
        todo!("Schema::mark_from_json requires Mark — implement in Phase 3")
    }
```

- [ ] **Step 4: 运行测试，确认通过**

```bash
cargo test -p md-editor model::schema::tests::schema
```

Expected: 全部 schema 测试通过

- [ ] **Step 5: Commit**

```bash
git add src/model/schema.rs
git commit -m "feat(model): add Schema::node_type, mark_type and stubs for Phase 3 methods"
```

---

## Task 4：全量验证 + 更新阶段计划

**Files:**
- Modify: `docs/superpowers/plans/model-phases.md`

- [ ] **Step 1: 全量测试**

```bash
cargo test -p md-editor
```

Expected（数量可能略有出入，全部通过）：
```
test result: ok. ~105 passed; 0 failed
```

分布：
- `model::errors::tests` — 5 个
- `model::compare_deep::tests` — 8 个
- `model::schema::tests` — 原有 21 个 + Phase 2.2 新增 32 个 + Phase 2.3 新增 24 个 = ~77 个
- `model::content::tests` — 26 个

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

将 `docs/superpowers/plans/model-phases.md` 中：
```
| Phase 2.3 | Schema::new 两遍构建 | schema.rs 续（Schema::new）+ tests_schema | Phase 2.2 | ⬜ 待开始 |
```
改为：
```
| Phase 2.3 | Schema::new 两遍构建 | schema.rs 续（Schema::new）+ tests_schema | Phase 2.2 | ✅ 完成 |
```

- [ ] **Step 5: Commit**

```bash
git add docs/superpowers/plans/model-phases.md
git commit -m "docs: mark model Phase 2.3 as complete"
```

---

## 自检

### Spec 覆盖

| TS 功能 | Rust 实现 | 任务 |
|---|---|---|
| Schema 构造器：验证 topNode | `new()` 校验 top_name | Task 1 |
| Schema 构造器：验证 text 节点 | `new()` 校验 "text" 存在 | Task 1 |
| Schema 构造器：验证 text 无 attr | `new()` 校验 text.attrs 为空 | Task 1 |
| Schema 构造器：验证名称不重复 | `new()` 检查 node/mark 名称冲突 | Task 1 |
| NodeType.compile：第一遍创建 | 第一遍循环 | Task 1 |
| MarkType.compile：第一遍创建 | 第一遍循环 | Task 1 |
| contentMatch 设置 | 第二遍 `Arc::get_mut` | Task 1 |
| inlineContent 设置 | 第二遍从 cm.inline_content | Task 1 |
| markSet 设置（包含 null/"_"/""/"x y z"）| `resolve_mark_set` | Task 1 |
| linebreakReplacement 检测 | 第二遍 linebreak_replacement | Task 1 |
| MarkType.excluded 设置 | `resolve_excluded` + 第二遍 | Task 1 |
| gatherMarks（展开组名）| `gather_marks` | Task 1 |
| Schema.topNodeType | `top_node_type` 字段 | Task 1 |
| Schema.nodeType(name) | `node_type(&self, name)` | Task 3 |
| Schema.node(...) | `node_stub` | Task 3 |
| Schema.text(...) | `text_stub` | Task 3 |
| Schema.mark(...) | `mark_stub` | Task 3 |
| Schema.nodeFromJSON | `node_from_json_stub` | Task 3 |
| Schema.markFromJSON | `mark_from_json_stub` | Task 3 |
| Schema.linebreakReplacement | `linebreak_replacement` 字段 | Task 1 |
| Schema.spec | `spec` 字段 | Task 1 |

### 注意事项

- `SchemaSpec` 被 `Schema::new` 消耗（move）后存入 `Schema.spec`，但第二遍循环需要访问 `spec.nodes` 和 `spec.marks`。由于第二遍只读，可以在第一遍之前建立所有辅助 map，然后第二遍通过已构建好的 `node_types` map 的 key 迭代（不直接访问 spec）。**实际实现中直接引用 `&spec.nodes` 和 `&spec.marks` 即可**——它们在 move 进 Schema 前仍有效。
- `MarkSpec` 在 Phase 1 已改为 `Clone`，`NodeSpec` 同样，所以 `.clone()` 没有问题。
- `gather_marks` 和 `resolve_excluded` 都要处理组名展开。如果 mark 有 `spec.group = Some("formatting")` 且 excludes 表达式为 `"formatting"`，应能正确展开。
