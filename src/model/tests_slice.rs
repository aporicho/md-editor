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
