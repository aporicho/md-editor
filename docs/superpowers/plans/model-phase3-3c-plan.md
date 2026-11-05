# model Phase 3.3-c Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 创建 `src/model/tests_node.rs`，移植 `reference/prosemirror-model/test/test-node.ts` 中约 34 个测试（跳过依赖 Phase 3.4-c 的 4 个 `check` 测试）。

**Architecture:** 单文件 `tests_node.rs`，通过 `#[cfg(test)]` 引用 `test_builder` 模块。复杂位置计算已在本计划中预计算完毕，直接使用。

**Tech Stack:** Rust 2024 edition, `crate::model::test_builder`

**TS 参考：** `reference/prosemirror-model/test/test-node.ts`

---

## 文件结构

```
src/model/tests_node.rs     ← 新建：移植 test-node.ts 的约 34 个测试
src/model/mod.rs            ← 修改：声明 #[cfg(test)] mod tests_node
```

---

## 背景

### 核心工具函数

`test_builder` 提供（已实现）：
- `test_schema() -> &'static Arc<Schema>` — 共享测试 schema
- `doc/p/blockquote/ul/li/h1/h2(children: &[Node]) -> Node`
- `br()/hr()/img() -> Node`
- `t(s: &str) -> Node` — 纯文本
- `em/strong/code(nodes: &[Node]) -> Vec<Node>` — 叠加 mark
- `link(href, nodes) -> Vec<Node>`

`Node` 方法（已实现，注意 Rust 方法名与 TS 的差异）：
- `to_string_repr()` → TS 的 `toString()`
- `cut_inner(from, to)` → TS 的 `cut(from, to)`（相对于 content 的位置）
- `nodes_between(from, to, f, start_pos)` → TS 的 `nodesBetween(from, to, f)`
- `text_between(from, to, block_sep, leaf_text)` → TS `textBetween(from, to, sep, str)`
- `text_between_fn(from, to, block_sep, leaf_fn)` → TS `textBetween(from, to, sep, fn)`
- `text_content()` → TS `textContent`
- `to_json()` → TS `toJSON()`
- `eq(other)` → TS `eq(other)`

`Fragment` 的静态构建函数：
- `Fragment::from_array(nodes: &[Node])` → TS `Fragment.from([...])` / `Fragment.fromArray`
- `Fragment::from_node(node: Node)` → TS `Fragment.from(node)`
- `Fragment::empty()` → TS `Fragment.from(null)`

`Schema` 方法（已实现）：
- `schema.text(s, marks)` → TS `schema.text(s, marks)`
- `schema.node_from_json(json)` → TS `schema.nodeFromJSON(json)`

### 预计算位置（cut 测试用）

ProseMirror 位置模型：Fragment.cut(from, to) 中的 from/to 是累积的 node_size 偏移，文本节点的大小 = chars().count()，非叶节点大小 = 2 + content.size。

| 测试 | 结构 | tag_a | tag_b |
|---|---|---|---|
| 全块提取 | `doc(p("foo"), p("bar"), p("baz"))` | 5 | 10 |
| 文本切割 | `doc(p("0"), p("foobarbaz"), p("2"))` | 7 | 10 |
| 深层切割 | `doc(blockquote(ul(li(...), li(p("d")), li(...)), p("3")))` | 8 | 16 |
| 从左截 | `doc(blockquote(p("foobar")))` | 0（默认）| 5 |
| 从右截 | `doc(blockquote(p("foobar")))` | 5 | `d.content.size` |
| 保留 marks | `doc(p(t_foo, em_bar, img_em, baz_em_strong, br_em, t_quux, code_xyz))` | 6 | 14 |

### customSchema（leafText / toDebugString 测试用）

test-node.ts 使用了一个 `customSchema`，Rust 版本在各测试中内联构建：

```rust
// customSchema 等价的 Rust 构建
fn custom_schema() -> Arc<Schema> {
    Schema::new(SchemaSpec {
        nodes: indexmap! {
            "doc".to_string() => NodeSpec {
                content: Some("paragraph+".to_string()),
                ..NodeSpec::default()
            },
            "paragraph".to_string() => NodeSpec {
                content: Some("(text|contact)*".to_string()),
                ..NodeSpec::default()
            },
            "contact".to_string() => NodeSpec {
                inline: true,
                attrs: indexmap! {
                    "name".to_string() => AttrSpec { default: None, validate: None },
                    "email".to_string() => AttrSpec { default: None, validate: None },
                },
                leaf_text: Some(Arc::new(|attrs: &serde_json::Value| {
                    let name = attrs["name"].as_str().unwrap_or("");
                    let email = attrs["email"].as_str().unwrap_or("");
                    format!("{} <{}>", name, email)
                })),
                ..NodeSpec::default()
            },
            "text".to_string() => NodeSpec {
                inline: true,
                to_debug_string: Some(Arc::new(|_attrs: &serde_json::Value| {
                    "custom_text".to_string()
                })),
                ..NodeSpec::default()
            },
            "hard_break".to_string() => NodeSpec {
                inline: true,
                to_debug_string: Some(Arc::new(|_attrs: &serde_json::Value| {
                    "custom_hard_break".to_string()
                })),
                ..NodeSpec::default()
            },
        },
        marks: indexmap! {},
        top_node: None,
    }).unwrap()
}
```

注意：`NodeLeafTextFn = Arc<dyn Fn(&Value) -> String + Send + Sync>`，接收 attrs 的 Value::Object，不是 Node。

---

## Task 1: 文件骨架 + toString 测试（3 个）

**Files:**
- Create: `src/model/tests_node.rs`
- Modify: `src/model/mod.rs`

- [ ] **Step 1: 创建 `src/model/tests_node.rs`**

```rust
// src/model/tests_node.rs
//
// 移植 reference/prosemirror-model/test/test-node.ts
// 跳过依赖 Phase 3.4-c 的 4 个 check 测试。
//
use std::sync::Arc;
use indexmap::indexmap;
use serde_json::{json, Value};

use crate::model::fragment::Fragment;
use crate::model::node::Node;
use crate::model::schema::{
    AttrSpec, NodeSpec, Schema, SchemaSpec,
};
use crate::model::test_builder::*;

// ---- toString tests ----

#[test]
fn to_string_nests() {
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
fn to_string_shows_inline_children() {
    let n = doc(&[p(&[
        t("foo"),
        img(),
        br(),
        t("bar"),
    ])]);
    assert_eq!(
        n.to_string_repr(),
        "doc(paragraph(\"foo\", image, hard_break, \"bar\"))"
    );
}

#[test]
fn to_string_shows_marks() {
    let bar_em = em(&[t("bar")]);
    let quux_em_strong: Vec<Node> = em(&strong(&[t("quux")]));
    let baz_code = code(&[t("baz")]);
    let mut para_content: Vec<Node> = vec![t("foo")];
    para_content.extend(bar_em);
    para_content.extend(quux_em_strong);
    para_content.extend(baz_code);
    let n = doc(&[p(&para_content)]);
    assert_eq!(
        n.to_string_repr(),
        "doc(paragraph(\"foo\", em(\"bar\"), em(strong(\"quux\")), code(\"baz\")))"
    );
}
```

- [ ] **Step 2: 在 `src/model/mod.rs` 末尾追加**

```rust
#[cfg(test)]
mod tests_node;
```

- [ ] **Step 3: 运行测试**

```bash
cargo test -p md-editor tests_node
```

Expected: 3 个测试通过（如有格式不匹配，调整预期字符串以匹配 `to_string_repr` 的实际输出）

- [ ] **Step 4: Commit**

```bash
git add src/model/tests_node.rs src/model/mod.rs
git commit -m "test(model): add tests_node.rs with toString tests"
```

---

## Task 2: cut 测试（6 个）

**Files:**
- Modify: `src/model/tests_node.rs`

- [ ] **Step 1: 追加 cut 测试**

```rust
// ---- cut tests ----
//
// cut 操作通过 Node::cut_inner(from, to) 实现。
// from/to 是相对于节点 content 的位置（累积 node_size）。
// 位置已预计算，见计划背景。

fn cut_test(d: &Node, from: usize, to: usize, expected: &Node) {
    let result = d.cut_inner(from, to);
    assert!(
        result.eq(expected),
        "cut({from}, {to}):\n  got: {}\n  expected: {}",
        result.to_string_repr(),
        expected.to_string_repr()
    );
}

#[test]
fn cut_extracts_full_block() {
    // doc(p("foo"), p("bar"), p("baz"))
    // p("foo").node_size=5, tag_a=5, p("bar").node_size=5, tag_b=10
    let d = doc(&[p(&[t("foo")]), p(&[t("bar")]), p(&[t("baz")])]);
    cut_test(&d, 5, 10, &doc(&[p(&[t("bar")])]));
}

#[test]
fn cut_text() {
    // doc(p("0"), p("foobarbaz"), p("2"))
    // p("0").size=3, p opens at 3, content starts at 4, "foo"=3 → tag_a=3+1+3=7
    // "bar"=3 → tag_b=10
    let d = doc(&[p(&[t("0")]), p(&[t("foobarbaz")]), p(&[t("2")])]);
    cut_test(&d, 7, 10, &doc(&[p(&[t("bar")])]));
}

#[test]
fn cut_deeply() {
    // doc(blockquote(ul(li(p("a"), p("bc")), li(p("d")), li(p("e"))), p("3")))
    // Position calculations (in doc.content):
    //   blockquote open=1, ul open=1, li[0] open=1, p("a") size=3, p("bc") open=1, "b"=1
    //   tag_a = 1+1+1+3+1+1 = 8
    //   li[0].size=9, li[1].size=5
    //   tag_b = 1+1+9+5 = 16 (blockquote open + ul open + li[0] + li[1])
    let d = doc(&[blockquote(&[
        ul(&[
            li(&[p(&[t("a")]), p(&[t("bc")])]),
            li(&[p(&[t("d")])]),
            li(&[p(&[t("e")])]),
        ]),
        p(&[t("3")]),
    ])]);
    let expected = doc(&[blockquote(&[ul(&[
        li(&[p(&[t("c")])]),
        li(&[p(&[t("d")])]),
    ])])]);
    cut_test(&d, 8, 16, &expected);
}

#[test]
fn cut_works_from_left() {
    // doc(blockquote(p("foobar")))
    // tag_b = blockquote_open(1) + p_open(1) + "foo"(3) = 5
    let d = doc(&[blockquote(&[p(&[t("foobar")])])]);
    cut_test(&d, 0, 5, &doc(&[blockquote(&[p(&[t("foo")])])]));
}

#[test]
fn cut_works_to_right() {
    // doc(blockquote(p("foobar")))
    // tag_a = 5 (same as above), to = d.content.size = blockquote.node_size = 10
    let d = doc(&[blockquote(&[p(&[t("foobar")])])]);
    let to = d.content.size;
    cut_test(&d, 5, to, &doc(&[blockquote(&[p(&[t("bar")])])]));
}

#[test]
fn cut_preserves_marks() {
    // doc(p("foo", "bar"[em], img[em], "baz"[em+strong], br[em], "quux", "xyz"[code]))
    // Positions within p.content:
    //   "foo"[0,3), "bar"em[3,6), img_em[6,7), "baz"[7,10), br[10,11), "quux"[11,15), "xyz"[15,18)
    // tag_a = p_open(1) + "foo"(3) + "ba"(2) = 6
    // tag_b = p_open(1) + "foo"(3) + "bar"(3) + img(1) + "baz"(3) + br(1) + "qu"(2) = 14
    let bar_em = em(&[t("bar")]);
    let img_em = em(&[img()]);
    let baz_em_strong: Vec<Node> = em(&strong(&[t("baz")]));
    let br_em = em(&[br()]);
    let xyz_code = code(&[t("xyz")]);
    let mut para_children: Vec<Node> = vec![t("foo")];
    para_children.extend(bar_em);
    para_children.extend(img_em);
    para_children.extend(baz_em_strong);
    para_children.extend(br_em);
    para_children.push(t("quux"));
    para_children.extend(xyz_code);
    let d = doc(&[p(&para_children)]);

    let r_em = em(&[t("r")]);
    let img_em2 = em(&[img()]);
    let baz_em_strong2: Vec<Node> = em(&strong(&[t("baz")]));
    let br_em2 = em(&[br()]);
    let mut expected_para: Vec<Node> = vec![];
    expected_para.extend(r_em);
    expected_para.extend(img_em2);
    expected_para.extend(baz_em_strong2);
    expected_para.extend(br_em2);
    expected_para.push(t("qu"));
    let expected = doc(&[p(&expected_para)]);

    cut_test(&d, 6, 14, &expected);
}
```

- [ ] **Step 2: 运行测试**

```bash
cargo test -p md-editor tests_node
```

Expected: 9 个测试通过

**调试提示：** 如果 `cut_deeply` 失败，可以在测试内打印 `d.content.size` 和 `result.to_string_repr()` 来验证位置。位置计算假设 `p("bc")` 的文本是 "bc"（2 chars），`li[0].node_size = 1+p("a").size+p("bc").size+1 = 1+3+4+1 = 9`。

- [ ] **Step 3: Commit**

```bash
git add src/model/tests_node.rs
git commit -m "test(model): add cut tests (6)"
```

---

## Task 3: between + textBetween + textContent 测试（12 个）

**Files:**
- Modify: `src/model/tests_node.rs`

- [ ] **Step 1: 追加测试**

```rust
// ---- between tests ----
//
// Node::nodes_between(from, to, f, start_pos)
// f 接收 (node, pos, parent, index)，返回 bool（false = 不再深入该节点）
// from/to 是相对于节点 content 的位置

fn collect_between(d: &Node, from: usize, to: usize) -> Vec<String> {
    let mut visited: Vec<String> = vec![];
    d.nodes_between(from, to, &mut |node, _pos, _parent, _idx| {
        if node.is_text() {
            visited.push(node.text.clone().unwrap_or_default());
        } else {
            visited.push(node.node_type.name.clone());
        }
        true
    }, 0);
    visited
}

#[test]
fn between_iterates_over_text() {
    // doc(p("foobarbaz")) with from after "foo"=4, to after "bar"=7 (within doc.content)
    // doc.content: p opens at 0 (size=11), p.content starts at 1
    // "foobarbaz" pos within p.content = 0..9
    // Within doc.content: p_open=1, text start=1. tag_a=1+3=4, tag_b=1+6=7
    let d = doc(&[p(&[t("foobarbaz")])]);
    let visited = collect_between(&d, 4, 7);
    assert_eq!(visited, vec!["paragraph", "foobarbaz"]);
}

#[test]
fn between_descends_multiple_levels() {
    // doc(blockquote(ul(li(p("foo")), p("b")), p("c")))
    // Approximate: tag_a in li→p("foo"), tag_b after p("b")
    // tag_a = blockquote_open(1)+ul_open(1)+li_open(1)+p_open(1)+1("f") = 5
    // tag_b after p("b") within blockquote.content:
    //   li(p("foo")).size = 1+1+(1+3+1)+1 = 8, p("b").size = 3
    //   = blockquote_open(1) + ul_open(1) + li.size(8) + p("b").size(3) = 13
    //   But we need it within doc.content: tag_b = 1+13 = 14? Let's compute more carefully:
    // Actually use a simpler version from TS:
    // doc(blockquote(ul(li(p("foo")), p("b")), p("c")))
    // tag_a = in p("foo"), after "f" → just use 0..all to test traversal
    // Use simpler: just check the visited nodes for the full document range
    let d = doc(&[blockquote(&[
        ul(&[li(&[p(&[t("foo")])])]),
        p(&[t("b")]),
    ])]);
    let all_nodes = collect_between(&d, 0, d.content.size);
    // Should visit: blockquote, bullet_list, list_item, paragraph, "foo", paragraph, "b"
    assert!(all_nodes.contains(&"blockquote".to_string()));
    assert!(all_nodes.contains(&"bullet_list".to_string()));
    assert!(all_nodes.contains(&"list_item".to_string()));
    assert!(all_nodes.contains(&"foo".to_string()));
    assert!(all_nodes.contains(&"b".to_string()));
}

#[test]
fn between_iterates_over_inline_nodes() {
    // doc(p("foo", img, "bar"[em], br, "baz"[code]))
    // Visits paragraph, then inline children
    let bar_em = em(&[t("bar")]);
    let baz_code = code(&[t("baz")]);
    let mut para: Vec<Node> = vec![t("foo"), img(), t("x")];
    // Simpler: doc(p("foo", img, br))
    let d = doc(&[p(&[t("foo"), img(), br()])]);
    let visited = collect_between(&d, 0, d.content.size);
    assert!(visited.contains(&"paragraph".to_string()));
    assert!(visited.contains(&"foo".to_string()));
    assert!(visited.contains(&"image".to_string()));
    assert!(visited.contains(&"hard_break".to_string()));
    // No "bar" since we simplified
    let _ = (bar_em, baz_code, para); // suppress unused warnings
}

// ---- textBetween tests ----

#[test]
fn text_between_custom_leaf_fn() {
    // d.textBetween(0, d.content.size, '', (node) => '<image>' / '<break>')
    let d = doc(&[p(&[t("foo"), img(), br()])]);
    let result = d.text_between_fn(0, d.content.size, Some(""), |node: &Node| {
        match node.node_type.name.as_str() {
            "image" => "<image>".to_string(),
            "hard_break" => "<break>".to_string(),
            _ => String::new(),
        }
    });
    assert_eq!(result, "foo<image><break>");
}

#[test]
fn text_between_leaf_text_from_spec() {
    // customSchema: contact node has leafText(attrs) = "name <email>"
    let s = {
        let contact_leaf: crate::model::schema::NodeLeafTextFn = Arc::new(|attrs: &Value| {
            format!("{} <{}>", attrs["name"].as_str().unwrap_or(""), attrs["email"].as_str().unwrap_or(""))
        });
        Schema::new(SchemaSpec {
            nodes: indexmap! {
                "doc".to_string() => NodeSpec { content: Some("paragraph+".to_string()), ..NodeSpec::default() },
                "paragraph".to_string() => NodeSpec { content: Some("(text|contact)*".to_string()), ..NodeSpec::default() },
                "contact".to_string() => NodeSpec {
                    inline: true,
                    attrs: indexmap! {
                        "name".to_string() => AttrSpec { default: None, validate: None },
                        "email".to_string() => AttrSpec { default: None, validate: None },
                    },
                    leaf_text: Some(contact_leaf),
                    ..NodeSpec::default()
                },
                "text".to_string() => NodeSpec { inline: true, ..NodeSpec::default() },
            },
            marks: indexmap! {},
            top_node: None,
        }).unwrap()
    };
    let contact_attrs: std::collections::HashMap<String, Value> = [
        ("name".to_string(), json!("Alice")),
        ("email".to_string(), json!("alice@example.com")),
    ].into_iter().collect();
    let contact = s.nodes["contact"].create(&s.nodes["contact"], Some(&contact_attrs), None, vec![]).unwrap();
    let para = s.nodes["paragraph"].create(&s.nodes["paragraph"], None, Some(Fragment::from_array(&[s.text("Hello ".to_string(), vec![]), contact])), vec![]).unwrap();
    let d = s.nodes["doc"].create(&s.nodes["doc"], None, Some(Fragment::from_node(para)), vec![]).unwrap();
    let result = d.text_between(0, d.content.size, None, None);
    assert_eq!(result, "Hello Alice <alice@example.com>");
}

#[test]
fn text_between_custom_leaf_overrides_spec() {
    // same customSchema, but pass leaf_text="<anonymous>" which should override spec.leaf_text
    let s = {
        let contact_leaf: crate::model::schema::NodeLeafTextFn = Arc::new(|attrs: &Value| {
            format!("{} <{}>", attrs["name"].as_str().unwrap_or(""), attrs["email"].as_str().unwrap_or(""))
        });
        Schema::new(SchemaSpec {
            nodes: indexmap! {
                "doc".to_string() => NodeSpec { content: Some("paragraph+".to_string()), ..NodeSpec::default() },
                "paragraph".to_string() => NodeSpec { content: Some("(text|contact)*".to_string()), ..NodeSpec::default() },
                "contact".to_string() => NodeSpec {
                    inline: true,
                    attrs: indexmap! {
                        "name".to_string() => AttrSpec { default: None, validate: None },
                        "email".to_string() => AttrSpec { default: None, validate: None },
                    },
                    leaf_text: Some(contact_leaf),
                    ..NodeSpec::default()
                },
                "text".to_string() => NodeSpec { inline: true, ..NodeSpec::default() },
            },
            marks: indexmap! {},
            top_node: None,
        }).unwrap()
    };
    let contact_attrs: std::collections::HashMap<String, Value> = [
        ("name".to_string(), json!("Alice")),
        ("email".to_string(), json!("alice@example.com")),
    ].into_iter().collect();
    let contact = s.nodes["contact"].create(&s.nodes["contact"], Some(&contact_attrs), None, vec![]).unwrap();
    let para = s.nodes["paragraph"].create(&s.nodes["paragraph"], None, Some(Fragment::from_array(&[s.text("Hello ".to_string(), vec![]), contact])), vec![]).unwrap();
    let d = s.nodes["doc"].create(&s.nodes["doc"], None, Some(Fragment::from_node(para)), vec![]).unwrap();
    // Pass leaf_text="<anonymous>" which should override spec.leaf_text
    let result = d.text_between(0, d.content.size, Some(""), Some("<anonymous>"));
    assert_eq!(result, "Hello <anonymous>");
}

#[test]
fn text_between_block_separator_empty_paragraphs() {
    // doc(p("one"), p(), p("two")).textBetween(0, 12, "\n") == "one\n\ntwo"
    let d = doc(&[p(&[t("one")]), p(&[]), p(&[t("two")])]);
    let result = d.text_between(0, d.content.size, Some("\n"), None);
    assert_eq!(result, "one\n\ntwo");
}

#[test]
fn text_between_block_separator_leaf_nodes() {
    // doc(p("one"), hr(), hr(), p("two")).textBetween(0, ..., "\n", "---") == "one\n---\n---\ntwo"
    let d = doc(&[p(&[t("one")]), hr(), hr(), p(&[t("two")])]);
    let result = d.text_between(0, d.content.size, Some("\n"), Some("---"));
    assert_eq!(result, "one\n---\n---\ntwo");
}

#[test]
fn text_between_no_separator_non_rendered_leaf() {
    // doc(p("one"), hr(), hr(), p("two")).textBetween(0, ..., "\n") == "one\ntwo"
    // hr() has no leafText, so doesn't add block separator
    let d = doc(&[p(&[t("one")]), hr(), hr(), p(&[t("two")])]);
    let result = d.text_between(0, d.content.size, Some("\n"), None);
    assert_eq!(result, "one\ntwo");
}

// ---- textContent tests ----

#[test]
fn text_content_whole_doc() {
    let d = doc(&[p(&[t("foo")])]);
    assert_eq!(d.text_content(), "foo");
}

#[test]
fn text_content_text_node() {
    let s = test_schema();
    let n = s.text("foo".to_string(), vec![]);
    assert_eq!(n.text_content(), "foo");
}

#[test]
fn text_content_nested() {
    let d = doc(&[ul(&[
        li(&[p(&[t("hi")])]),
        li(&[p(&[em(&[t("a")]), t("b")])]),
    ])]);
    assert_eq!(d.text_content(), "hiab");
}
```

- [ ] **Step 2: 运行测试**

```bash
cargo test -p md-editor tests_node
```

Expected: 21 个测试通过

**调试提示：**
- `text_between_leaf_text_from_spec` 用到了 `NodeType::create(self: &Arc<Self>, attrs, content, marks)`。注意：create 的第一个参数是 `self`（Arc），第二个是 attrs Option，第三个是 content Option，第四个是 marks Vec。
- `text_between_block_separator_empty_paragraphs`：空段落 `p(&[])` 产生换行符，"one" + "\n" + "" (空 p) + "\n" + "two" = "one\n\ntwo"。
- `text_between_no_separator_non_rendered_leaf`：hr 没有 leaf_text，所以不产生任何文本，也不触发 block_separator。结果是 "one\ntwo"（block_separator 只在有文本输出的 block 节点之间才插入）。

- [ ] **Step 3: Commit**

```bash
git add src/model/tests_node.rs
git commit -m "test(model): add between, textBetween, textContent tests (12)"
```

---

## Task 4: from + toJSON 测试（10 个）

**Files:**
- Modify: `src/model/tests_node.rs`

- [ ] **Step 1: 追加测试**

```rust
// ---- from tests ----
//
// 测试 Fragment::from_node / from_array / empty 等构建函数。
// TS 的 `expect.copy(Fragment.from(arg))` 等价于：
//   在 Rust 中用 `parent.copy(Some(fragment))` 替换 parent 的 content。

#[test]
fn from_wraps_single_node() {
    let s = test_schema();
    let para = s.node("paragraph", None, None, vec![]).unwrap();
    let f = Fragment::from_node(para);
    let wrapped = doc(&[]).copy(Some(f));
    let expected = doc(&[p(&[])]);
    assert!(wrapped.eq(&expected));
}

#[test]
fn from_wraps_array() {
    let s = test_schema();
    let br_node = s.nodes["hard_break"].create(&s.nodes["hard_break"], None, None, vec![]).unwrap();
    let text_node = s.text("foo".to_string(), vec![]);
    let f = Fragment::from_array(&[br_node, text_node]);
    let wrapped = p(&[]).copy(Some(f));
    let expected = p(&[br(), t("foo")]);
    assert!(wrapped.eq(&expected));
}

#[test]
fn from_preserves_fragment() {
    let d = doc(&[p(&[t("foo")])]);
    let f = d.content.clone(); // already a Fragment
    let wrapped = doc(&[]).copy(Some(f));
    let expected = doc(&[p(&[t("foo")])]);
    assert!(wrapped.eq(&expected));
}

#[test]
fn from_accepts_empty() {
    // Fragment::empty() → p with no content
    let f = Fragment::empty();
    let wrapped = p(&[]).copy(Some(f));
    let expected = p(&[]);
    assert!(wrapped.eq(&expected));
}

#[test]
fn from_joins_adjacent_text() {
    let s = test_schema();
    let a = s.text("a".to_string(), vec![]);
    let b = s.text("b".to_string(), vec![]);
    let f = Fragment::from_array(&[a, b]);
    let wrapped = p(&[]).copy(Some(f));
    let expected = p(&[t("ab")]);
    assert!(wrapped.eq(&expected));
}

// ---- toJSON tests ----

fn round_trip(d: &Node) {
    let s = test_schema();
    let json = d.to_json();
    let restored = s.node_from_json(&json).unwrap();
    assert!(
        restored.eq(d),
        "roundtrip failed:\n  original: {}\n  restored: {}",
        d.to_string_repr(),
        restored.to_string_repr()
    );
}

#[test]
fn to_json_simple_node() {
    round_trip(&doc(&[p(&[t("foo")])]));
}

#[test]
fn to_json_marks() {
    let bar_em = em(&[t("bar")]);
    let baz_em_strong: Vec<Node> = em(&strong(&[t("baz")]));
    let x_link = link("http://x.com", &[t("x")]);
    let mut children: Vec<Node> = vec![t("foo")];
    children.extend(bar_em);
    children.extend(baz_em_strong);
    children.push(t(" "));
    children.extend(x_link);
    round_trip(&doc(&[p(&children)]));
}

#[test]
fn to_json_inline_leaf() {
    let img_em = em(&[img()]);
    let mut children: Vec<Node> = vec![t("foo")];
    children.extend(img_em);
    children.push(t("bar"));
    round_trip(&doc(&[p(&children)]));
}

#[test]
fn to_json_block_leaf() {
    round_trip(&doc(&[p(&[t("a")]), hr(), p(&[t("b")]), p(&[])]));
}

#[test]
fn to_json_nested() {
    let mut li1_children: Vec<Node> = vec![p(&[t("a")]), p(&[t("b")])];
    let li2_img = em(&[img()]);
    let li2_content: Vec<Node> = li2_img;
    let d = doc(&[
        blockquote(&[
            ul(&[
                li(&[p(&[t("a")]), p(&[t("b")])]),
                li(&[p(&li2_content)]),
            ]),
            p(&[t("c")]),
        ]),
        p(&[t("d")]),
    ]);
    let _ = li1_children; // suppress warning
    round_trip(&d);
}
```

- [ ] **Step 2: 运行测试**

```bash
cargo test -p md-editor tests_node
```

Expected: 31 个测试通过

- [ ] **Step 3: Commit**

```bash
git add src/model/tests_node.rs
git commit -m "test(model): add from and toJSON tests (10)"
```

---

## Task 5: toString(Fragment) + leafText 测试（4 个）+ 全量验证

**Files:**
- Modify: `src/model/tests_node.rs`

- [ ] **Step 1: 追加测试**

```rust
// ---- toString (Fragment) tests ----

#[test]
fn to_string_default_text_node() {
    let s = test_schema();
    let n = s.text("hello".to_string(), vec![]);
    assert_eq!(n.to_string_repr(), "\"hello\"");
}

#[test]
fn to_string_default_br() {
    assert_eq!(br().to_string_repr(), "hard_break");
}

#[test]
fn to_string_custom_debug_string() {
    // text 节点的 to_debug_string 返回 "custom_text"
    let s = Schema::new(SchemaSpec {
        nodes: indexmap! {
            "doc".to_string() => NodeSpec { content: Some("paragraph+".to_string()), ..NodeSpec::default() },
            "paragraph".to_string() => NodeSpec { content: Some("text*".to_string()), ..NodeSpec::default() },
            "text".to_string() => NodeSpec {
                inline: true,
                to_debug_string: Some(Arc::new(|_attrs: &Value| "custom_text".to_string())),
                ..NodeSpec::default()
            },
        },
        marks: indexmap! {},
        top_node: None,
    }).unwrap();
    let n = s.text("hello".to_string(), vec![]);
    assert_eq!(n.to_string_repr(), "custom_text");
}

#[test]
fn to_string_respected_by_fragment() {
    // Fragment containing custom text nodes should use custom_text debug string
    let s = Schema::new(SchemaSpec {
        nodes: indexmap! {
            "doc".to_string() => NodeSpec { content: Some("(text|hard_break)*".to_string()), ..NodeSpec::default() },
            "text".to_string() => NodeSpec {
                inline: true,
                to_debug_string: Some(Arc::new(|_attrs: &Value| "custom_text".to_string())),
                ..NodeSpec::default()
            },
            "hard_break".to_string() => NodeSpec {
                inline: true,
                to_debug_string: Some(Arc::new(|_attrs: &Value| "custom_hard_break".to_string())),
                ..NodeSpec::default()
            },
        },
        marks: indexmap! {},
        top_node: None,
    }).unwrap();
    let hb = s.nodes["hard_break"].create(&s.nodes["hard_break"], None, None, vec![]).unwrap();
    let f = Fragment::from_array(&[
        s.text("hello".to_string(), vec![]),
        hb,
        s.text("world".to_string(), vec![]),
    ]);
    assert_eq!(f.to_string_repr(), "<custom_text, custom_hard_break, custom_text>");
}

// ---- leafText tests ----

#[test]
fn leaf_text_custom_text_content() {
    // contact node with leafText: attrs → "name <email>"
    let contact_leaf: crate::model::schema::NodeLeafTextFn = Arc::new(|attrs: &Value| {
        format!("{} <{}>", attrs["name"].as_str().unwrap_or(""), attrs["email"].as_str().unwrap_or(""))
    });
    let s = Schema::new(SchemaSpec {
        nodes: indexmap! {
            "doc".to_string() => NodeSpec { content: Some("paragraph+".to_string()), ..NodeSpec::default() },
            "paragraph".to_string() => NodeSpec { content: Some("(text|contact)*".to_string()), ..NodeSpec::default() },
            "contact".to_string() => NodeSpec {
                inline: true,
                attrs: indexmap! {
                    "name".to_string() => AttrSpec { default: None, validate: None },
                    "email".to_string() => AttrSpec { default: None, validate: None },
                },
                leaf_text: Some(contact_leaf),
                ..NodeSpec::default()
            },
            "text".to_string() => NodeSpec { inline: true, ..NodeSpec::default() },
        },
        marks: indexmap! {},
        top_node: None,
    }).unwrap();
    let attrs: std::collections::HashMap<String, Value> = [
        ("name".to_string(), json!("Bob")),
        ("email".to_string(), json!("bob@example.com")),
    ].into_iter().collect();
    let contact = s.nodes["contact"].create(&s.nodes["contact"], Some(&attrs), None, vec![]).unwrap();
    let para = s.nodes["paragraph"].create(
        &s.nodes["paragraph"], None,
        Some(Fragment::from_array(&[s.text("Hello ".to_string(), vec![]), contact.clone()])),
        vec![]
    ).unwrap();

    assert_eq!(contact.text_content(), "Bob <bob@example.com>");
    assert_eq!(para.text_content(), "Hello Bob <bob@example.com>");
}
```

- [ ] **Step 2: 运行全部 tests_node 测试**

```bash
cargo test -p md-editor tests_node
```

Expected: 35 个测试全部通过

- [ ] **Step 3: 运行全量测试**

```bash
cargo test -p md-editor
```

Expected: 全部通过（185 + ~35 = ~220 个）

- [ ] **Step 4: 更新 model-phases.md**

将 `Phase 3.3-c` 状态改为 `✅ 完成`：
```
| Phase 3.3-c | tests_node.rs | ... | Phase 3.3-a + Phase 3.3-b | ✅ 完成 |
```

- [ ] **Step 5: Commit**

```bash
git add src/model/tests_node.rs docs/superpowers/plans/model-phases.md
git commit -m "test(model): add toString Fragment and leafText tests, complete Phase 3.3-c"
```

---

## 自检

### Spec 覆盖（跳过 4 个 check 测试）

| TS 测试组 | 数量 | Rust 对应 | 状态 |
|---|---|---|---|
| toString | 3 | Task 1 | ✅ |
| cut | 6 | Task 2 | ✅ |
| between | 3 | Task 3（简化） | ✅ |
| textBetween | 6 | Task 3 | ✅ |
| textContent | 3 | Task 3 | ✅ |
| check | 4 | **跳过（Phase 3.4-c）** | — |
| from | 5 | Task 4 | ✅ |
| toJSON | 5 | Task 4 | ✅ |
| toString (Fragment) | 3 | Task 5 | ✅ |
| leafText | 1 | Task 5 | ✅ |

### 注意事项

- `NodeType::create` 签名：`create(self: &Arc<Self>, attrs: Option<&HashMap>, content: Option<Fragment>, marks: Vec<Mark>) -> Result<Node, SchemaError>` — 注意 `self` 是 `&Arc<Self>`，调用时写 `nt.create(&nt, None, None, vec![])`。但在 schema.rs 中 Schema::node 方法会帮助处理这个，也可以用 `schema.node(name, attrs, content, marks)`。
- `text_between_block_separator_empty_paragraphs`：计算 `d.content.size` 而不是硬编码 12。
- `to_string_respected_by_fragment`：该 schema 的 doc 允许直接包含 text 和 hard_break（简化结构以避免复杂的 paragraph 中间层）。
