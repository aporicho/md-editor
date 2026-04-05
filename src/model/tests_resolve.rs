// src/model/tests_resolve.rs
//
// Phase 4.2 — ResolvedPos resolve 算法基础测试。
// Phase 4.3/4.4 将在本文件补充访问器(node/start/end/before/after/marks)测试。

use crate::model::resolved_pos::{PathFrame, ResolvedPos};
use crate::model::test_builder::{doc, p, t};

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
