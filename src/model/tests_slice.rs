// src/model/tests_slice.rs
//
// Phase 4.1 — Slice 单元测试。
// TS 的 test-slice.ts 测试 Node::slice()（依赖 ResolvedPos），将在 Phase 4.7 移植。
// 本文件直接构造 Slice/Fragment 做覆盖测试。

use crate::model::fragment::Fragment;
use crate::model::replace::Slice;
use crate::model::test_builder::{doc, p, t};

#[test]
fn empty_slice_has_zero_size_and_zero_open() {
    let s = Slice::empty();
    assert_eq!(s.open_start, 0);
    assert_eq!(s.open_end, 0);
    assert_eq!(s.content.size, 0);
    assert_eq!(s.size(), 0);
}

#[test]
fn size_subtracts_open_from_content_size() {
    // doc(p("hello")) 的 content 是 Fragment[p("hello")]，size = 2 + 5 = 7
    let d = doc(&[p(&[t("hello")])]);
    let content = d.content.clone();
    // open_start = 1, open_end = 1 ⇒ size = 7 - 1 - 1 = 5
    let s = Slice::new(content.clone(), 1, 1);
    assert_eq!(s.content.size, 7);
    assert_eq!(s.size(), 5);
}

#[test]
fn size_zero_open_equals_content_size() {
    let content = Fragment::from_node(p(&[t("abc")]));
    let s = Slice::new(content, 0, 0);
    assert_eq!(s.size(), 5); // p 的 size = 2+3 = 5
}

#[test]
fn eq_returns_true_for_identical_slices() {
    let content = Fragment::from_node(p(&[t("x")]));
    let s1 = Slice::new(content.clone(), 0, 0);
    let s2 = Slice::new(content, 0, 0);
    assert!(s1.eq(&s2));
}

#[test]
fn eq_returns_false_for_different_open_start() {
    let content = Fragment::from_node(p(&[t("x")]));
    let s1 = Slice::new(content.clone(), 0, 0);
    let s2 = Slice::new(content, 1, 0);
    assert!(!s1.eq(&s2));
}

#[test]
fn eq_returns_false_for_different_open_end() {
    let content = Fragment::from_node(p(&[t("x")]));
    let s1 = Slice::new(content.clone(), 0, 0);
    let s2 = Slice::new(content, 0, 1);
    assert!(!s1.eq(&s2));
}

#[test]
fn eq_returns_false_for_different_content() {
    let s1 = Slice::new(Fragment::from_node(p(&[t("a")])), 0, 0);
    let s2 = Slice::new(Fragment::from_node(p(&[t("b")])), 0, 0);
    assert!(!s1.eq(&s2));
}

#[test]
fn to_string_repr_includes_open_depths() {
    // content = <paragraph("hi")>，open_start = 0, open_end = 1
    let s = Slice::new(Fragment::from_node(p(&[t("hi")])), 0, 1);
    // 格式：<fragment_inner>(open_start,open_end)
    assert_eq!(s.to_string_repr(), "<paragraph(\"hi\")>(0,1)");
}

#[test]
fn to_string_repr_empty_slice() {
    let s = Slice::empty();
    assert_eq!(s.to_string_repr(), "<>(0,0)");
}

#[test]
fn to_json_empty_slice_returns_none() {
    let s = Slice::empty();
    assert!(s.to_json().is_none());
}

#[test]
fn to_json_non_empty_no_open_has_only_content() {
    let s = Slice::new(Fragment::from_node(p(&[t("hi")])), 0, 0);
    let json = s.to_json().unwrap();
    let obj = json.as_object().unwrap();
    assert!(obj.contains_key("content"));
    assert!(!obj.contains_key("openStart"));
    assert!(!obj.contains_key("openEnd"));
}

#[test]
fn to_json_includes_open_start_when_nonzero() {
    let s = Slice::new(Fragment::from_node(p(&[t("hi")])), 1, 0);
    let json = s.to_json().unwrap();
    let obj = json.as_object().unwrap();
    assert_eq!(obj.get("openStart").and_then(|v| v.as_u64()), Some(1));
    assert!(!obj.contains_key("openEnd"));
}

#[test]
fn to_json_includes_open_end_when_nonzero() {
    let s = Slice::new(Fragment::from_node(p(&[t("hi")])), 0, 1);
    let json = s.to_json().unwrap();
    let obj = json.as_object().unwrap();
    assert_eq!(obj.get("openEnd").and_then(|v| v.as_u64()), Some(1));
    assert!(!obj.contains_key("openStart"));
}

#[test]
fn to_json_includes_both_open_fields() {
    let s = Slice::new(Fragment::from_node(p(&[t("hi")])), 2, 3);
    let json = s.to_json().unwrap();
    let obj = json.as_object().unwrap();
    assert_eq!(obj.get("openStart").and_then(|v| v.as_u64()), Some(2));
    assert_eq!(obj.get("openEnd").and_then(|v| v.as_u64()), Some(3));
}

use crate::model::test_builder::test_schema;
use serde_json::{json, Value};

#[test]
fn from_json_null_returns_empty_slice() {
    let s = test_schema();
    let result = Slice::from_json(s, Some(&Value::Null)).unwrap();
    assert!(result.eq(&Slice::empty()));
}

#[test]
fn from_json_none_returns_empty_slice() {
    let s = test_schema();
    let result = Slice::from_json(s, None).unwrap();
    assert!(result.eq(&Slice::empty()));
}

#[test]
fn from_json_content_only_defaults_open_to_zero() {
    let s = test_schema();
    // 构造一个含 paragraph("hi") 的 slice 的 JSON
    let original = Slice::new(Fragment::from_node(p(&[t("hi")])), 0, 0);
    let json = original.to_json().unwrap();
    let restored = Slice::from_json(s, Some(&json)).unwrap();
    assert_eq!(restored.open_start, 0);
    assert_eq!(restored.open_end, 0);
    assert!(restored.eq(&original));
}

#[test]
fn from_json_with_open_fields() {
    let s = test_schema();
    let original = Slice::new(Fragment::from_node(p(&[t("hi")])), 1, 2);
    let json = original.to_json().unwrap();
    let restored = Slice::from_json(s, Some(&json)).unwrap();
    assert_eq!(restored.open_start, 1);
    assert_eq!(restored.open_end, 2);
    assert!(restored.eq(&original));
}

#[test]
fn from_json_invalid_open_start_type_errors() {
    let s = test_schema();
    let json = json!({
        "content": [{"type": "paragraph", "content": [{"type": "text", "text": "x"}]}],
        "openStart": "oops",
    });
    let err = Slice::from_json(s, Some(&json)).err().unwrap();
    assert!(err.0.contains("Invalid input for Slice.fromJSON"));
}

#[test]
fn from_json_invalid_open_end_type_errors() {
    let s = test_schema();
    let json = json!({
        "content": [{"type": "paragraph", "content": [{"type": "text", "text": "x"}]}],
        "openEnd": [],
    });
    let err = Slice::from_json(s, Some(&json)).err().unwrap();
    assert!(err.0.contains("Invalid input for Slice.fromJSON"));
}

#[test]
fn from_json_roundtrip() {
    let s = test_schema();
    let d = doc(&[p(&[t("hello")]), p(&[t("world")])]);
    let content = Fragment::from_node(d);
    let original = Slice::new(content, 1, 1);
    let json = original.to_json().unwrap();
    let restored = Slice::from_json(s, Some(&json)).unwrap();
    assert!(restored.eq(&original));
}
