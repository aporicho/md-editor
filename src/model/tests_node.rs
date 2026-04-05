// src/model/tests_node.rs
//
// 移植 reference/prosemirror-model/test/test-node.ts
// 跳过依赖 Phase 3.4-c 的 4 个 check 测试。
//
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
