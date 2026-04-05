// src/model/test_builder.rs
//
// 测试基础设施 — 仅在 #[cfg(test)] 下可见。
// 提供统一的测试 Schema 和节点快速构建函数。
//
use std::sync::{Arc, OnceLock};
use indexmap::indexmap;
use serde_json::json;

use crate::model::fragment::Fragment;
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
                    group: Some("inline".to_string()),
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
                    group: Some("inline".to_string()),
                    ..NodeSpec::default()
                },
                "text".to_string() => NodeSpec {
                    inline: true,
                    group: Some("inline".to_string()),
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

fn attrs_map(attrs: &Attrs) -> std::collections::HashMap<String, serde_json::Value> {
    attrs.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
}

fn make_node(type_name: &str, attrs: Attrs, children: &[Node]) -> Node {
    let s = test_schema();
    let nt = s.nodes[type_name].clone();
    let content = Fragment::from_array(children);
    let resolved_attrs = if attrs.is_empty() {
        nt.create(None, Some(content.clone()), vec![])
            .expect("make_node failed")
            .attrs
    } else {
        let m = attrs_map(&attrs);
        nt.create(Some(&m), Some(content.clone()), vec![])
            .expect("make_node failed")
            .attrs
    };
    Node::new(nt, resolved_attrs, content, vec![])
}

fn make_leaf(type_name: &str, attrs: Attrs) -> Node {
    let s = test_schema();
    let nt = s.nodes[type_name].clone();
    if attrs.is_empty() {
        nt.create(None, None, vec![]).expect("make_leaf failed")
    } else {
        let m = attrs_map(&attrs);
        nt.create(Some(&m), None, vec![]).expect("make_leaf failed")
    }
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
    let mark = s.marks[mark_name]
        .create(&s.marks[mark_name], attrs)
        .expect("mark creation failed");
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
        let inner = strong(&[t("quux")]);
        let outer = em(&inner);
        assert_eq!(outer.len(), 1);
        assert_eq!(outer[0].marks.len(), 2);
        let mark_names: Vec<&str> =
            outer[0].marks.iter().map(|m| m.mark_type.name.as_str()).collect();
        assert!(mark_names.contains(&"em"));
        assert!(mark_names.contains(&"strong"));
    }

    #[test]
    fn link_mark_has_href() {
        let nodes = link("https://example.com", &[t("click")]);
        assert_eq!(nodes.len(), 1);
        let mark = &nodes[0].marks[0];
        assert_eq!(mark.mark_type.name, "link");
        assert_eq!(
            mark.attrs.get("href").and_then(|v| v.as_str()),
            Some("https://example.com")
        );
    }

    #[test]
    fn nested_doc_to_string() {
        let n = doc(&[ul(&[
            li(&[p(&[t("hey")]), p(&[])]),
            li(&[p(&[t("foo")])]),
        ])]);
        // 先打印实际输出再断言，方便调试
        let actual = n.to_string_repr();
        assert_eq!(
            actual,
            "doc(bullet_list(list_item(paragraph(\"hey\"), paragraph), list_item(paragraph(\"foo\"))))"
        );
    }

    #[test]
    fn ul_li_p_structure() {
        let n = doc(&[ul(&[
            li(&[p(&[t("a")])]),
            li(&[p(&[t("b")])]),
        ])]);
        assert_eq!(n.child_count(), 1);
        let bl = n.child(0).unwrap();
        assert_eq!(bl.child_count(), 2);
    }
}
