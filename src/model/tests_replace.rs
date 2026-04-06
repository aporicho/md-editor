// src/model/tests_replace.rs
//
// Phase 4.7 — 移植 test-replace.ts 全部 22 个测试。
// 使用 tag 系统（TaggedNode）标记文档位置。

use crate::model::replace::Slice;
use crate::model::test_builder::{
    blockquote, doc, h1, li, p, t,
    tag_blockquote, tag_doc, tag_h1, tag_p, tag_ul,
    TChild, TaggedNode,
};

// ---------------------------------------------------------------------------
// 测试辅助函数
// ---------------------------------------------------------------------------

/// 成功替换测试：将 doc 中 [a, b) 范围替换为 insert 的 [a, b) slice，
/// 结果应与 expected 深度相等。
fn rpl(source: TaggedNode, insert: Option<TaggedNode>, expected: crate::model::node::Node) {
    let slice = match insert {
        Some(ins) => {
            let a = ins.tag["a"];
            let b = ins.tag["b"];
            ins.node.slice(a, Some(b), false)
                .expect("rpl: insert.slice failed")
        }
        None => Slice::empty(),
    };
    let a = source.tag["a"];
    let b = source.tag["b"];
    let result = source.node.replace(a, b, &slice)
        .expect("rpl: replace failed");
    assert!(
        result.eq(&expected),
        "replace mismatch:\n  expected: {}\n  got:      {}",
        expected.to_string_repr(),
        result.to_string_repr(),
    );
}

/// 失败替换测试：替换应返回 Err，且错误消息包含 pattern（大小写不敏感）。
fn bad(source: TaggedNode, insert: Option<TaggedNode>, pattern: &str) {
    let slice = match insert {
        Some(ins) => {
            let a = ins.tag["a"];
            let b = ins.tag["b"];
            ins.node.slice(a, Some(b), false)
                .expect("bad: insert.slice failed")
        }
        None => Slice::empty(),
    };
    let a = source.tag["a"];
    let b = source.tag["b"];
    let result = source.node.replace(a, b, &slice);
    match result {
        Ok(_) => panic!("Expected replace to fail with '{}', but it succeeded", pattern),
        Err(e) => {
            let err_msg = e.0.to_lowercase();
            assert!(
                err_msg.contains(&pattern.to_lowercase()),
                "Error '{}' doesn't match pattern '{}'",
                err_msg,
                pattern,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 成功替换测试 (16 个)
// ---------------------------------------------------------------------------

#[test]
fn joins_on_delete() {
    rpl(
        tag_doc(vec![tag_p(vec!["on<a>e".into()]).into(), tag_p(vec!["t<b>wo".into()]).into()]),
        None,
        doc(&[p(&[t("onwo")])]),
    );
}

#[test]
fn merges_matching_blocks() {
    rpl(
        tag_doc(vec![tag_p(vec!["on<a>e".into()]).into(), tag_p(vec!["t<b>wo".into()]).into()]),
        Some(tag_doc(vec![tag_p(vec!["xx<a>xx".into()]).into(), tag_p(vec!["yy<b>yy".into()]).into()])),
        doc(&[p(&[t("onxx")]), p(&[t("yywo")])]),
    );
}

#[test]
fn merges_when_adding_text() {
    rpl(
        tag_doc(vec![tag_p(vec!["on<a>e".into()]).into(), tag_p(vec!["t<b>wo".into()]).into()]),
        Some(tag_doc(vec![tag_p(vec!["<a>H<b>".into()]).into()])),
        doc(&[p(&[t("onHwo")])]),
    );
}

#[test]
fn can_insert_text() {
    rpl(
        tag_doc(vec![
            TChild::N(p(&[t("before")])),
            tag_p(vec!["on<a><b>e".into()]).into(),
            TChild::N(p(&[t("after")])),
        ]),
        Some(tag_doc(vec![tag_p(vec!["<a>H<b>".into()]).into()])),
        doc(&[p(&[t("before")]), p(&[t("onHe")]), p(&[t("after")])]),
    );
}

#[test]
fn doesnt_merge_non_matching_blocks() {
    rpl(
        tag_doc(vec![tag_p(vec!["on<a>e".into()]).into(), tag_p(vec!["t<b>wo".into()]).into()]),
        Some(tag_doc(vec![tag_h1(vec!["<a>H<b>".into()]).into()])),
        doc(&[p(&[t("onHwo")])]),
    );
}

#[test]
fn can_merge_a_nested_node() {
    rpl(
        tag_doc(vec![
            tag_blockquote(vec![
                tag_blockquote(vec![
                    tag_p(vec!["on<a>e".into()]).into(),
                    tag_p(vec!["t<b>wo".into()]).into(),
                ]).into(),
            ]).into(),
        ]),
        Some(tag_doc(vec![tag_p(vec!["<a>H<b>".into()]).into()])),
        doc(&[blockquote(&[blockquote(&[p(&[t("onHwo")])])])]),
    );
}

#[test]
fn can_replace_within_a_block() {
    rpl(
        tag_doc(vec![
            tag_blockquote(vec![
                tag_p(vec!["a<a>bc<b>d".into()]).into(),
            ]).into(),
        ]),
        Some(tag_doc(vec![tag_p(vec!["x<a>y<b>z".into()]).into()])),
        doc(&[blockquote(&[p(&[t("ayd")])])]),
    );
}

#[test]
fn can_insert_a_lopsided_slice() {
    rpl(
        tag_doc(vec![
            tag_blockquote(vec![
                tag_blockquote(vec![
                    tag_p(vec!["on<a>e".into()]).into(),
                    TChild::N(p(&[t("two")])),
                    "<b>".into(),
                    TChild::N(p(&[t("three")])),
                ]).into(),
            ]).into(),
        ]),
        Some(tag_doc(vec![
            tag_blockquote(vec![
                tag_p(vec!["aa<a>aa".into()]).into(),
                TChild::N(p(&[t("bb")])),
                TChild::N(p(&[t("cc")])),
                "<b>".into(),
                TChild::N(p(&[t("dd")])),
            ]).into(),
        ])),
        doc(&[blockquote(&[blockquote(&[
            p(&[t("onaa")]),
            p(&[t("bb")]),
            p(&[t("cc")]),
            p(&[t("three")]),
        ])])]),
    );
}

#[test]
fn can_insert_a_deep_lopsided_slice() {
    rpl(
        tag_doc(vec![
            tag_blockquote(vec![
                tag_blockquote(vec![
                    tag_p(vec!["on<a>e".into()]).into(),
                    TChild::N(p(&[t("two")])),
                    TChild::N(p(&[t("three")])),
                ]).into(),
                "<b>".into(),
                TChild::N(p(&[t("x")])),
            ]).into(),
        ]),
        Some(tag_doc(vec![
            tag_blockquote(vec![
                tag_p(vec!["aa<a>aa".into()]).into(),
                TChild::N(p(&[t("bb")])),
                TChild::N(p(&[t("cc")])),
            ]).into(),
            "<b>".into(),
            TChild::N(p(&[t("dd")])),
        ])),
        doc(&[blockquote(&[
            blockquote(&[p(&[t("onaa")]), p(&[t("bb")]), p(&[t("cc")])]),
            p(&[t("x")]),
        ])]),
    );
}

#[test]
fn can_merge_multiple_levels() {
    rpl(
        tag_doc(vec![
            tag_blockquote(vec![
                tag_blockquote(vec![
                    tag_p(vec!["hell<a>o".into()]).into(),
                ]).into(),
            ]).into(),
            tag_blockquote(vec![
                tag_blockquote(vec![
                    tag_p(vec!["<b>a".into()]).into(),
                ]).into(),
            ]).into(),
        ]),
        None,
        doc(&[blockquote(&[blockquote(&[p(&[t("hella")])])])]),
    );
}

#[test]
fn can_merge_multiple_levels_while_inserting() {
    rpl(
        tag_doc(vec![
            tag_blockquote(vec![
                tag_blockquote(vec![
                    tag_p(vec!["hell<a>o".into()]).into(),
                ]).into(),
            ]).into(),
            tag_blockquote(vec![
                tag_blockquote(vec![
                    tag_p(vec!["<b>a".into()]).into(),
                ]).into(),
            ]).into(),
        ]),
        Some(tag_doc(vec![tag_p(vec!["<a>i<b>".into()]).into()])),
        doc(&[blockquote(&[blockquote(&[p(&[t("hellia")])])])]),
    );
}

#[test]
fn can_insert_a_split() {
    rpl(
        tag_doc(vec![tag_p(vec!["foo<a><b>bar".into()]).into()]),
        Some(tag_doc(vec![
            tag_p(vec!["<a>x".into()]).into(),
            tag_p(vec!["y<b>".into()]).into(),
        ])),
        doc(&[p(&[t("foox")]), p(&[t("ybar")])]),
    );
}

#[test]
fn can_insert_a_deep_split() {
    rpl(
        tag_doc(vec![
            tag_blockquote(vec![
                tag_p(vec!["foo<a>x<b>bar".into()]).into(),
            ]).into(),
        ]),
        Some(tag_doc(vec![
            tag_blockquote(vec![tag_p(vec!["<a>x".into()]).into()]).into(),
            tag_blockquote(vec![tag_p(vec!["y<b>".into()]).into()]).into(),
        ])),
        doc(&[blockquote(&[p(&[t("foox")])]), blockquote(&[p(&[t("ybar")])])]),
    );
}

#[test]
fn can_add_a_split_one_level_up() {
    rpl(
        tag_doc(vec![
            tag_blockquote(vec![
                tag_p(vec!["foo<a>u".into()]).into(),
                tag_p(vec!["v<b>bar".into()]).into(),
            ]).into(),
        ]),
        Some(tag_doc(vec![
            tag_blockquote(vec![tag_p(vec!["<a>x".into()]).into()]).into(),
            tag_blockquote(vec![tag_p(vec!["y<b>".into()]).into()]).into(),
        ])),
        doc(&[blockquote(&[p(&[t("foox")])]), blockquote(&[p(&[t("ybar")])])]),
    );
}

#[test]
fn keeps_the_node_type_of_the_left_node() {
    rpl(
        tag_doc(vec![
            tag_h1(vec!["foo<a>bar".into()]).into(),
            "<b>".into(),
        ]),
        Some(tag_doc(vec![
            tag_p(vec!["foo<a>baz".into()]).into(),
            "<b>".into(),
        ])),
        doc(&[h1(&[t("foobaz")])]),
    );
}

#[test]
fn keeps_the_node_type_even_when_empty() {
    rpl(
        tag_doc(vec![
            tag_h1(vec!["<a>bar".into()]).into(),
            "<b>".into(),
        ]),
        Some(tag_doc(vec![
            tag_p(vec!["foo<a>baz".into()]).into(),
            "<b>".into(),
        ])),
        doc(&[h1(&[t("baz")])]),
    );
}

// ---------------------------------------------------------------------------
// 失败替换测试 (6 个)
// ---------------------------------------------------------------------------

#[test]
fn doesnt_allow_the_left_side_to_be_too_deep() {
    bad(
        tag_doc(vec![tag_p(vec!["<a><b>".into()]).into()]),
        Some(tag_doc(vec![
            tag_blockquote(vec![tag_p(vec!["<a>".into()]).into()]).into(),
            "<b>".into(),
        ])),
        "deeper",
    );
}

#[test]
fn doesnt_allow_a_depth_mismatch() {
    bad(
        tag_doc(vec![tag_p(vec!["<a><b>".into()]).into()]),
        Some(tag_doc(vec![
            "<a>".into(),
            tag_p(vec!["<b>".into()]).into(),
        ])),
        "inconsistent",
    );
}

#[test]
fn rejects_a_bad_fit() {
    bad(
        tag_doc(vec!["<a><b>".into()]),
        Some(tag_doc(vec![tag_p(vec!["<a>foo<b>".into()]).into()])),
        "invalid content",
    );
}

#[test]
fn rejects_unjoinable_content() {
    bad(
        tag_doc(vec![
            tag_ul(vec![
                TChild::N(li(&[p(&[t("a")])])),
                "<a>".into(),
            ]).into(),
            "<b>".into(),
        ]),
        Some(tag_doc(vec![
            tag_p(vec!["foo".into(), "<a>".into()]).into(),
            "<b>".into(),
        ])),
        "cannot join",
    );
}

#[test]
fn rejects_an_unjoinable_delete() {
    bad(
        tag_doc(vec![
            tag_blockquote(vec![
                TChild::N(p(&[t("a")])),
                "<a>".into(),
            ]).into(),
            tag_ul(vec![
                "<b>".into(),
                TChild::N(li(&[p(&[t("b")])])),
            ]).into(),
        ]),
        None,
        "cannot join",
    );
}

#[test]
fn check_content_validity() {
    bad(
        tag_doc(vec![
            tag_blockquote(vec![
                "<a>".into(),
                TChild::N(p(&[t("hi")])),
            ]).into(),
            "<b>".into(),
        ]),
        Some(tag_doc(vec![
            tag_blockquote(vec![
                TChild::N(t("hi")),
                "<a>".into(),
            ]).into(),
            "<b>".into(),
        ])),
        "invalid content",
    );
}
