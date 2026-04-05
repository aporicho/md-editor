// src/model/tests_resolve.rs
//
// Phase 4.2 — ResolvedPos resolve 算法基础测试。
// Phase 4.3/4.4 将在本文件补充访问器(node/start/end/before/after/marks)测试。

use crate::model::resolved_pos::{PathFrame, ResolvedPos};
use crate::model::test_builder::{blockquote, doc, p, t};

#[test]
fn path_frame_can_be_constructed() {
    let d = doc(&[p(&[t("hi")])]);
    let frame = PathFrame {
        node: d.clone(),
        index: 0,
        start_pos: 0,
    };
    assert_eq!(frame.index, 0);
    assert_eq!(frame.start_pos, 0);
    assert_eq!(frame.node.node_type.name, "doc");
}

#[test]
fn resolved_pos_can_be_constructed_via_new() {
    let d = doc(&[p(&[t("x")])]);
    let path = vec![PathFrame {
        node: d.clone(),
        index: 0,
        start_pos: 0,
    }];
    let rp = ResolvedPos::new(0, path, 0);
    assert_eq!(rp.pos, 0);
    assert_eq!(rp.parent_offset, 0);
    assert_eq!(rp.depth, 0);
    assert_eq!(rp.path.len(), 1);
}

#[test]
fn resolved_pos_depth_is_path_len_minus_one() {
    let d = doc(&[p(&[t("x")])]);
    // 手工构造深度为 2 的 path (doc + blockquote + p)
    let frames = vec![
        PathFrame { node: d.clone(), index: 0, start_pos: 0 },
        PathFrame { node: d.clone(), index: 0, start_pos: 1 },
        PathFrame { node: d.clone(), index: 0, start_pos: 2 },
    ];
    let rp = ResolvedPos::new(3, frames, 0);
    assert_eq!(rp.depth, 2);
    assert_eq!(rp.path.len(), 3);
}

#[test]
fn resolve_depth_defaults_to_self_depth() {
    let d = doc(&[p(&[t("x")])]);
    let frames = vec![
        PathFrame { node: d.clone(), index: 0, start_pos: 0 },
        PathFrame { node: d.clone(), index: 0, start_pos: 1 },
    ];
    let rp = ResolvedPos::new(1, frames, 0);
    assert_eq!(rp.resolve_depth(None), 1);
}

#[test]
fn resolve_depth_accepts_positive_value() {
    let d = doc(&[p(&[t("x")])]);
    let frames = vec![
        PathFrame { node: d.clone(), index: 0, start_pos: 0 },
        PathFrame { node: d.clone(), index: 0, start_pos: 1 },
    ];
    let rp = ResolvedPos::new(1, frames, 0);
    assert_eq!(rp.resolve_depth(Some(0)), 0);
    assert_eq!(rp.resolve_depth(Some(1)), 1);
}

#[test]
fn resolve_depth_negative_wraps_from_self_depth() {
    let d = doc(&[p(&[t("x")])]);
    // depth=2 path
    let frames = vec![
        PathFrame { node: d.clone(), index: 0, start_pos: 0 },
        PathFrame { node: d.clone(), index: 0, start_pos: 1 },
        PathFrame { node: d.clone(), index: 0, start_pos: 2 },
    ];
    let rp = ResolvedPos::new(3, frames, 0);
    // self.depth = 2; -1 ⇒ 2 + (-1) = 1; -2 ⇒ 0
    assert_eq!(rp.resolve_depth(Some(-1)), 1);
    assert_eq!(rp.resolve_depth(Some(-2)), 0);
}

// --- resolve_no_cache 算法测试 ---
// 测试 doc: doc(p("ab"), blockquote(p("cdef")))
//   p("ab").size = 2+2 = 4
//   blockquote(p("cdef")).size = 2+(2+4) = 8
//   doc.size = 4+8 = 12

fn test_doc() -> crate::model::node::Node {
    doc(&[p(&[t("ab")]), blockquote(&[p(&[t("cdef")])])])
}

#[test]
fn resolve_no_cache_pos_zero_stays_in_doc() {
    let d = test_doc();
    let rp = ResolvedPos::resolve_no_cache(&d, 0).unwrap();
    // pos=0 ⇒ 处于 doc 层的 offset 0,rem=0 立即 break
    assert_eq!(rp.depth, 0);
    assert_eq!(rp.parent_offset, 0);
    assert_eq!(rp.path.len(), 1);
    assert_eq!(rp.path[0].node.node_type.name, "doc");
    assert_eq!(rp.path[0].index, 0);
    assert_eq!(rp.path[0].start_pos, 0);
}

#[test]
fn resolve_no_cache_pos_one_inside_first_paragraph() {
    let d = test_doc();
    let rp = ResolvedPos::resolve_no_cache(&d, 1).unwrap();
    // pos=1 进入 p1 内部,文本 "ab" offset=0 ⇒ rem=1 hit text break
    assert_eq!(rp.depth, 1);
    assert_eq!(rp.parent_offset, 0); // "ab" 的字符 0
    assert_eq!(rp.path[0].node.node_type.name, "doc");
    assert_eq!(rp.path[0].index, 0);
    assert_eq!(rp.path[0].start_pos, 0);
    assert_eq!(rp.path[1].node.node_type.name, "paragraph");
    assert_eq!(rp.path[1].index, 0);
    assert_eq!(rp.path[1].start_pos, 1);
}

#[test]
fn resolve_no_cache_pos_three_at_p1_end_boundary() {
    let d = test_doc();
    let rp = ResolvedPos::resolve_no_cache(&d, 3).unwrap();
    // pos=3 在 p1 末尾(text "ab" 之后,p1 close 之前)
    // doc findIndex(3): index=0, offset=0, rem=3, 下降进 p1
    // p1 findIndex(2): 2==text("ab").size=2 ⇒ (index=1, offset=2), rem=0 break
    assert_eq!(rp.depth, 1);
    assert_eq!(rp.parent_offset, 2);
    assert_eq!(rp.path[0].node.node_type.name, "doc");
    assert_eq!(rp.path[0].index, 0);
    assert_eq!(rp.path[1].node.node_type.name, "paragraph");
    assert_eq!(rp.path[1].index, 1); // 在 text "ab" 之后
    // start_pos = start(1) + offset(2) = 3(p1 末尾的绝对位置)
    assert_eq!(rp.path[1].start_pos, 3);
}

#[test]
fn resolve_no_cache_pos_four_between_blocks() {
    let d = test_doc();
    let rp = ResolvedPos::resolve_no_cache(&d, 4).unwrap();
    // pos=4 在 p1 和 blockquote 之间
    // doc findIndex(4): p1.size=4, 4==4 ⇒ (index=1, offset=4), rem=0 break
    assert_eq!(rp.depth, 0);
    assert_eq!(rp.parent_offset, 4);
    assert_eq!(rp.path[0].node.node_type.name, "doc");
    assert_eq!(rp.path[0].index, 1);
    assert_eq!(rp.path[0].start_pos, 4);
}

#[test]
fn resolve_no_cache_pos_seven_deep_in_nested_paragraph() {
    let d = test_doc();
    let rp = ResolvedPos::resolve_no_cache(&d, 7).unwrap();
    // pos=7 在 blockquote > p2 > text "cdef" 的字符 1(即 "c" 后 "d" 前)
    // doc.findIndex(7): blockquote@offset 4..12 ⇒ (1, 4), rem=3
    // 下降到 blk, parentOffset=2, start=5
    // blk.findIndex(2): p2@offset 0..6, 2<6 ⇒ (0, 0), rem=2
    // 下降到 p2, parentOffset=1, start=6
    // p2.findIndex(1): text("cdef")@size 4, 1<4 ⇒ (0, 0), rem=1
    // 下降到 text("cdef"), isText ⇒ break
    assert_eq!(rp.depth, 2);
    assert_eq!(rp.parent_offset, 1);
    assert_eq!(rp.path[0].node.node_type.name, "doc");
    assert_eq!(rp.path[0].index, 1);
    assert_eq!(rp.path[0].start_pos, 4);
    assert_eq!(rp.path[1].node.node_type.name, "blockquote");
    assert_eq!(rp.path[1].index, 0);
    assert_eq!(rp.path[1].start_pos, 5);
    assert_eq!(rp.path[2].node.node_type.name, "paragraph");
    assert_eq!(rp.path[2].index, 0);
    assert_eq!(rp.path[2].start_pos, 6);
}

#[test]
fn resolve_no_cache_pos_eleven_at_p2_end() {
    let d = test_doc();
    let rp = ResolvedPos::resolve_no_cache(&d, 11).unwrap();
    // pos=11 在 p2 末尾(text 之后,p2 close 之前)
    // doc.findIndex(11): blockquote@offset 4..12, 11<12 ⇒ (1, 4), rem=7
    // 下降到 blk, parentOffset=6, start=5
    // blk.findIndex(6): p2.size=6, 6==6 ⇒ (1, 6), rem=0 break
    assert_eq!(rp.depth, 1);
    assert_eq!(rp.parent_offset, 6);
    assert_eq!(rp.path[0].node.node_type.name, "doc");
    assert_eq!(rp.path[0].index, 1);
    assert_eq!(rp.path[0].start_pos, 4);
    assert_eq!(rp.path[1].node.node_type.name, "blockquote");
    assert_eq!(rp.path[1].index, 1); // 在 p2 之后
    // start_pos = start(5) + offset(6) = 11(blk 内容末尾的绝对位置)
    assert_eq!(rp.path[1].start_pos, 11);
}

#[test]
fn resolve_no_cache_pos_twelve_at_doc_end() {
    let d = test_doc();
    let rp = ResolvedPos::resolve_no_cache(&d, 12).unwrap();
    // pos=12 在 doc 末尾
    // doc.findIndex(12): doc.size=12, 12==12 ⇒ (2, 12), rem=0 break
    assert_eq!(rp.depth, 0);
    assert_eq!(rp.parent_offset, 12);
    assert_eq!(rp.path[0].node.node_type.name, "doc");
    assert_eq!(rp.path[0].index, 2); // 在 blockquote 之后
    assert_eq!(rp.path[0].start_pos, 12);
}
