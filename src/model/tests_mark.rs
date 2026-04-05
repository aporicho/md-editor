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

// ---- custom schema for exclusion tests ----

fn custom_schema() -> std::sync::Arc<Schema> {
    use indexmap::indexmap;
    Schema::new(SchemaSpec {
        nodes: indexmap! {
            "doc".to_string() => NodeSpec { content: Some("paragraph+".to_string()), ..NodeSpec::default() },
            "paragraph".to_string() => NodeSpec { content: Some("text*".to_string()), ..NodeSpec::default() },
            "text".to_string() => NodeSpec { inline: true, ..NodeSpec::default() },
        },
        marks: indexmap! {
            "remark".to_string() => MarkSpec {
                attrs: indexmap! { "id".to_string() => AttrSpec { default: None, validate: None } },
                excludes: Some("".to_string()),
                inclusive: false,
                ..MarkSpec::default()
            },
            "user".to_string() => MarkSpec {
                attrs: indexmap! { "id".to_string() => AttrSpec { default: None, validate: None } },
                excludes: Some("_".to_string()),
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

fn mk_custom(s: &std::sync::Arc<Schema>, name: &str, id: i64) -> Mark {
    use std::collections::HashMap;
    let mut attrs: HashMap<String, serde_json::Value> = HashMap::new();
    attrs.insert("id".to_string(), json!(id));
    s.marks[name].create(&s.marks[name], Some(&attrs)).unwrap()
}

fn mk_no_attr(s: &std::sync::Arc<Schema>, name: &str) -> Mark {
    s.marks[name].create(&s.marks[name], None).unwrap()
}

// ---- add_to_set 测试 ----

#[test]
fn add_to_set_empty_set() {
    let s = test_schema();
    let em = mk_em(&s);
    let result = em.add_to_set(&[]);
    assert!(Mark::same_set(&result, &[mk_em(&s)]));
}

#[test]
fn add_to_set_already_present() {
    let s = test_schema();
    let em = mk_em(&s);
    let result = em.add_to_set(&[em.clone()]);
    assert!(Mark::same_set(&result, &[mk_em(&s)]));
}

#[test]
fn add_to_set_lower_rank_before_others() {
    let s = test_schema();
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
fn add_to_set_replaces_excluded_link() {
    let s = test_schema();
    let result = mk_link(&s, "http://bar", None).add_to_set(&[mk_link(&s, "http://foo", None), mk_em(&s)]);
    assert!(Mark::same_set(&result, &[mk_em(&s), mk_link(&s, "http://bar", None)]));
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
    let result = mk_strong(&s).add_to_set(&[mk_em(&s), mk_code(&s)]);
    assert!(Mark::same_set(&result, &[mk_em(&s), mk_strong(&s), mk_code(&s)]));
}

#[test]
fn add_to_set_nonexclusive_allows_multiple() {
    let cs = custom_schema();
    let r1 = mk_custom(&cs, "remark", 1);
    let r2 = mk_custom(&cs, "remark", 2);
    let result = r2.add_to_set(&[r1.clone()]);
    assert!(Mark::same_set(&result, &[r1, r2]));
}

#[test]
fn add_to_set_no_duplicate_nonexclusive() {
    let cs = custom_schema();
    let r1 = mk_custom(&cs, "remark", 1);
    let result = r1.clone().add_to_set(&[r1.clone()]);
    assert!(Mark::same_set(&result, &[r1]));
}

#[test]
fn add_to_set_globally_excluding_clears_others() {
    let cs = custom_schema();
    let r1 = mk_custom(&cs, "remark", 1);
    let user1 = mk_custom(&cs, "user", 1);
    let em = mk_no_attr(&cs, "em");
    let result = user1.clone().add_to_set(&[r1, em]);
    assert!(Mark::same_set(&result, &[user1]));
}

#[test]
fn add_to_set_cannot_add_to_globally_excluding() {
    let cs = custom_schema();
    let user1 = mk_custom(&cs, "user", 1);
    let em = mk_no_attr(&cs, "em");
    let result = em.add_to_set(&[user1.clone()]);
    assert!(Mark::same_set(&result, &[user1]));
}

#[test]
fn add_to_set_globally_excluding_overwrites_itself() {
    let cs = custom_schema();
    let user1 = mk_custom(&cs, "user", 1);
    let user2 = mk_custom(&cs, "user", 2);
    let result = user2.clone().add_to_set(&[user1]);
    assert!(Mark::same_set(&result, &[user2]));
}

#[test]
fn add_to_set_removes_excluded_by_new() {
    let cs = custom_schema();
    let r1 = mk_custom(&cs, "remark", 1);
    let strong = mk_no_attr(&cs, "strong");
    let em = mk_no_attr(&cs, "em");
    let result = strong.clone().add_to_set(&[r1.clone(), em]);
    assert!(Mark::same_set(&result, &[r1, strong]));
}

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
    let result = strong.remove_from_set(&[em.clone()]);
    assert!(Mark::same_set(&result, &[em]));
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
    let j = em.to_json();
    assert_eq!(j["type"], json!("em"));
}

#[test]
fn mark_to_json_with_attrs() {
    let s = test_schema();
    let link = mk_link(&s, "http://foo", None);
    let j = link.to_json();
    assert_eq!(j["type"], json!("link"));
    assert_eq!(j["attrs"]["href"], json!("http://foo"));
}

#[test]
fn mark_from_json_roundtrip() {
    let s = test_schema();
    let link = mk_link(&s, "http://foo", None);
    let j = link.to_json();
    let restored = Mark::from_json(&s, &j).unwrap();
    assert!(link.eq_mark(&restored));
}

#[test]
fn mark_from_json_invalid_type() {
    let s = test_schema();
    let j = json!({ "type": "nonexistent" });
    assert!(Mark::from_json(&s, &j).is_err());
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
    let unsorted = vec![mk_strong(&s), mk_em(&s)];
    let sorted = Mark::set_from(Some(&unsorted));
    assert_eq!(sorted[0].mark_type.name, "em");
    assert_eq!(sorted[1].mark_type.name, "strong");
}
