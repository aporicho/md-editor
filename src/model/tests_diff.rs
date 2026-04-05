// src/model/tests_diff.rs
use std::sync::Arc;
use indexmap::indexmap;
use crate::model::fragment::Fragment;
use crate::model::node::Node;
use crate::model::schema::{Schema, SchemaSpec, NodeSpec, MarkSpec};

fn schema() -> Arc<Schema> {
    Schema::new(SchemaSpec {
        nodes: indexmap! {
            "doc".to_string() => NodeSpec { content: Some("paragraph+".to_string()), ..NodeSpec::default() },
            "paragraph".to_string() => NodeSpec { content: Some("text*".to_string()), ..NodeSpec::default() },
            "heading".to_string() => NodeSpec { content: Some("text*".to_string()), group: Some("block".to_string()), ..NodeSpec::default() },
            "text".to_string() => NodeSpec { inline: true, ..NodeSpec::default() },
        },
        marks: indexmap! {
            "em".to_string() => MarkSpec::default(),
            "strong".to_string() => MarkSpec::default(),
        },
        top_node: None,
    }).unwrap()
}

fn empty_attrs() -> Arc<std::collections::HashMap<String, serde_json::Value>> {
    Arc::new(Default::default())
}

fn text(s: &Arc<Schema>, t: &str) -> Node {
    Node::text_node(s.nodes["text"].clone(), empty_attrs(), t.to_string(), vec![])
}

fn text_em(s: &Arc<Schema>, t: &str) -> Node {
    let em = s.marks["em"].create(&s.marks["em"], None).unwrap();
    Node::text_node(s.nodes["text"].clone(), empty_attrs(), t.to_string(), vec![em])
}

fn text_strong(s: &Arc<Schema>, t: &str) -> Node {
    let strong = s.marks["strong"].create(&s.marks["strong"], None).unwrap();
    Node::text_node(s.nodes["text"].clone(), empty_attrs(), t.to_string(), vec![strong])
}

fn para(s: &Arc<Schema>, children: Vec<Node>) -> Node {
    let content = Fragment::from_array(&children);
    Node::new(s.nodes["paragraph"].clone(), empty_attrs(), content, vec![])
}

fn heading(s: &Arc<Schema>, children: Vec<Node>) -> Node {
    let content = Fragment::from_array(&children);
    Node::new(s.nodes["heading"].clone(), empty_attrs(), content, vec![])
}

fn doc(s: &Arc<Schema>, children: Vec<Node>) -> Node {
    let content = Fragment::from_array(&children);
    Node::new(s.nodes["doc"].clone(), empty_attrs(), content, vec![])
}

// ---- find_diff_start tests ----

#[test]
fn diff_start_identical_returns_none() {
    let s = schema();
    let a = doc(&s, vec![para(&s, vec![text(&s, "a"), text_em(&s, "b")]), para(&s, vec![text(&s, "hello")])]);
    let b = a.clone();
    assert!(a.content.find_diff_start(&b.content, 0).is_none());
}

#[test]
fn diff_start_one_longer() {
    let s = schema();
    let shared = vec![para(&s, vec![text(&s, "a")])];
    let mut a_children = shared.clone();
    a_children.push(para(&s, vec![text(&s, "extra")]));
    let a = doc(&s, a_children);
    let b = doc(&s, shared);
    // a has extra paragraph, diff at end of b's content
    let diff = a.content.find_diff_start(&b.content, 0);
    assert!(diff.is_some());
    // b has 1 paragraph of size 3 (open + "a" + close), so diff at 3
    assert_eq!(diff.unwrap(), 3);
}

#[test]
fn diff_start_different_marks() {
    let s = schema();
    let a = doc(&s, vec![para(&s, vec![text(&s, "a"), text_em(&s, "b")])]);
    let b = doc(&s, vec![para(&s, vec![text(&s, "a"), text_strong(&s, "b")])]);
    let diff = a.content.find_diff_start(&b.content, 0);
    assert!(diff.is_some());
    // doc opens at 0, paragraph opens at 0 (pos 0), "a" at pos 1 (size 1), diff at pos 2
    assert_eq!(diff.unwrap(), 2);
}

#[test]
fn diff_start_longer_text() {
    let s = schema();
    // a: para("foobar" + em("b")), b: para("foo" + em("b"))
    let a = doc(&s, vec![para(&s, vec![text(&s, "foobar"), text_em(&s, "b")])]);
    let b = doc(&s, vec![para(&s, vec![text(&s, "foo"), text_em(&s, "b")])]);
    let diff = a.content.find_diff_start(&b.content, 0);
    assert!(diff.is_some());
    // pos 0 = doc content start
    // child[0] = para, open at 0, content starts at 1
    // "foobar" vs "foo": common prefix "foo" (3 chars), diff at 1+3 = 4
    assert_eq!(diff.unwrap(), 4);
}

#[test]
fn diff_start_different_char() {
    let s = schema();
    let a = doc(&s, vec![para(&s, vec![text(&s, "foobar")])]);
    let b = doc(&s, vec![para(&s, vec![text(&s, "foocar")])]);
    let diff = a.content.find_diff_start(&b.content, 0);
    assert!(diff.is_some());
    // common prefix "foo" (3 chars), diff at 1+3 = 4
    assert_eq!(diff.unwrap(), 4);
}

#[test]
fn diff_start_different_node_type() {
    let s = schema();
    let a = doc(&s, vec![para(&s, vec![text(&s, "a")]), para(&s, vec![text(&s, "b")])]);
    let b = doc(&s, vec![para(&s, vec![text(&s, "a")]), heading(&s, vec![text(&s, "b")])]);
    let diff = a.content.find_diff_start(&b.content, 0);
    assert!(diff.is_some());
    // para("a") size = 1(open) + 1("a") + 1(close) = 3, diff at 3
    assert_eq!(diff.unwrap(), 3);
}

#[test]
fn diff_start_at_beginning() {
    let s = schema();
    let a = doc(&s, vec![para(&s, vec![text(&s, "b")])]);
    let b = doc(&s, vec![heading(&s, vec![text(&s, "b")])]);
    let diff = a.content.find_diff_start(&b.content, 0);
    assert!(diff.is_some());
    // Different node type at first child, diff at pos 0
    assert_eq!(diff.unwrap(), 0);
}

// ---- find_diff_end tests ----

#[test]
fn diff_end_identical_returns_none() {
    let s = schema();
    let a = doc(&s, vec![para(&s, vec![text(&s, "a"), text_em(&s, "b")]), para(&s, vec![text(&s, "hello")])]);
    let b = a.clone();
    let diff = a.content.find_diff_end(&b.content, a.content.size, b.content.size);
    assert!(diff.is_none());
}

#[test]
fn diff_end_one_longer() {
    let s = schema();
    let shared = vec![para(&s, vec![text(&s, "hello")])];
    let mut a_children = vec![para(&s, vec![text(&s, "extra")])];
    a_children.extend(shared.clone());
    let a = doc(&s, a_children);
    let b = doc(&s, shared);
    let diff = a.content.find_diff_end(&b.content, a.content.size, b.content.size);
    assert!(diff.is_some());
}

#[test]
fn diff_end_different_nodes() {
    let s = schema();
    let a = doc(&s, vec![para(&s, vec![text(&s, "a")]), para(&s, vec![text(&s, "b")])]);
    let b = doc(&s, vec![heading(&s, vec![text(&s, "a")]), para(&s, vec![text(&s, "b")])]);
    let diff = a.content.find_diff_end(&b.content, a.content.size, b.content.size);
    assert!(diff.is_some());
    let (da, _db) = diff.unwrap();
    // para("b") is same from end (size 3), diff is at first para/heading boundary
    let para_b_size = 3usize; // open + "b" + close
    assert_eq!(da, a.content.size - para_b_size);
}

#[test]
fn diff_end_longer_text() {
    let s = schema();
    // a: para("barfoo" + em("b")), b: para("foo" + em("b"))
    // From end: em("b") is same, then "barfoo" vs "foo" -- common suffix "foo"
    let a = doc(&s, vec![para(&s, vec![text(&s, "barfoo"), text_em(&s, "b")])]);
    let b = doc(&s, vec![para(&s, vec![text(&s, "foo"), text_em(&s, "b")])]);
    let diff = a.content.find_diff_end(&b.content, a.content.size, b.content.size);
    assert!(diff.is_some());
    // The "foo" suffix is common, so diff is at position after "bar" in a
    // a.content.size = 1(para open) + 6("barfoo") + 1("b" em) + 1(para close) = 9
    // em("b") is same (size 1), then "barfoo" vs "foo": common suffix "foo" (3),
    // pos_a ends up at 9 - 1(em) - 3(foo) = 5 (pointing after "bar")
    let (da, _db) = diff.unwrap();
    assert!(da > 0);
}
