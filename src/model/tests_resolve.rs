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

#[test]
fn resolve_out_of_range_errors() {
    let d = test_doc();
    // doc.content.size = 12,超过 12 应报错
    let err = ResolvedPos::resolve(&d, 13).err().unwrap();
    assert!(err.0.contains("Position 13 out of range"));
}

#[test]
fn resolve_pos_at_exact_size_is_valid() {
    let d = test_doc();
    // pos == doc.content.size 是合法的(在 doc 末尾)
    let rp = ResolvedPos::resolve(&d, 12).unwrap();
    assert_eq!(rp.pos, 12);
    assert_eq!(rp.depth, 0);
}

#[test]
fn resolve_delegates_to_resolve_no_cache() {
    let d = test_doc();
    // resolve 应返回与 resolve_no_cache 结构相同的 ResolvedPos
    let r_cached = ResolvedPos::resolve(&d, 7).unwrap();
    let r_nocache = ResolvedPos::resolve_no_cache(&d, 7).unwrap();
    assert_eq!(r_cached.pos, r_nocache.pos);
    assert_eq!(r_cached.depth, r_nocache.depth);
    assert_eq!(r_cached.parent_offset, r_nocache.parent_offset);
    assert_eq!(r_cached.path.len(), r_nocache.path.len());
    for i in 0..r_cached.path.len() {
        assert_eq!(r_cached.path[i].node.node_type.name, r_nocache.path[i].node.node_type.name);
        assert_eq!(r_cached.path[i].index, r_nocache.path[i].index);
        assert_eq!(r_cached.path[i].start_pos, r_nocache.path[i].start_pos);
    }
}

#[test]
fn resolve_in_empty_doc() {
    // doc 内部必须至少有一个 block(schema 要求 block+),用最小的 doc
    let d = doc(&[p(&[])]);
    // p() 空段落 size = 2,doc.size = 2
    // pos=0: 在 doc 层 offset 0,rem=0 break ⇒ depth=0
    let rp = ResolvedPos::resolve(&d, 0).unwrap();
    assert_eq!(rp.depth, 0);
    assert_eq!(rp.parent_offset, 0);
    // pos=1: 进入 p,parentOffset=0,p.content.size=0 ⇒ find_index(0)=(0,0), rem=0 break
    let rp = ResolvedPos::resolve(&d, 1).unwrap();
    assert_eq!(rp.depth, 1);
    assert_eq!(rp.parent_offset, 0);
    assert_eq!(rp.path[1].node.node_type.name, "paragraph");
    // pos=2: 在 doc 末尾
    let rp = ResolvedPos::resolve(&d, 2).unwrap();
    assert_eq!(rp.depth, 0);
    assert_eq!(rp.parent_offset, 2);
}

// --- Task 1: 简单 path 访问器测试 ---

#[test]
fn parent_returns_deepest_frame_node() {
    let d = test_doc();
    // pos=7 ⇒ path=[doc, blk, p2],parent 是 p2 (paragraph)
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    assert_eq!(rp.parent().node_type.name, "paragraph");
}

#[test]
fn doc_returns_root_node() {
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    assert_eq!(rp.doc().node_type.name, "doc");
    // pos=0 时 depth=0,doc() 和 parent() 相同
    let rp_zero = ResolvedPos::resolve(&d, 0).unwrap();
    assert_eq!(rp_zero.doc().node_type.name, "doc");
    assert_eq!(rp_zero.parent().node_type.name, "doc");
}

#[test]
fn node_returns_frame_at_requested_depth() {
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    // pos=7 path=[doc, blk, p2]
    assert_eq!(rp.node(Some(0)).node_type.name, "doc");
    assert_eq!(rp.node(Some(1)).node_type.name, "blockquote");
    assert_eq!(rp.node(Some(2)).node_type.name, "paragraph");
    // None ⇒ self.depth (2)
    assert_eq!(rp.node(None).node_type.name, "paragraph");
    // 负数:-1 ⇒ depth-1 = 1 (blockquote)
    assert_eq!(rp.node(Some(-1)).node_type.name, "blockquote");
}

#[test]
fn index_returns_frame_index_at_depth() {
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    // pos=7 的 path 索引:(doc,1,4),(blk,0,5),(p2,0,6)
    assert_eq!(rp.index(Some(0)), 1); // blockquote 是 doc 的第 1 个子
    assert_eq!(rp.index(Some(1)), 0); // p2 是 blk 的第 0 个子
    assert_eq!(rp.index(Some(2)), 0); // text 是 p2 的第 0 个子
    assert_eq!(rp.index(None), 0);    // 默认 = self.depth = 2
}

#[test]
fn index_after_excludes_current_index_when_at_boundary() {
    let d = test_doc();
    // pos=4(p1 和 blk 之间,depth=0,text_offset=0)
    // index(0)=1, depth==self.depth && text_offset==0 ⇒ +0 ⇒ 1
    let rp = ResolvedPos::resolve(&d, 4).unwrap();
    assert_eq!(rp.index_after(None), 1);
    assert_eq!(rp.index_after(Some(0)), 1);
}

#[test]
fn index_after_increments_when_inside_text() {
    let d = test_doc();
    // pos=7(在 text "cdef" 内,depth=2,text_offset=1>0)
    // index(2)=0, depth==self.depth && text_offset>0 ⇒ +1 ⇒ 1
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    assert_eq!(rp.index_after(None), 1);
    assert_eq!(rp.index_after(Some(2)), 1);
}

#[test]
fn index_after_increments_when_depth_not_self_depth() {
    let d = test_doc();
    // pos=7(depth=2),index_after(0) 访问 depth=0 != self.depth ⇒ +1
    // index(0)=1, +1 = 2
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    assert_eq!(rp.index_after(Some(0)), 2);
    assert_eq!(rp.index_after(Some(1)), 1); // index(1)=0, +1=1
}

// --- Task 2: 位置访问器测试 ---

#[test]
fn start_returns_zero_at_depth_zero() {
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    assert_eq!(rp.start(Some(0)), 0);
}

#[test]
fn start_returns_inner_content_start_at_deeper_depths() {
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    // path=[(doc,1,4),(blk,0,5),(p2,0,6)]
    // start(1) = path[0].start_pos + 1 = 4+1 = 5(blockquote content 开始)
    // start(2) = path[1].start_pos + 1 = 5+1 = 6(p2 content 开始)
    assert_eq!(rp.start(Some(1)), 5);
    assert_eq!(rp.start(Some(2)), 6);
    // None 默认 = self.depth = 2
    assert_eq!(rp.start(None), 6);
}

#[test]
fn end_is_start_plus_content_size() {
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    // end(0) = 0 + doc.content.size = 12
    // end(1) = 5 + blk.content.size = 5+6 = 11(p2.node_size=6)
    // end(2) = 6 + p2.content.size = 6+4 = 10(text.node_size=4)
    assert_eq!(rp.end(Some(0)), 12);
    assert_eq!(rp.end(Some(1)), 11);
    assert_eq!(rp.end(Some(2)), 10);
}

#[test]
fn text_offset_is_pos_minus_deepest_start_pos() {
    let d = test_doc();
    // pos=7,deepest frame start_pos=6 ⇒ text_offset = 1
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    assert_eq!(rp.text_offset(), 1);
    // pos=0,deepest frame(doc,0,0).start_pos=0 ⇒ text_offset=0
    let rp0 = ResolvedPos::resolve(&d, 0).unwrap();
    assert_eq!(rp0.text_offset(), 0);
    // pos=4,deepest(doc,1,4).start_pos=4 ⇒ text_offset=0
    let rp4 = ResolvedPos::resolve(&d, 4).unwrap();
    assert_eq!(rp4.text_offset(), 0);
}

#[test]
fn before_errors_at_depth_zero() {
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    let err = rp.before(Some(0)).err().unwrap();
    assert!(err.0.contains("no position before the top-level node"));
}

#[test]
fn before_returns_pos_when_depth_equals_self_depth_plus_one() {
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap(); // self.depth=2
    // before(3) = before(self.depth+1) ⇒ self.pos = 7
    assert_eq!(rp.before(Some(3)).unwrap(), 7);
}

#[test]
fn before_returns_frame_start_pos_otherwise() {
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    // before(1) = path[0].start_pos = 4(blockquote 的 open tag 位置)
    // before(2) = path[1].start_pos = 5(p2 的 open tag 位置)
    assert_eq!(rp.before(Some(1)).unwrap(), 4);
    assert_eq!(rp.before(Some(2)).unwrap(), 5);
}

#[test]
fn after_errors_at_depth_zero() {
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    let err = rp.after(Some(0)).err().unwrap();
    assert!(err.0.contains("no position after the top-level node"));
}

#[test]
fn after_returns_pos_when_depth_equals_self_depth_plus_one() {
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    assert_eq!(rp.after(Some(3)).unwrap(), 7);
}

#[test]
fn after_returns_frame_start_plus_node_size_otherwise() {
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    // after(1) = path[0].start_pos + path[1].node.node_size() = 4 + 8 = 12(blockquote 关闭后的位置)
    // after(2) = path[1].start_pos + path[2].node.node_size() = 5 + 6 = 11(p2 关闭后的位置)
    assert_eq!(rp.after(Some(1)).unwrap(), 12);
    assert_eq!(rp.after(Some(2)).unwrap(), 11);
}

// --- Task 3: pos_at_index + node_after + node_before 测试 ---

#[test]
fn pos_at_index_at_depth_zero() {
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    // depth=0,node=doc,base pos=0
    // pos_at_index(0, 0) = 0
    // pos_at_index(1, 0) = 0 + p1.node_size(4) = 4
    // pos_at_index(2, 0) = 4 + blk.node_size(8) = 12
    assert_eq!(rp.pos_at_index(0, Some(0)), 0);
    assert_eq!(rp.pos_at_index(1, Some(0)), 4);
    assert_eq!(rp.pos_at_index(2, Some(0)), 12);
}

#[test]
fn pos_at_index_at_deeper_depth() {
    let d = test_doc();
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    // depth=2,node=p2,base pos = path[1].start_pos + 1 = 5+1 = 6
    // pos_at_index(0, 2) = 6
    // pos_at_index(1, 2) = 6 + text("cdef").node_size(4) = 10
    assert_eq!(rp.pos_at_index(0, Some(2)), 6);
    assert_eq!(rp.pos_at_index(1, Some(2)), 10);
}

#[test]
fn node_after_returns_full_child_when_at_boundary() {
    let d = test_doc();
    // pos=4(p1 和 blk 之间,text_offset=0,index=1)
    // node_after = parent(doc).child(1) = blockquote 完整节点
    let rp = ResolvedPos::resolve(&d, 4).unwrap();
    let after = rp.node_after().unwrap();
    assert_eq!(after.node_type.name, "blockquote");
}

#[test]
fn node_after_returns_none_at_last_index() {
    let d = test_doc();
    // pos=12(doc 末尾,index=2==doc.child_count=2)
    let rp = ResolvedPos::resolve(&d, 12).unwrap();
    assert!(rp.node_after().is_none());
}

#[test]
fn node_after_cuts_text_when_inside_text_node() {
    let d = test_doc();
    // pos=7(text "cdef" 内部,text_offset=1)
    // parent=p2, index=0, child=text("cdef"), d_off=1 ⇒ cut(1) = text("def")
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    let after = rp.node_after().unwrap();
    assert!(after.is_text());
    assert_eq!(after.text.as_deref(), Some("def"));
}

#[test]
fn node_before_returns_none_at_start() {
    let d = test_doc();
    // pos=0(doc 开头,index=0,text_offset=0)
    let rp = ResolvedPos::resolve(&d, 0).unwrap();
    assert!(rp.node_before().is_none());
}

#[test]
fn node_before_returns_previous_child_when_at_boundary() {
    let d = test_doc();
    // pos=4(text_offset=0,index=1)⇒ parent.child(0) = p1 整个节点
    let rp = ResolvedPos::resolve(&d, 4).unwrap();
    let before = rp.node_before().unwrap();
    assert_eq!(before.node_type.name, "paragraph");
    assert_eq!(before.text_content(), "ab");
}

#[test]
fn node_before_cuts_text_when_inside_text_node() {
    let d = test_doc();
    // pos=7(text "cdef" 内部,text_offset=1)
    // parent=p2, index=0, d_off=1 > 0 ⇒ parent.child(0).cut(0, 1) = text("c")
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    let before = rp.node_before().unwrap();
    assert!(before.is_text());
    assert_eq!(before.text.as_deref(), Some("c"));
}

// --- Task 4: integration test ---
//
// 对照 TS test-resolve.ts 的 "should reflect the document structure" 测试。
// 循环 pos 0..=12 验证所有访问器的一致性。

#[test]
fn resolve_integration_all_accessors_consistent() {
    let d = test_doc();
    // 定义每个 pos 的期望 depth
    let expected_depth: [usize; 13] = [0, 1, 1, 1, 0, 1, 2, 2, 2, 2, 2, 1, 0];
    let expected_parent_offset: [usize; 13] = [0, 0, 1, 2, 4, 0, 0, 1, 2, 3, 4, 6, 12];

    for pos in 0..=12 {
        let rp = ResolvedPos::resolve(&d, pos).unwrap();
        assert_eq!(rp.depth, expected_depth[pos], "pos={} depth", pos);
        assert_eq!(rp.parent_offset, expected_parent_offset[pos], "pos={} parent_offset", pos);

        // doc() 始终返回 doc 节点
        assert_eq!(rp.doc().node_type.name, "doc", "pos={} doc", pos);

        // start(0) 始终为 0, end(0) 始终为 doc.content.size = 12
        assert_eq!(rp.start(Some(0)), 0, "pos={} start(0)", pos);
        assert_eq!(rp.end(Some(0)), 12, "pos={} end(0)", pos);

        // before(0)/after(0) 始终报错
        assert!(rp.before(Some(0)).is_err(), "pos={} before(0)", pos);
        assert!(rp.after(Some(0)).is_err(), "pos={} after(0)", pos);

        // 对每个层级验证 node/start/end/before/after 的一致性
        for i in 0..=rp.depth {
            let node = rp.node(Some(i as i32));
            let start = rp.start(Some(i as i32));
            let end = rp.end(Some(i as i32));
            // end == start + content.size(self-consistent invariant)
            assert_eq!(end, start + node.content.size, "pos={} level={} end-start", pos, i);

            if i > 0 {
                // before(i) 指向 node 的开始 tag 之前,即 start-1
                assert_eq!(rp.before(Some(i as i32)).unwrap(), start - 1, "pos={} level={} before", pos, i);
                // after(i) 指向 node 的结束 tag 之后,即 end+1
                assert_eq!(rp.after(Some(i as i32)).unwrap(), end + 1, "pos={} level={} after", pos, i);
            }
        }

        // parent_offset 不变式:parent_offset == pos - start(self.depth)
        // (parent_offset 即 pos 相对于最深 node content 起始的偏移)
        let depth_start = rp.start(Some(rp.depth as i32));
        assert_eq!(rp.parent_offset, pos - depth_start, "pos={} parent_offset invariant", pos);

        // text_offset 不变式:text_offset == pos - path[self.depth].start_pos
        // 注意:text_offset 不一定等于 parent_offset —— 当 path[depth].index > 0
        // (pos 落在某个 child 之后的边界)时,text_offset < parent_offset。
        // 只在 path[depth].index==0 或 pos 精确落在 text 内部时两者相等。
        if rp.index(None) == 0 {
            assert_eq!(rp.text_offset(), rp.parent_offset, "pos={} text_offset at index 0", pos);
        }
    }
}

#[test]
fn resolve_integration_node_before_after_at_text_boundaries() {
    let d = test_doc();

    // pos=7:text "cdef" 内部字符 1(c 之后, d 之前)
    // node_before = text("c"), node_after = text("def")
    let rp = ResolvedPos::resolve(&d, 7).unwrap();
    assert_eq!(rp.node_before().unwrap().text.as_deref(), Some("c"));
    assert_eq!(rp.node_after().unwrap().text.as_deref(), Some("def"));

    // pos=8:text "cdef" 内部字符 2
    let rp = ResolvedPos::resolve(&d, 8).unwrap();
    assert_eq!(rp.node_before().unwrap().text.as_deref(), Some("cd"));
    assert_eq!(rp.node_after().unwrap().text.as_deref(), Some("ef"));

    // pos=6:p2 的开头,text_offset=0
    // node_before = None(index=0), node_after = text("cdef") 整个
    let rp = ResolvedPos::resolve(&d, 6).unwrap();
    assert!(rp.node_before().is_none());
    assert_eq!(rp.node_after().unwrap().text.as_deref(), Some("cdef"));

    // pos=10:p2 的末尾,text 之后
    // node_before = text("cdef") 整个,node_after = None(index=1==child_count)
    let rp = ResolvedPos::resolve(&d, 10).unwrap();
    assert_eq!(rp.node_before().unwrap().text.as_deref(), Some("cdef"));
    assert!(rp.node_after().is_none());
}

// --- Task 1: 简单位置 / 比较方法测试 ---

#[test]
fn same_parent_returns_true_for_same_parent_content() {
    let d = test_doc();
    // pos=6 和 pos=7 都在 p2 content 内,parent 相同
    let rp6 = ResolvedPos::resolve(&d, 6).unwrap();
    let rp7 = ResolvedPos::resolve(&d, 7).unwrap();
    assert!(rp6.same_parent(&rp7));
}

#[test]
fn same_parent_returns_false_for_different_parents() {
    let d = test_doc();
    // pos=1(在 p1 内)和 pos=7(在 p2 内)parent 不同
    let rp1 = ResolvedPos::resolve(&d, 1).unwrap();
    let rp7 = ResolvedPos::resolve(&d, 7).unwrap();
    assert!(!rp1.same_parent(&rp7));
}

#[test]
fn max_returns_the_greater_pos() {
    let d = test_doc();
    let rp3 = ResolvedPos::resolve(&d, 3).unwrap();
    let rp7 = ResolvedPos::resolve(&d, 7).unwrap();
    assert_eq!(rp3.max(&rp7).pos, 7);
    assert_eq!(rp7.max(&rp3).pos, 7);
    // 相等时返回 self
    let rp3b = ResolvedPos::resolve(&d, 3).unwrap();
    assert_eq!(rp3.max(&rp3b).pos, 3);
}

#[test]
fn min_returns_the_smaller_pos() {
    let d = test_doc();
    let rp3 = ResolvedPos::resolve(&d, 3).unwrap();
    let rp7 = ResolvedPos::resolve(&d, 7).unwrap();
    assert_eq!(rp3.min(&rp7).pos, 3);
    assert_eq!(rp7.min(&rp3).pos, 3);
}

#[test]
fn shared_depth_returns_deepest_shared_ancestor() {
    let d = test_doc();
    // pos=7 的 depth=2, start(1)=5, end(1)=11, start(2)=6, end(2)=10
    // shared_depth(7): start(2)=6 <= 7 && end(2)=10 >= 7 ⇒ 返回 2
    let rp7 = ResolvedPos::resolve(&d, 7).unwrap();
    assert_eq!(rp7.shared_depth(7), 2);
    // shared_depth(5): start(2)=6 <= 5? NO, 继续 depth=1: start(1)=5 <= 5 && end(1)=11 >= 5 ⇒ 1
    assert_eq!(rp7.shared_depth(5), 1);
    // shared_depth(3): start(2)=6 <= 3? NO. start(1)=5 <= 3? NO. ⇒ 返回 0
    assert_eq!(rp7.shared_depth(3), 0);
    // shared_depth(12): start(2)=6 <= 12 && end(2)=10 >= 12? NO. start(1)=5 <= 12 && end(1)=11 >= 12? NO. ⇒ 0
    assert_eq!(rp7.shared_depth(12), 0);
}

#[test]
fn shared_depth_at_top_level_returns_zero() {
    let d = test_doc();
    // pos=4 depth=0,循环 depth>0 不执行,返回 0
    let rp4 = ResolvedPos::resolve(&d, 4).unwrap();
    assert_eq!(rp4.shared_depth(7), 0);
}
