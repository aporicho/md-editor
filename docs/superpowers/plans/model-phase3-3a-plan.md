# model Phase 3.3-a Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 创建 `src/model/test_builder.rs`——一个仅在测试中可见的节点快速构建模块，供 Phase 3.3-c、3.4-b、3.4-c 的测试复用。

**Architecture:** 单文件 `test_builder.rs`，用 `#[cfg(test)]` 限制可见性。内部持有一个 `OnceLock<Arc<Schema>>` 的共享测试 schema，所有构建函数直接用它，调用方无需传 schema 参数。节点构建函数接收 `&[Node]`，Mark 构建函数接收 `&[Node]` 并返回 `Vec<Node>`（每个 node 叠加该 mark）。

**Tech Stack:** Rust 2024 edition, `std::sync::{Arc, OnceLock}`, `indexmap`, `serde_json`

**TS 参考：** `reference/prosemirror-model/test/test-node.ts`（了解测试用到了哪些节点/mark 类型）

---

## 文件结构

```
src/model/test_builder.rs   ← 新建：#[cfg(test)] 测试基础设施
src/model/mod.rs            ← 修改：声明 test_builder 模块
```

---

## 背景

当前测试文件（tests_diff.rs、tests_mark.rs）各自内联了 schema 构建和 helper 函数，无法复用。  
Phase 3.3-c 的 tests_node.ts 移植需要 doc/p/blockquote/ul/li/em/strong/br/hr/img 等，如果不做统一封装，每个测试文件都要重复几十行 schema 代码。

**测试 schema 节点类型（对应 prosemirror-test-builder 的标准 schema）：**

| Rust 名 | TS 名 | 类型 | 属性 | content |
|---|---|---|---|---|
| `doc` | doc | block | — | `"block+"` |
| `paragraph` | paragraph | block | — | `"inline*"`, group: "block" |
| `blockquote` | blockquote | block | — | `"block+"`, group: "block" |
| `bullet_list` | bullet_list | block | — | `"list_item+"`, group: "block" |
| `list_item` | list_item | block | — | `"paragraph block*"` |
| `heading` | heading | block | level: number(1) | `"inline*"`, group: "block" |
| `horizontal_rule` | horizontal_rule | block, leaf | — | —, group: "block" |
| `image` | image | inline, leaf | src: string(""), alt: string(""), title: string("") | — |
| `hard_break` | hard_break | inline, leaf | — | — |
| `text` | text | inline | — | — |

**测试 schema mark 类型：**

| Rust 名 | TS 名 | 属性 |
|---|---|---|
| `em` | em | — |
| `strong` | strong | — |
| `code` | code | — |
| `link` | link | href: string(required), title: string(null) |

---

## Task 1: 创建 test_builder.rs 骨架 + 声明模块

**Files:**
- Create: `src/model/test_builder.rs`
- Modify: `src/model/mod.rs`

- [ ] **Step 1: 创建 `src/model/test_builder.rs`**

```rust
// src/model/test_builder.rs
//
// 测试基础设施 — 仅在 #[cfg(test)] 下可见。
// 提供统一的测试 Schema 和节点快速构建函数。
//
use std::sync::{Arc, OnceLock};
use indexmap::indexmap;
use serde_json::json;

use crate::model::fragment::Fragment;
use crate::model::mark::Mark;
use crate::model::node::Node;
use crate::model::schema::{
    AttrSpec, Attrs, MarkSpec, NodeSpec, Schema, SchemaSpec,
};

// ---------------------------------------------------------------------------
// 共享测试 Schema
// ---------------------------------------------------------------------------

/// 返回对共享测试 Schema 的静态引用。
/// 该 schema 与 prosemirror-test-builder 的标准 schema 等价。
pub fn test_schema() -> &'static Arc<Schema> {
    static SCHEMA: OnceLock<Arc<Schema>> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        Schema::new(SchemaSpec {
            nodes: indexmap! {
                "doc".to_string() => NodeSpec {
                    content: Some("block+".to_string()),
                    ..NodeSpec::default()
                },
                "paragraph".to_string() => NodeSpec {
                    content: Some("inline*".to_string()),
                    group: Some("block".to_string()),
                    ..NodeSpec::default()
                },
                "blockquote".to_string() => NodeSpec {
                    content: Some("block+".to_string()),
                    group: Some("block".to_string()),
                    ..NodeSpec::default()
                },
                "bullet_list".to_string() => NodeSpec {
                    content: Some("list_item+".to_string()),
                    group: Some("block".to_string()),
                    ..NodeSpec::default()
                },
                "list_item".to_string() => NodeSpec {
                    content: Some("paragraph block*".to_string()),
                    ..NodeSpec::default()
                },
                "heading".to_string() => NodeSpec {
                    content: Some("inline*".to_string()),
                    group: Some("block".to_string()),
                    attrs: indexmap! {
                        "level".to_string() => AttrSpec {
                            default: Some(json!(1)),
                            validate: None,
                        },
                    },
                    ..NodeSpec::default()
                },
                "horizontal_rule".to_string() => NodeSpec {
                    group: Some("block".to_string()),
                    ..NodeSpec::default()
                },
                "image".to_string() => NodeSpec {
                    inline: true,
                    attrs: indexmap! {
                        "src".to_string() => AttrSpec {
                            default: Some(json!("")),
                            validate: None,
                        },
                        "alt".to_string() => AttrSpec {
                            default: Some(json!("")),
                            validate: None,
                        },
                        "title".to_string() => AttrSpec {
                            default: Some(json!("")),
                            validate: None,
                        },
                    },
                    ..NodeSpec::default()
                },
                "hard_break".to_string() => NodeSpec {
                    inline: true,
                    ..NodeSpec::default()
                },
                "text".to_string() => NodeSpec {
                    inline: true,
                    ..NodeSpec::default()
                },
            },
            marks: indexmap! {
                "em".to_string() => MarkSpec::default(),
                "strong".to_string() => MarkSpec::default(),
                "code".to_string() => MarkSpec::default(),
                "link".to_string() => MarkSpec {
                    attrs: indexmap! {
                        "href".to_string() => AttrSpec {
                            default: None,
                            validate: None,
                        },
                        "title".to_string() => AttrSpec {
                            default: Some(json!(null)),
                            validate: None,
                        },
                    },
                    ..MarkSpec::default()
                },
            },
            top_node: None,
        })
        .expect("test_builder schema construction failed")
    })
}

fn empty_attrs() -> Attrs {
    Arc::new(Default::default())
}

fn make_node(type_name: &str, attrs: Attrs, children: &[Node]) -> Node {
    let s = test_schema();
    let nt = s.nodes[type_name].clone();
    let content = Fragment::from_array(children);
    Node::new(nt, attrs, content, vec![])
}

fn make_leaf(type_name: &str, attrs: Attrs) -> Node {
    let s = test_schema();
    let nt = s.nodes[type_name].clone();
    Node::new(nt, attrs, Fragment::empty(), vec![])
}

// ---------------------------------------------------------------------------
// Text node builder
// ---------------------------------------------------------------------------

/// 创建纯文本节点（无 mark）。
pub fn t(s: &str) -> Node {
    let schema = test_schema();
    schema.text(s.to_string(), vec![])
}

// ---------------------------------------------------------------------------
// Block node builders
// ---------------------------------------------------------------------------

/// 创建 doc 节点。
pub fn doc(children: &[Node]) -> Node {
    make_node("doc", empty_attrs(), children)
}

/// 创建 paragraph 节点。
pub fn p(children: &[Node]) -> Node {
    make_node("paragraph", empty_attrs(), children)
}

/// 创建 blockquote 节点。
pub fn blockquote(children: &[Node]) -> Node {
    make_node("blockquote", empty_attrs(), children)
}

/// 创建 bullet_list 节点。
pub fn ul(children: &[Node]) -> Node {
    make_node("bullet_list", empty_attrs(), children)
}

/// 创建 list_item 节点。
pub fn li(children: &[Node]) -> Node {
    make_node("list_item", empty_attrs(), children)
}

/// 创建 heading 节点（level=1）。
pub fn h1(children: &[Node]) -> Node {
    let attrs = Arc::new([("level".to_string(), json!(1))].into_iter().collect());
    make_node("heading", attrs, children)
}

/// 创建 heading 节点（level=2）。
pub fn h2(children: &[Node]) -> Node {
    let attrs = Arc::new([("level".to_string(), json!(2))].into_iter().collect());
    make_node("heading", attrs, children)
}

// ---------------------------------------------------------------------------
// Inline leaf builders
// ---------------------------------------------------------------------------

/// 创建 hard_break 节点。
pub fn br() -> Node {
    make_leaf("hard_break", empty_attrs())
}

/// 创建 horizontal_rule 节点（block leaf）。
pub fn hr() -> Node {
    make_leaf("horizontal_rule", empty_attrs())
}

/// 创建 image 节点（src/alt/title 全用默认值 ""）。
pub fn img() -> Node {
    make_leaf("image", empty_attrs())
}

// ---------------------------------------------------------------------------
// Mark builders
// ---------------------------------------------------------------------------

/// 对 nodes 中的每个节点叠加 em mark，返回新的 Vec<Node>。
pub fn em(nodes: &[Node]) -> Vec<Node> {
    apply_mark(nodes, "em", None)
}

/// 对 nodes 中的每个节点叠加 strong mark，返回新的 Vec<Node>。
pub fn strong(nodes: &[Node]) -> Vec<Node> {
    apply_mark(nodes, "strong", None)
}

/// 对 nodes 中的每个节点叠加 code mark，返回新的 Vec<Node>。
pub fn code(nodes: &[Node]) -> Vec<Node> {
    apply_mark(nodes, "code", None)
}

/// 对 nodes 中的每个节点叠加 link mark（带 href），返回新的 Vec<Node>。
pub fn link(href: &str, nodes: &[Node]) -> Vec<Node> {
    let extra: std::collections::HashMap<String, serde_json::Value> =
        [("href".to_string(), json!(href))].into_iter().collect();
    apply_mark(nodes, "link", Some(&extra))
}

fn apply_mark(
    nodes: &[Node],
    mark_name: &str,
    attrs: Option<&std::collections::HashMap<String, serde_json::Value>>,
) -> Vec<Node> {
    let s = test_schema();
    let mt = &s.marks[mark_name];
    let mark = mt.create(mt, attrs).expect("mark creation failed");
    nodes
        .iter()
        .map(|node| {
            let new_marks = mark.add_to_set(&node.marks);
            node.with_marks(new_marks)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Smoke tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_builds_successfully() {
        let s = test_schema();
        assert!(s.nodes.contains_key("doc"));
        assert!(s.nodes.contains_key("paragraph"));
        assert!(s.nodes.contains_key("bullet_list"));
        assert!(s.nodes.contains_key("image"));
        assert!(s.marks.contains_key("em"));
        assert!(s.marks.contains_key("link"));
    }

    #[test]
    fn text_node_has_correct_size() {
        let n = t("hello");
        assert_eq!(n.node_size(), 5);
        assert!(n.is_text());
    }

    #[test]
    fn paragraph_size_is_two_plus_content() {
        // p("hi") → open(1) + "hi"(2) + close(1) = 4
        let n = p(&[t("hi")]);
        assert_eq!(n.node_size(), 4);
    }

    #[test]
    fn doc_contains_paragraphs() {
        let n = doc(&[p(&[t("foo")]), p(&[t("bar")])]);
        assert_eq!(n.child_count(), 2);
    }

    #[test]
    fn br_is_inline_leaf() {
        let n = br();
        assert!(n.is_leaf());
        assert!(n.is_inline());
    }

    #[test]
    fn hr_is_block_leaf() {
        let n = hr();
        assert!(n.is_leaf());
        assert!(n.is_block());
    }

    #[test]
    fn img_has_src_attr() {
        let n = img();
        assert_eq!(n.attrs.get("src").and_then(|v| v.as_str()), Some(""));
    }

    #[test]
    fn em_applies_mark() {
        let nodes = em(&[t("foo")]);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].marks.len(), 1);
        assert_eq!(nodes[0].marks[0].mark_type.name, "em");
    }

    #[test]
    fn strong_inside_em_has_two_marks() {
        // em([strong([t("quux")])])  → "quux" 有 em+strong 两个 mark，顺序由 rank 决定
        let inner = strong(&[t("quux")]);
        let outer = em(&inner);
        assert_eq!(outer.len(), 1);
        assert_eq!(outer[0].marks.len(), 2);
        let mark_names: Vec<&str> = outer[0].marks.iter().map(|m| m.mark_type.name.as_str()).collect();
        assert!(mark_names.contains(&"em"));
        assert!(mark_names.contains(&"strong"));
    }

    #[test]
    fn link_mark_has_href() {
        let nodes = link("https://example.com", &[t("click")]);
        assert_eq!(nodes.len(), 1);
        let mark = &nodes[0].marks[0];
        assert_eq!(mark.mark_type.name, "link");
        assert_eq!(mark.attrs.get("href").and_then(|v| v.as_str()), Some("https://example.com"));
    }

    #[test]
    fn nested_doc_to_string() {
        let n = doc(&[ul(&[
            li(&[p(&[t("hey")]), p(&[])]),
            li(&[p(&[t("foo")])]),
        ])]);
        assert_eq!(
            n.to_string_repr(),
            "doc(bullet_list(list_item(paragraph(\"hey\"), paragraph), list_item(paragraph(\"foo\"))))"
        );
    }

    #[test]
    fn ul_li_p_structure() {
        let n = doc(&[
            ul(&[
                li(&[p(&[t("a")])]),
                li(&[p(&[t("b")])]),
            ])
        ]);
        assert_eq!(n.child_count(), 1); // one bullet_list
        let bl = n.child(0).unwrap();
        assert_eq!(bl.child_count(), 2); // two list_items
    }
}
```

- [ ] **Step 2: 在 `src/model/mod.rs` 中声明模块**

在 mod.rs 末尾追加：

```rust
#[cfg(test)]
pub(crate) mod test_builder;
```

- [ ] **Step 3: 运行 `cargo check`**

```bash
cargo check -p md-editor
```

Expected: 无报错

- [ ] **Step 4: Commit**

```bash
git add src/model/test_builder.rs src/model/mod.rs
git commit -m "feat(model): add test_builder module with shared schema and node/mark builders"
```

---

## Task 2: 运行 smoke tests + 修复问题

**Files:**
- Modify: `src/model/test_builder.rs`（如有需要）

- [ ] **Step 1: 运行 test_builder smoke tests**

```bash
cargo test -p md-editor model::test_builder::tests
```

Expected: 12 个测试全部通过

如有失败，根据错误信息修复：
- `nested_doc_to_string` 最可能失败，因为 `to_string_repr` 对文本节点输出 `"hello"` 格式（带引号），对非叶节点输出 `typename(children)` 格式。检查 node.rs 的 `to_string_repr` 实现是否与预期一致。
- 如果失败，调整测试的预期字符串以匹配实际输出格式。

- [ ] **Step 2: 运行全量测试，确认没有回归**

```bash
cargo test -p md-editor
```

Expected: 全部通过（171 个 + 12 个新增 = 183 个）

- [ ] **Step 3: Commit（仅在有修复时）**

```bash
git add src/model/test_builder.rs
git commit -m "fix(model): adjust test_builder smoke tests to match actual output format"
```

---

## 自检

### Spec 覆盖

| 要求 | 任务 | 状态 |
|---|---|---|
| 共享测试 schema（OnceLock，无需传参） | Task 1 | ✅ |
| doc/p/blockquote/ul/li/h1/h2 | Task 1 | ✅ |
| br/hr/img（inline + block leaf） | Task 1 | ✅ |
| em/strong/code/link mark 构建函数 | Task 1 | ✅ |
| 嵌套 mark 叠加（em inside strong） | Task 1 smoke test | ✅ |
| 模块声明（#[cfg(test)]） | Task 1 | ✅ |
| 全量测试无回归 | Task 2 | ✅ |

### 注意事项

- `to_string_repr` 测试：TS 的 `toString()` 对文本节点输出带双引号的字符串（`"hey"`），对有内容的节点输出 `typename(children)`，对空节点输出 `typename`。Rust 的 `to_string_repr` 应遵循相同格式——检查 node.rs 第 269 行附近的实现。
- `apply_mark` 函数对每个 node 调用 `mark.add_to_set(&node.marks)`，这会按 rank 正确排序 marks（em rank < strong rank，所以 em+strong 的顺序是 em 在前 strong 在后）。
- `make_leaf` 对 horizontal_rule 传空 content——这是正确的，因为它是 leaf（is_leaf() 返回 true，content_match 无边且 valid_end=true）。
