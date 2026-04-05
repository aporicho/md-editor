// src/model/tests_mark.rs
use std::sync::Arc;
use indexmap::indexmap;
use serde_json::json;

use crate::model::mark::Mark;
use crate::model::schema::{AttrSpec, MarkSpec, NodeSpec, Schema, SchemaSpec};

/// 构建测试用 Schema
pub(crate) fn test_schema() -> Arc<Schema> {
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
                excludes: Some("".to_string()),
                inclusive: false,
                ..MarkSpec::default()
            },
        },
        top_node: None,
    }).unwrap()
}

pub(crate) fn mk_em(s: &Arc<Schema>) -> Mark {
    s.marks["em"].create(&s.marks["em"], None).unwrap()
}

pub(crate) fn mk_strong(s: &Arc<Schema>) -> Mark {
    s.marks["strong"].create(&s.marks["strong"], None).unwrap()
}

pub(crate) fn mk_code(s: &Arc<Schema>) -> Mark {
    s.marks["code"].create(&s.marks["code"], None).unwrap()
}

pub(crate) fn mk_link(s: &Arc<Schema>, href: &str, title: Option<&str>) -> Mark {
    use std::collections::HashMap;
    let mut attrs: HashMap<String, serde_json::Value> = HashMap::new();
    attrs.insert("href".to_string(), json!(href));
    if let Some(t) = title {
        attrs.insert("title".to_string(), json!(t));
    }
    s.marks["link"].create(&s.marks["link"], Some(&attrs)).unwrap()
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
