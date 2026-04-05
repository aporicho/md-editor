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
