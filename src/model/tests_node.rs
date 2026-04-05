// src/model/tests_node.rs
//
// 移植 reference/prosemirror-model/test/test-node.ts
// 跳过依赖 Phase 3.4-c 的 4 个 check 测试。
//
use crate::model::test_builder::*;
use crate::model::node::Node;

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
    let quux_strong = strong(&[t("quux")]);
    let quux_em_strong: Vec<_> = em(&quux_strong);
    let baz_code = code(&[t("baz")]);
    let mut para_content: Vec<_> = vec![t("foo")];
    para_content.extend(bar_em);
    para_content.extend(quux_em_strong);
    para_content.extend(baz_code);
    let n = doc(&[p(&para_content)]);
    assert_eq!(
        n.to_string_repr(),
        "doc(paragraph(\"foo\", em(\"bar\"), em(strong(\"quux\")), code(\"baz\")))"
    );
}

// ---- cut tests ----

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
    // p("foo").node_size=5 → tag_a=5, tag_b=10
    let d = doc(&[p(&[t("foo")]), p(&[t("bar")]), p(&[t("baz")])]);
    cut_test(&d, 5, 10, &doc(&[p(&[t("bar")])]));
}

#[test]
fn cut_text() {
    // p("0").size=3, p content starts at 4, "foo"=3 → tag_a=7, "bar"=3 → tag_b=10
    let d = doc(&[p(&[t("0")]), p(&[t("foobarbaz")]), p(&[t("2")])]);
    cut_test(&d, 7, 10, &doc(&[p(&[t("bar")])]));
}

#[test]
fn cut_deeply() {
    // Positions: blockquote_open(1)+ul_open(1)+li[0]_open(1)+p("a").size(3)+p("bc")_open(1)+"b"(1)=8
    // tag_b = blockquote_open(1)+ul_open(1)+li[0].size(9)+li[1].size(5) = 16
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
    // blockquote_open(1)+p_open(1)+"foo"(3) = 5
    let d = doc(&[blockquote(&[p(&[t("foobar")])])]);
    cut_test(&d, 0, 5, &doc(&[blockquote(&[p(&[t("foo")])])]));
}

#[test]
fn cut_works_to_right() {
    // tag_a=5, to=d.content.size
    let d = doc(&[blockquote(&[p(&[t("foobar")])])]);
    let to = d.content.size;
    cut_test(&d, 5, to, &doc(&[blockquote(&[p(&[t("bar")])])]));
}

#[test]
fn cut_preserves_marks() {
    // doc(p("foo", "bar"[em], img[em], "baz"[em+strong], br[em], "quux", "xyz"[code]))
    // p.content positions: "foo"[0,3), "bar"em[3,6), img_em[6,7), "baz"[7,10), br[10,11), "quux"[11,15), "xyz"[15,18)
    // tag_a = 1(p_open)+3("foo")+2("ba") = 6
    // tag_b = 1+3+3+1+3+1+2 = 14
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

// ---- between tests ----

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
    // doc(p("foobarbaz"))
    // In doc.content: p opens at 0 (node_size=11), p.content starts at offset 1.
    // "foobarbaz" within p.content: "foo"[0,3), gap at 3=tag_a, "bar"[3,6), gap at 6=tag_b
    // Within doc.content: tag_a=1+3=4, tag_b=1+6=7
    let d = doc(&[p(&[t("foobarbaz")])]);
    let visited = collect_between(&d, 4, 7);
    assert_eq!(visited, vec!["paragraph", "foobarbaz"]);
}

#[test]
fn between_descends_multiple_levels() {
    let d = doc(&[blockquote(&[
        ul(&[li(&[p(&[t("foo")])])]),
        p(&[t("b")]),
    ])]);
    let all_nodes = collect_between(&d, 0, d.content.size);
    assert!(all_nodes.contains(&"blockquote".to_string()));
    assert!(all_nodes.contains(&"bullet_list".to_string()));
    assert!(all_nodes.contains(&"list_item".to_string()));
    assert!(all_nodes.contains(&"foo".to_string()));
    assert!(all_nodes.contains(&"b".to_string()));
}

#[test]
fn between_iterates_over_inline_nodes() {
    let d = doc(&[p(&[t("foo"), img(), br()])]);
    let visited = collect_between(&d, 0, d.content.size);
    assert!(visited.contains(&"paragraph".to_string()));
    assert!(visited.contains(&"foo".to_string()));
    assert!(visited.contains(&"image".to_string()));
    assert!(visited.contains(&"hard_break".to_string()));
}

// ---- textBetween tests ----

#[test]
fn text_between_custom_leaf_fn() {
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

fn make_custom_schema() -> std::sync::Arc<crate::model::schema::Schema> {
    use indexmap::indexmap;
    use serde_json::json;
    use crate::model::schema::{AttrSpec, NodeSpec, Schema, SchemaSpec};
    let contact_leaf: crate::model::schema::NodeLeafTextFn = std::sync::Arc::new(|attrs: &serde_json::Value| {
        format!("{} <{}>",
            attrs["name"].as_str().unwrap_or(""),
            attrs["email"].as_str().unwrap_or(""))
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
}

fn make_doc_with_contact(s: &std::sync::Arc<crate::model::schema::Schema>) -> Node {
    use serde_json::json;
    use crate::model::fragment::Fragment;
    let attrs: std::collections::HashMap<String, serde_json::Value> = [
        ("name".to_string(), json!("Alice")),
        ("email".to_string(), json!("alice@example.com")),
    ].into_iter().collect();
    let contact_nt = s.nodes["contact"].clone();
    let contact = contact_nt.create(Some(&attrs), None, vec![]).unwrap();
    let para_nt = s.nodes["paragraph"].clone();
    let para = para_nt.create(
        None,
        Some(Fragment::from_array(&[s.text("Hello ".to_string(), vec![]), contact])),
        vec![]
    ).unwrap();
    let doc_nt = s.nodes["doc"].clone();
    doc_nt.create(None, Some(Fragment::from_node(para)), vec![]).unwrap()
}

#[test]
fn text_between_leaf_text_from_spec() {
    let s = make_custom_schema();
    let d = make_doc_with_contact(&s);
    let result = d.text_between(0, d.content.size, None, None);
    assert_eq!(result, "Hello Alice <alice@example.com>");
}

#[test]
fn text_between_custom_leaf_overrides_spec() {
    let s = make_custom_schema();
    let d = make_doc_with_contact(&s);
    let result = d.text_between(0, d.content.size, Some(""), Some("<anonymous>"));
    assert_eq!(result, "Hello <anonymous>");
}

#[test]
fn text_between_block_separator_empty_paragraphs() {
    let d = doc(&[p(&[t("one")]), p(&[]), p(&[t("two")])]);
    let result = d.text_between(0, d.content.size, Some("\n"), None);
    assert_eq!(result, "one\n\ntwo");
}

#[test]
fn text_between_block_separator_leaf_nodes() {
    let d = doc(&[p(&[t("one")]), hr(), hr(), p(&[t("two")])]);
    let result = d.text_between(0, d.content.size, Some("\n"), Some("---"));
    assert_eq!(result, "one\n---\n---\ntwo");
}

#[test]
fn text_between_no_separator_non_rendered_leaf() {
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
    let a_em = em(&[t("a")]);
    let d = doc(&[ul(&[
        li(&[p(&[t("hi")])]),
        li(&[p(&[a_em[0].clone(), t("b")])]),
    ])]);
    assert_eq!(d.text_content(), "hiab");
}

// ---- from tests ----

#[test]
fn from_wraps_single_node() {
    let para = p(&[]);
    let f = crate::model::fragment::Fragment::from_node(para);
    let wrapped = doc(&[]).copy(Some(f));
    let expected = doc(&[p(&[])]);
    assert!(wrapped.eq(&expected));
}

#[test]
fn from_wraps_array() {
    let br_node = br();
    let text_node = t("foo");
    let f = crate::model::fragment::Fragment::from_array(&[br_node, text_node]);
    let wrapped = p(&[]).copy(Some(f));
    let expected = p(&[br(), t("foo")]);
    assert!(wrapped.eq(&expected));
}

#[test]
fn from_preserves_fragment() {
    let d = doc(&[p(&[t("foo")])]);
    let f = d.content.clone();
    let wrapped = doc(&[]).copy(Some(f));
    let expected = doc(&[p(&[t("foo")])]);
    assert!(wrapped.eq(&expected));
}

#[test]
fn from_accepts_empty() {
    let f = crate::model::fragment::Fragment::empty();
    let wrapped = p(&[]).copy(Some(f));
    let expected = p(&[]);
    assert!(wrapped.eq(&expected));
}

#[test]
fn from_joins_adjacent_text() {
    let a = t("a");
    let b = t("b");
    let f = crate::model::fragment::Fragment::from_array(&[a, b]);
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
    let img_em = em(&[img()]);
    let d = doc(&[
        blockquote(&[
            ul(&[
                li(&[p(&[t("a")]), p(&[t("b")])]),
                li(&[p(&img_em)]),
            ]),
            p(&[t("c")]),
        ]),
        p(&[t("d")]),
    ]);
    round_trip(&d);
}

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
    use indexmap::indexmap;
    use std::sync::Arc;
    use crate::model::schema::{NodeSpec, Schema, SchemaSpec};
    let s = Schema::new(SchemaSpec {
        nodes: indexmap! {
            "doc".to_string() => NodeSpec { content: Some("paragraph+".to_string()), ..NodeSpec::default() },
            "paragraph".to_string() => NodeSpec { content: Some("text*".to_string()), ..NodeSpec::default() },
            "text".to_string() => NodeSpec {
                inline: true,
                to_debug_string: Some(Arc::new(|_attrs: &serde_json::Value| "custom_text".to_string())),
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
    use indexmap::indexmap;
    use std::sync::Arc;
    use crate::model::fragment::Fragment;
    use crate::model::schema::{NodeSpec, Schema, SchemaSpec};
    let s = Schema::new(SchemaSpec {
        nodes: indexmap! {
            "doc".to_string() => NodeSpec { content: Some("(text|hard_break)*".to_string()), ..NodeSpec::default() },
            "text".to_string() => NodeSpec {
                inline: true,
                to_debug_string: Some(Arc::new(|_attrs: &serde_json::Value| "custom_text".to_string())),
                ..NodeSpec::default()
            },
            "hard_break".to_string() => NodeSpec {
                inline: true,
                to_debug_string: Some(Arc::new(|_attrs: &serde_json::Value| "custom_hard_break".to_string())),
                ..NodeSpec::default()
            },
        },
        marks: indexmap! {},
        top_node: None,
    }).unwrap();
    let hb = s.nodes["hard_break"].create(None, None, vec![]).unwrap();
    let f = Fragment::from_array(&[
        s.text("hello".to_string(), vec![]),
        hb,
        s.text("world".to_string(), vec![]),
    ]);
    assert_eq!(f.to_string_repr(), "<custom_text, custom_hard_break, custom_text>");
}
