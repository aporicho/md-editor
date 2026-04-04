//! TS 测试一对一移植
//! 原始文件：reference/prosemirror-model/test/test-content.ts
//!             reference/prosemirror-model/test/test-diff.ts
//!             reference/prosemirror-model/test/test-mark.ts
//!             reference/prosemirror-model/test/test-resolve.ts

#![cfg(test)]

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use super::content::ContentMatch;
use super::fragment::Fragment;
use super::mark::Mark;
use super::node::Node;
use super::schema::{MarkType, NodeType};
use super::Attrs;

// ─────────────────────────────────────────────
//  Test schema (matches prosemirror-test-builder)
// ─────────────────────────────────────────────

fn make_nt(name: &str, is_block: bool, groups: &[&str]) -> Arc<NodeType> {
    Arc::new(NodeType {
        name: name.into(),
        groups: groups.iter().map(|s| s.to_string()).collect(),
        is_block,
        is_text: name == "text",
        inline_content: false,
        mark_set: None,
        content_match: None,
    })
}

fn make_mt(name: &str, rank: usize) -> Arc<MarkType> {
    Arc::new(MarkType { name: name.into(), rank, excluded: vec![], inclusive: None })
}

/// prosemirror-test-builder と同等のスキーマ
fn test_schema() -> HashMap<String, Arc<NodeType>> {
    let mut m = HashMap::new();
    for (name, is_block, groups) in &[
        ("text",             false, vec!["inline"]),
        ("image",            false, vec!["inline"]),
        ("hard_break",       false, vec!["inline"]),
        ("paragraph",        true,  vec!["block"]),
        ("heading",          true,  vec!["block"]),
        ("horizontal_rule",  true,  vec!["block"]),
        ("code_block",       true,  vec!["block"]),
        ("bullet_list",      true,  vec!["block"]),
        ("list_item",        true,  vec![]),
        ("blockquote",       true,  vec!["block"]),
        ("doc",              true,  vec![]),
    ] {
        m.insert(name.to_string(), make_nt(name, *is_block, groups));
    }
    m
}

/// 空属性
fn empty_attrs() -> Attrs { BTreeMap::new() }

/// テキストノード作成
fn text_node(s: &Node, text: &str) -> Node {
    Node { node_type: Arc::clone(&s.node_type), attrs: empty_attrs(),
           content: Fragment::empty(), marks: vec![], text: Some(text.into()) }
}

/// 任意のノードタイプで子ノードを持つノード作成
fn node(nt: &Arc<NodeType>, children: Vec<Node>) -> Node {
    Node { node_type: Arc::clone(nt), attrs: empty_attrs(),
           content: Fragment::from_array(children), marks: vec![], text: None }
}

/// テキストノード（マーク付き）
fn marked_text(schema: &HashMap<String, Arc<NodeType>>, s: &str, marks: Vec<Mark>) -> Node {
    Node { node_type: Arc::clone(schema.get("text").unwrap()),
           attrs: empty_attrs(), content: Fragment::empty(),
           marks, text: Some(s.into()) }
}

fn p(schema: &HashMap<String, Arc<NodeType>>, children: Vec<Node>) -> Node {
    node(schema.get("paragraph").unwrap(), children)
}
fn hr(schema: &HashMap<String, Arc<NodeType>>) -> Node {
    node(schema.get("horizontal_rule").unwrap(), vec![])
}
fn br(schema: &HashMap<String, Arc<NodeType>>) -> Node {
    node(schema.get("hard_break").unwrap(), vec![])
}
fn h1(schema: &HashMap<String, Arc<NodeType>>, children: Vec<Node>) -> Node {
    node(schema.get("heading").unwrap(), children)
}
fn pre(schema: &HashMap<String, Arc<NodeType>>, children: Vec<Node>) -> Node {
    node(schema.get("code_block").unwrap(), children)
}
fn bq(schema: &HashMap<String, Arc<NodeType>>, children: Vec<Node>) -> Node {
    node(schema.get("blockquote").unwrap(), children)
}
fn doc(schema: &HashMap<String, Arc<NodeType>>, children: Vec<Node>) -> Node {
    node(schema.get("doc").unwrap(), children)
}
fn img(schema: &HashMap<String, Arc<NodeType>>) -> Node {
    node(schema.get("image").unwrap(), vec![])
}
fn txt(schema: &HashMap<String, Arc<NodeType>>, s: &str) -> Node {
    Node { node_type: Arc::clone(schema.get("text").unwrap()),
           attrs: empty_attrs(), content: Fragment::empty(),
           marks: vec![], text: Some(s.into()) }
}

// ─────────────────────────────────────────────
//  Content match helpers
// ─────────────────────────────────────────────

/// TS: match(expr, types) → match types sequentially, return validEnd
fn match_types(expr: &str, types_str: &str, schema: &HashMap<String, Arc<NodeType>>) -> bool {
    let cm = match ContentMatch::parse(expr, schema) {
        Ok(cm) => cm,
        Err(_) => return false,
    };
    let ts: Vec<Arc<NodeType>> = if types_str.is_empty() {
        vec![]
    } else {
        types_str.split(' ')
            .filter_map(|t| schema.get(t).map(Arc::clone))
            .collect()
    };
    let mut state: Option<Arc<ContentMatch>> = Some(cm);
    for t in &ts {
        state = state.and_then(|s| s.match_type(t));
    }
    state.map(|s| s.valid_end).unwrap_or(false)
}

fn valid(expr: &str, types: &str, schema: &HashMap<String, Arc<NodeType>>) {
    assert!(match_types(expr, types, schema),
        "expected valid: expr={:?} types={:?}", expr, types);
}
fn invalid(expr: &str, types: &str, schema: &HashMap<String, Arc<NodeType>>) {
    assert!(!match_types(expr, types, schema),
        "expected invalid: expr={:?} types={:?}", expr, types);
}

/// Fragment 構造比較（ノード名と子ノード構造）
fn assert_frag_eq(a: &Fragment, b: &Fragment, ctx: &str) {
    assert_eq!(a.child_count(), b.child_count(),
        "{}: child count mismatch (got {}, expected {})", ctx, a.child_count(), b.child_count());
    for i in 0..a.child_count() {
        let na = a.child(i);
        let nb = b.child(i);
        assert_eq!(na.node_type.name, nb.node_type.name,
            "{}: node[{}] type mismatch", ctx, i);
        assert_frag_eq(&na.content, &nb.content, ctx);
    }
}

/// TS: fill(expr, before, after, result) — fillBefore test
fn fill_test(
    expr: &str,
    before_frag: Fragment,
    after_frag: Fragment,
    result: Option<Fragment>,
    schema: &HashMap<String, Arc<NodeType>>,
) {
    let cm = ContentMatch::parse(expr, schema).expect("parse");
    let state = cm.match_fragment(&before_frag, 0, before_frag.child_count());
    let filled = state.and_then(|s| s.fill_before(&after_frag, true, 0));
    match result {
        Some(expected) => {
            let filled = filled.expect("expected fill_before to succeed");
            assert_frag_eq(&filled, &expected, expr);
        }
        None => {
            assert!(filled.is_none(), "expected fill_before to fail for {:?}", expr);
        }
    }
}

/// TS: fill3(expr, before, mid, after, left, right?)
fn fill3_test(
    expr: &str,
    before_frag: Fragment,
    mid_frag: Fragment,
    after_frag: Fragment,
    left: Option<Fragment>,
    right: Option<Fragment>,
    schema: &HashMap<String, Arc<NodeType>>,
) {
    let cm = ContentMatch::parse(expr, schema).expect("parse");
    let a = cm.match_fragment(&before_frag, 0, before_frag.child_count())
        .and_then(|s| s.fill_before(&mid_frag, false, 0));
    let b = a.as_ref().and_then(|a_fill| {
        let combined = before_frag.append(a_fill).append(&mid_frag);
        cm.match_fragment(&combined, 0, combined.child_count())
            .and_then(|s| s.fill_before(&after_frag, true, 0))
    });
    match left {
        Some(l) => {
            let a = a.expect("expected a to succeed");
            let b = b.expect("expected b to succeed");
            assert_frag_eq(&a, &l, &format!("{} left", expr));
            assert_frag_eq(&b, &right.unwrap(), &format!("{} right", expr));
        }
        None => {
            assert!(b.is_none(), "expected fill3 to fail for {:?}", expr);
        }
    }
}

// ═════════════════════════════════════════════
//  test-content.ts — ContentMatch.matchType
// ═════════════════════════════════════════════

#[test]
fn content_accepts_empty_for_empty_expr() {
    // TS: "accepts empty content for the empty expr"
    let s = test_schema();
    valid("", "", &s);
}

#[test]
fn content_rejects_content_in_empty_expr() {
    // TS: "doesn't accept content in the empty expr"
    let s = test_schema();
    invalid("", "image", &s);
}

#[test]
fn content_star_matches_nothing() {
    // TS: "matches nothing to an asterisk"
    let s = test_schema();
    valid("image*", "", &s);
}

#[test]
fn content_star_matches_one() {
    // TS: "matches one element to an asterisk"
    let s = test_schema();
    valid("image*", "image", &s);
}

#[test]
fn content_star_matches_multiple() {
    // TS: "matches multiple elements to an asterisk"
    let s = test_schema();
    valid("image*", "image image image image", &s);
}

#[test]
fn content_star_rejects_wrong_type() {
    // TS: "only matches appropriate elements to an asterisk"
    let s = test_schema();
    invalid("image*", "image text", &s);
}

#[test]
fn content_group_matches_members() {
    // TS: "matches group members to a group"
    let s = test_schema();
    valid("inline*", "image text", &s);
}

#[test]
fn content_group_rejects_non_members() {
    // TS: "doesn't match non-members to a group"
    let s = test_schema();
    invalid("inline*", "paragraph", &s);
}

#[test]
fn content_choice_matches_one() {
    // TS: "matches an element to a choice expression"
    let s = test_schema();
    valid("(paragraph | heading)", "paragraph", &s);
}

#[test]
fn content_choice_rejects_unmentioned() {
    // TS: "doesn't match unmentioned elements to a choice expr"
    let s = test_schema();
    invalid("(paragraph | heading)", "image", &s);
}

#[test]
fn content_sequence_matches() {
    // TS: "matches a simple sequence"
    let s = test_schema();
    valid("paragraph horizontal_rule paragraph",
          "paragraph horizontal_rule paragraph", &s);
}

#[test]
fn content_sequence_too_long() {
    // TS: "fails when a sequence is too long"
    let s = test_schema();
    invalid("paragraph horizontal_rule",
            "paragraph horizontal_rule paragraph", &s);
}

#[test]
fn content_sequence_too_short() {
    // TS: "fails when a sequence is too short"
    let s = test_schema();
    invalid("paragraph horizontal_rule paragraph",
            "paragraph horizontal_rule", &s);
}

#[test]
fn content_sequence_starts_wrong() {
    // TS: "fails when a sequence starts incorrectly"
    let s = test_schema();
    invalid("paragraph horizontal_rule",
            "horizontal_rule paragraph horizontal_rule", &s);
}

#[test]
fn content_seq_star_zero() {
    // TS: "accepts a sequence asterisk matching zero elements"
    let s = test_schema();
    valid("heading paragraph*", "heading", &s);
}

#[test]
fn content_seq_star_multiple() {
    // TS: "accepts a sequence asterisk matching multiple elts"
    let s = test_schema();
    valid("heading paragraph*", "heading paragraph paragraph", &s);
}

#[test]
fn content_seq_plus_one() {
    // TS: "accepts a sequence plus matching one element"
    let s = test_schema();
    valid("heading paragraph+", "heading paragraph", &s);
}

#[test]
fn content_seq_plus_multiple() {
    // TS: "accepts a sequence plus matching multiple elts"
    let s = test_schema();
    valid("heading paragraph+", "heading paragraph paragraph", &s);
}

#[test]
fn content_seq_plus_zero_fails() {
    // TS: "fails when a sequence plus has no elements"
    let s = test_schema();
    invalid("heading paragraph+", "heading", &s);
}

#[test]
fn content_seq_plus_misses_start() {
    // TS: "fails when a sequence plus misses its start"
    let s = test_schema();
    invalid("heading paragraph+", "paragraph paragraph", &s);
}

#[test]
fn content_opt_present() {
    // TS: "accepts an optional element being present"
    let s = test_schema();
    valid("image?", "image", &s);
}

#[test]
fn content_opt_absent() {
    // TS: "accepts an optional element being missing"
    let s = test_schema();
    valid("image?", "", &s);
}

#[test]
fn content_opt_twice_fails() {
    // TS: "fails when an optional element is present twice"
    let s = test_schema();
    invalid("image?", "image image", &s);
}

#[test]
fn content_nested_repeat() {
    // TS: "accepts a nested repeat"
    let s = test_schema();
    valid("(heading paragraph+)+",
          "heading paragraph heading paragraph paragraph", &s);
}

#[test]
fn content_nested_repeat_extra_fails() {
    // TS: "fails on extra input after a nested repeat"
    let s = test_schema();
    invalid("(heading paragraph+)+",
            "heading paragraph heading paragraph paragraph horizontal_rule", &s);
}

#[test]
fn content_count_exact() {
    // TS: "accepts a matching count"
    let s = test_schema();
    valid("hard_break{2}", "hard_break hard_break", &s);
}

#[test]
fn content_count_short_fails() {
    // TS: "rejects a count that comes up short"
    let s = test_schema();
    invalid("hard_break{2}", "hard_break", &s);
}

#[test]
fn content_count_too_many_fails() {
    // TS: "rejects a count that has too many elements"
    let s = test_schema();
    invalid("hard_break{2}", "hard_break hard_break hard_break", &s);
}

#[test]
fn content_range_lower() {
    // TS: "accepts a count on the lower bound"
    let s = test_schema();
    valid("hard_break{2, 4}", "hard_break hard_break", &s);
}

#[test]
fn content_range_upper() {
    // TS: "accepts a count on the upper bound"
    let s = test_schema();
    valid("hard_break{2, 4}", "hard_break hard_break hard_break hard_break", &s);
}

#[test]
fn content_range_between() {
    // TS: "accepts a count between the bounds"
    let s = test_schema();
    valid("hard_break{2, 4}", "hard_break hard_break hard_break", &s);
}

#[test]
fn content_range_too_few_fails() {
    // TS: "rejects a sequence with too few elements"
    let s = test_schema();
    invalid("hard_break{2, 4}", "hard_break", &s);
}

#[test]
fn content_range_too_many_fails() {
    // TS: "rejects a sequence with too many elements"
    let s = test_schema();
    invalid("hard_break{2, 4}",
            "hard_break hard_break hard_break hard_break hard_break", &s);
}

#[test]
fn content_range_bad_element_after() {
    // TS: "rejects a sequence with a bad element after it"
    let s = test_schema();
    invalid("hard_break{2, 4} text*", "hard_break hard_break image", &s);
}

#[test]
fn content_range_good_element_after() {
    // TS: "accepts a sequence with a matching element after it"
    let s = test_schema();
    valid("hard_break{2, 4} image?", "hard_break hard_break image", &s);
}

#[test]
fn content_open_range_exact() {
    // TS: "accepts an open range"
    let s = test_schema();
    valid("hard_break{2,}", "hard_break hard_break", &s);
}

#[test]
fn content_open_range_many() {
    // TS: "accepts an open range matching many"
    let s = test_schema();
    valid("hard_break{2,}", "hard_break hard_break hard_break hard_break", &s);
}

#[test]
fn content_open_range_too_few() {
    // TS: "rejects an open range with too few elements"
    let s = test_schema();
    invalid("hard_break{2,}", "hard_break", &s);
}

// ═════════════════════════════════════════════
//  test-content.ts — fillBefore
// ═════════════════════════════════════════════

#[test]
fn fill_empty_when_things_match() {
    // TS: "returns the empty fragment when things match"
    // fill("paragraph horizontal_rule paragraph", doc(p(), hr()), doc(p()), doc())
    let s = test_schema();
    fill_test(
        "paragraph horizontal_rule paragraph",
        Fragment::from_array(vec![p(&s, vec![]), hr(&s)]),
        Fragment::from_array(vec![p(&s, vec![])]),
        Some(Fragment::empty()),
        &s,
    );
}

#[test]
fn fill_adds_node_when_necessary() {
    // TS: "adds a node when necessary"
    // fill("paragraph horizontal_rule paragraph", doc(p()), doc(p()), doc(hr()))
    let s = test_schema();
    fill_test(
        "paragraph horizontal_rule paragraph",
        Fragment::from_array(vec![p(&s, vec![])]),
        Fragment::from_array(vec![p(&s, vec![])]),
        Some(Fragment::from_array(vec![hr(&s)])),
        &s,
    );
}

#[test]
fn fill_star_across_bound() {
    // TS: "accepts an asterisk across the bound"
    // fill("hard_break*", p(br()), p(br()), p())
    let s = test_schema();
    fill_test(
        "hard_break*",
        Fragment::from_array(vec![br(&s)]),
        Fragment::from_array(vec![br(&s)]),
        Some(Fragment::empty()),
        &s,
    );
}

#[test]
fn fill_star_only_left() {
    // TS: "accepts an asterisk only on the left"
    // fill("hard_break*", p(br()), p(), p())
    let s = test_schema();
    fill_test(
        "hard_break*",
        Fragment::from_array(vec![br(&s)]),
        Fragment::empty(),
        Some(Fragment::empty()),
        &s,
    );
}

#[test]
fn fill_star_only_right() {
    // TS: "accepts an asterisk only on the right"
    // fill("hard_break*", p(), p(br()), p())
    let s = test_schema();
    fill_test(
        "hard_break*",
        Fragment::empty(),
        Fragment::from_array(vec![br(&s)]),
        Some(Fragment::empty()),
        &s,
    );
}

#[test]
fn fill_star_no_elements() {
    // TS: "accepts an asterisk with no elements"
    // fill("hard_break*", p(), p(), p())
    let s = test_schema();
    fill_test("hard_break*", Fragment::empty(), Fragment::empty(), Some(Fragment::empty()), &s);
}

#[test]
fn fill_plus_across_bound() {
    // TS: "accepts a plus across the bound"
    // fill("hard_break+", p(br()), p(br()), p())
    let s = test_schema();
    fill_test(
        "hard_break+",
        Fragment::from_array(vec![br(&s)]),
        Fragment::from_array(vec![br(&s)]),
        Some(Fragment::empty()),
        &s,
    );
}

#[test]
fn fill_plus_adds_element() {
    // TS: "adds an element for a content-less plus"
    // fill("hard_break+", p(), p(), p(br()))
    let s = test_schema();
    fill_test(
        "hard_break+",
        Fragment::empty(),
        Fragment::empty(),
        Some(Fragment::from_array(vec![br(&s)])),
        &s,
    );
}

#[test]
fn fill_plus_mismatched_fails() {
    // TS: "fails for a mismatched plus"
    // fill("hard_break+", p(), p(img()), null)
    let s = test_schema();
    fill_test(
        "hard_break+",
        Fragment::empty(),
        Fragment::from_array(vec![img(&s)]),
        None,
        &s,
    );
}

#[test]
fn fill_heading_star_para_star_both() {
    // TS: "accepts asterisk with content on both sides"
    // fill("heading* paragraph*", doc(h1()), doc(p()), doc())
    let s = test_schema();
    fill_test(
        "heading* paragraph*",
        Fragment::from_array(vec![h1(&s, vec![])]),
        Fragment::from_array(vec![p(&s, vec![])]),
        Some(Fragment::empty()),
        &s,
    );
}

#[test]
fn fill_heading_star_para_star_no_after() {
    // TS: "accepts asterisk with no content after"
    // fill("heading* paragraph*", doc(h1()), doc(), doc())
    let s = test_schema();
    fill_test(
        "heading* paragraph*",
        Fragment::from_array(vec![h1(&s, vec![])]),
        Fragment::empty(),
        Some(Fragment::empty()),
        &s,
    );
}

#[test]
fn fill_heading_plus_para_plus_both() {
    // TS: "accepts plus with content on both sides"
    // fill("heading+ paragraph+", doc(h1()), doc(p()), doc())
    let s = test_schema();
    fill_test(
        "heading+ paragraph+",
        Fragment::from_array(vec![h1(&s, vec![])]),
        Fragment::from_array(vec![p(&s, vec![])]),
        Some(Fragment::empty()),
        &s,
    );
}

#[test]
fn fill_heading_plus_para_plus_no_after() {
    // TS: "accepts plus with no content after"
    // fill("heading+ paragraph+", doc(h1()), doc(), doc(p()))
    let s = test_schema();
    fill_test(
        "heading+ paragraph+",
        Fragment::from_array(vec![h1(&s, vec![])]),
        Fragment::empty(),
        Some(Fragment::from_array(vec![p(&s, vec![])])),
        &s,
    );
}

#[test]
fn fill_count_adds_elements() {
    // TS: "adds elements to match a count"
    // fill("hard_break{3}", p(br()), p(br()), p(br()))
    let s = test_schema();
    fill_test(
        "hard_break{3}",
        Fragment::from_array(vec![br(&s)]),
        Fragment::from_array(vec![br(&s)]),
        Some(Fragment::from_array(vec![br(&s)])),
        &s,
    );
}

#[test]
fn fill_count_too_many_fails() {
    // TS: "fails when there are too many elements"
    // fill("hard_break{3}", p(br(), br()), p(br(), br()), null)
    let s = test_schema();
    fill_test(
        "hard_break{3}",
        Fragment::from_array(vec![br(&s), br(&s)]),
        Fragment::from_array(vec![br(&s), br(&s)]),
        None,
        &s,
    );
}

#[test]
fn fill_two_counted_groups() {
    // TS: "adds elements for two counted groups"
    // fill("code_block{2} paragraph{2}", doc(pre()), doc(p()), doc(pre(), p()))
    let s = test_schema();
    fill_test(
        "code_block{2} paragraph{2}",
        Fragment::from_array(vec![pre(&s, vec![])]),
        Fragment::from_array(vec![p(&s, vec![])]),
        Some(Fragment::from_array(vec![pre(&s, vec![]), p(&s, vec![])])),
        &s,
    );
}

#[test]
fn fill_doesnt_include_optional() {
    // TS: "doesn't include optional elements"
    // fill("heading paragraph? horizontal_rule", doc(h1()), doc(), doc(hr()))
    let s = test_schema();
    fill_test(
        "heading paragraph? horizontal_rule",
        Fragment::from_array(vec![h1(&s, vec![])]),
        Fragment::empty(),
        Some(Fragment::from_array(vec![hr(&s)])),
        &s,
    );
}

// fill3 tests

#[test]
fn fill3_completes_sequence() {
    // TS: "completes a sequence"
    // fill3("paragraph horizontal_rule paragraph horizontal_rule paragraph",
    //       doc(p()), doc(p()), doc(p()), doc(hr()), doc(hr()))
    let s = test_schema();
    fill3_test(
        "paragraph horizontal_rule paragraph horizontal_rule paragraph",
        Fragment::from_array(vec![p(&s, vec![])]),
        Fragment::from_array(vec![p(&s, vec![])]),
        Fragment::from_array(vec![p(&s, vec![])]),
        Some(Fragment::from_array(vec![hr(&s)])),
        Some(Fragment::from_array(vec![hr(&s)])),
        &s,
    );
}

#[test]
fn fill3_plus_across_two_bounds() {
    // TS: "accepts plus across two bounds"
    // fill3("code_block+ paragraph+", doc(pre()), doc(pre()), doc(p()), doc(), doc())
    let s = test_schema();
    fill3_test(
        "code_block+ paragraph+",
        Fragment::from_array(vec![pre(&s, vec![])]),
        Fragment::from_array(vec![pre(&s, vec![])]),
        Fragment::from_array(vec![p(&s, vec![])]),
        Some(Fragment::empty()),
        Some(Fragment::empty()),
        &s,
    );
}

#[test]
fn fill3_plus_from_empty() {
    // TS: "fills a plus from empty input"
    // fill3("code_block+ paragraph+", doc(), doc(), doc(), doc(), doc(pre(), p()))
    let s = test_schema();
    fill3_test(
        "code_block+ paragraph+",
        Fragment::empty(),
        Fragment::empty(),
        Fragment::empty(),
        Some(Fragment::empty()),
        Some(Fragment::from_array(vec![pre(&s, vec![]), p(&s, vec![])])),
        &s,
    );
}

#[test]
fn fill3_completes_count() {
    // TS: "completes a count"
    // fill3("code_block{3} paragraph{3}", doc(pre()), doc(p()), doc(),
    //       doc(pre(), pre()), doc(p(), p()))
    let s = test_schema();
    fill3_test(
        "code_block{3} paragraph{3}",
        Fragment::from_array(vec![pre(&s, vec![])]),
        Fragment::from_array(vec![p(&s, vec![])]),
        Fragment::empty(),
        Some(Fragment::from_array(vec![pre(&s, vec![]), pre(&s, vec![])])),
        Some(Fragment::from_array(vec![p(&s, vec![]), p(&s, vec![])])),
        &s,
    );
}

#[test]
fn fill3_non_matching_fails() {
    // TS: "fails on non-matching elements"
    // fill3("paragraph*", doc(p()), doc(pre()), doc(p()), null)
    let s = test_schema();
    fill3_test(
        "paragraph*",
        Fragment::from_array(vec![p(&s, vec![])]),
        Fragment::from_array(vec![pre(&s, vec![])]),
        Fragment::from_array(vec![p(&s, vec![])]),
        None, None,
        &s,
    );
}

#[test]
fn fill3_paragraph4() {
    // TS: "completes a plus across two bounds"
    // fill3("paragraph{4}", doc(p()), doc(p()), doc(p()), doc(), doc(p()))
    let s = test_schema();
    fill3_test(
        "paragraph{4}",
        Fragment::from_array(vec![p(&s, vec![])]),
        Fragment::from_array(vec![p(&s, vec![])]),
        Fragment::from_array(vec![p(&s, vec![])]),
        Some(Fragment::empty()),
        Some(Fragment::from_array(vec![p(&s, vec![])])),
        &s,
    );
}

#[test]
fn fill3_paragraph2_overflow_fails() {
    // TS: "refuses to complete an overflown count across two bounds"
    // fill3("paragraph{2}", doc(p()), doc(p()), doc(p()), null)
    let s = test_schema();
    fill3_test(
        "paragraph{2}",
        Fragment::from_array(vec![p(&s, vec![])]),
        Fragment::from_array(vec![p(&s, vec![])]),
        Fragment::from_array(vec![p(&s, vec![])]),
        None, None,
        &s,
    );
}

// ═════════════════════════════════════════════
//  test-diff.ts — Fragment.findDiffStart/End
// ═════════════════════════════════════════════

fn make_em_mt() -> Arc<MarkType> {
    Arc::new(MarkType { name: "em".into(), rank: 0, excluded: vec![], inclusive: None })
}
fn make_strong_mt() -> Arc<MarkType> {
    Arc::new(MarkType { name: "strong".into(), rank: 1, excluded: vec![], inclusive: None })
}

fn em_text(s: &HashMap<String, Arc<NodeType>>, text: &str) -> Node {
    let em = Arc::new(MarkType { name: "em".into(), rank: 0, excluded: vec![], inclusive: None });
    Node { node_type: Arc::clone(s.get("text").unwrap()), attrs: BTreeMap::new(),
           content: Fragment::empty(), text: Some(text.into()),
           marks: vec![Mark { mark_type: em, attrs: BTreeMap::new() }] }
}
fn strong_text(s: &HashMap<String, Arc<NodeType>>, text: &str) -> Node {
    let st = Arc::new(MarkType { name: "strong".into(), rank: 1, excluded: vec![], inclusive: None });
    Node { node_type: Arc::clone(s.get("text").unwrap()), attrs: BTreeMap::new(),
           content: Fragment::empty(), text: Some(text.into()),
           marks: vec![Mark { mark_type: st, attrs: BTreeMap::new() }] }
}

fn frag_eq(a: &Fragment, b: &Fragment) -> bool {
    if a.child_count() != b.child_count() { return false; }
    a.content.iter().zip(b.content.iter()).all(|(na, nb)| node_eq(na, nb))
}
fn node_eq(a: &Node, b: &Node) -> bool {
    a.same_markup(b) && frag_eq(&a.content, &b.content)
}

// findDiffStart

#[test]
fn diff_start_identical() {
    // TS: "returns null for identical nodes"
    let s = test_schema();
    let a = Fragment::from_array(vec![p(&s, vec![txt(&s, "a"), em_text(&s, "b")]),
                                      p(&s, vec![txt(&s, "hello")]),
                                      bq(&s, vec![h1(&s, vec![txt(&s, "bye")])])]);
    let b = a.clone();
    assert_eq!(a.find_diff_start(&b, 0), None);
}

#[test]
fn diff_start_one_longer() {
    // TS: "notices when one node is longer"
    // a = doc(p("a",em("b")), p("hello"), blockquote(h1("bye"))) + extra paragraph
    // b = a without extra
    // diff at position = a.size - extra.size
    let s = test_schema();
    let shared = vec![
        p(&s, vec![txt(&s, "a"), em_text(&s, "b")]),
        p(&s, vec![txt(&s, "hello")]),
        bq(&s, vec![h1(&s, vec![txt(&s, "bye")])]),
    ];
    let a = Fragment::from_array({
        let mut v = shared.clone();
        v.push(p(&s, vec![]));
        v
    });
    let b = Fragment::from_array(shared.clone());
    let diff = a.find_diff_start(&b, 0);
    assert!(diff.is_some(), "expected diff");
    // diff should be at the start of the extra paragraph
    assert_eq!(diff.unwrap(), b.size);
}

#[test]
fn diff_start_one_shorter() {
    // TS: "notices when one node is shorter"
    let s = test_schema();
    let shared = vec![
        p(&s, vec![txt(&s, "a"), em_text(&s, "b")]),
        p(&s, vec![txt(&s, "hello")]),
        bq(&s, vec![h1(&s, vec![txt(&s, "bye")])]),
    ];
    let a = Fragment::from_array({
        let mut v = shared.clone();
        v.push(p(&s, vec![]));
        v
    });
    let b = Fragment::from_array(shared);
    // diff from b's perspective when a has extra
    let diff = b.find_diff_start(&a, 0);
    assert!(diff.is_some());
}

#[test]
fn diff_start_differing_marks() {
    // TS: "notices differing marks" — "a<a>" em("b") vs "a" strong("b")
    let s = test_schema();
    let a = Fragment::from_array(vec![
        p(&s, vec![txt(&s, "a"), em_text(&s, "b")])
    ]);
    let b = Fragment::from_array(vec![
        p(&s, vec![txt(&s, "a"), strong_text(&s, "b")])
    ]);
    let diff = a.find_diff_start(&b, 0);
    assert!(diff.is_some());
    // diff starts at "a" position + 1 (after the "a" text in p)
    // p starts at 0, inside p at 1, after "a" (1 char) = 2
    assert_eq!(diff.unwrap(), 2);
}

#[test]
fn diff_start_text_difference() {
    // TS: "stops at a different character" — "foobar" vs "foocar" → diff at 3
    let s = test_schema();
    let a = Fragment::from_array(vec![p(&s, vec![txt(&s, "foobar")])]);
    let b = Fragment::from_array(vec![p(&s, vec![txt(&s, "foocar")])]);
    let diff = a.find_diff_start(&b, 0);
    // inside p: p.start=0, content starts at 1. "foo" matches, diff at 1+3=4
    assert_eq!(diff, Some(4));
}

#[test]
fn diff_start_different_node_type() {
    // TS: "stops at a different node type"
    let s = test_schema();
    let a = Fragment::from_array(vec![p(&s, vec![txt(&s, "a")]), p(&s, vec![txt(&s, "b")])]);
    let b = Fragment::from_array(vec![p(&s, vec![txt(&s, "a")]), h1(&s, vec![txt(&s, "b")])]);
    let diff = a.find_diff_start(&b, 0);
    // p("a").nodeSize = 3, so second block starts at pos 3
    assert_eq!(diff, Some(3));
}

#[test]
fn diff_start_difference_at_start() {
    // TS: "works when the difference is at the start"
    let s = test_schema();
    let a = Fragment::from_array(vec![p(&s, vec![txt(&s, "b")])]);
    let b = Fragment::from_array(vec![h1(&s, vec![txt(&s, "b")])]);
    assert_eq!(a.find_diff_start(&b, 0), Some(0));
}

// findDiffEnd

#[test]
fn diff_end_identical() {
    // TS: "returns null when there is no difference"
    let s = test_schema();
    let a = Fragment::from_array(vec![
        p(&s, vec![txt(&s, "a"), em_text(&s, "b")]),
        p(&s, vec![txt(&s, "hello")]),
    ]);
    let b = a.clone();
    assert_eq!(a.find_diff_end(&b, a.size, b.size), None);
}

#[test]
fn diff_end_second_longer() {
    // TS: "notices when the second doc is longer"
    let s = test_schema();
    let base = Fragment::from_array(vec![p(&s, vec![txt(&s, "a")])]);
    let longer = Fragment::from_array(vec![p(&s, vec![txt(&s, "b")]), p(&s, vec![txt(&s, "a")])]);
    let diff = base.find_diff_end(&longer, base.size, longer.size);
    assert!(diff.is_some());
    assert_eq!(diff.unwrap().0, 0); // diff starts at beginning of base
}

#[test]
fn diff_end_different_text() {
    // TS: "spots different text" — "foob<a>ar" vs "foocar" → diff.a at 4
    let s = test_schema();
    let a = Fragment::from_array(vec![p(&s, vec![txt(&s, "foobar")])]);
    let b = Fragment::from_array(vec![p(&s, vec![txt(&s, "foocar")])]);
    // "ar" is shared at end. diff at position after "foob" from start
    // a.size = p.nodeSize = 1+6+1... wait p content "foobar" = 6 chars
    // but from_array merges text, so p has one text "foobar"
    // a.size = p.nodeSize = 6+2=8
    let diff = a.find_diff_end(&b, a.size, b.size);
    assert!(diff.is_some());
    // "ar" (2 chars) shared from end → pos starts at 7 inside p content,
    // decrements by 2 → pos=5. found.a=5 (p content offset 4 = after "foob").
    assert_eq!(diff.unwrap().0, 5);
}

#[test]
fn diff_end_difference_at_end() {
    // TS: "notices a difference at the end"
    let s = test_schema();
    let a = Fragment::from_array(vec![p(&s, vec![txt(&s, "b")])]);
    let b = Fragment::from_array(vec![h1(&s, vec![txt(&s, "b")])]);
    let diff = a.find_diff_end(&b, a.size, b.size);
    assert!(diff.is_some());
}

// ═════════════════════════════════════════════
//  test-resolve.ts — main structure test
// ═════════════════════════════════════════════

/// const testDoc = doc(p("ab"), blockquote(p(em("cd"), "ef")))
fn make_test_doc(s: &HashMap<String, Arc<NodeType>>) -> Node {
    let em_mark = Arc::new(MarkType { name: "em".into(), rank: 0, excluded: vec![], inclusive: None });
    let cd = Node { node_type: Arc::clone(s.get("text").unwrap()), attrs: BTreeMap::new(),
                    content: Fragment::empty(), text: Some("cd".into()),
                    marks: vec![Mark { mark_type: em_mark, attrs: BTreeMap::new() }] };
    let ef = txt(s, "ef");
    let inner_p = p(s, vec![cd, ef]);
    let blk = bq(s, vec![inner_p]);
    let ab = txt(s, "ab");
    let p1 = p(s, vec![ab]);
    doc(s, vec![p1, blk])
}

#[test]
fn resolve_structure() {
    // TS: "should reflect the document structure"
    use super::resolvedpos::ResolvedPos;

    let s = test_schema();
    let d = make_test_doc(&s);

    // doc.content.size = p("ab").nodeSize + blockquote(...).nodeSize
    // p("ab"): content.size=2, nodeSize=4
    // inner_p(em("cd"),"ef"): content.size=4, nodeSize=6
    // blockquote(inner_p): content.size=6, nodeSize=8
    // doc.content.size = 4+8 = 12
    assert_eq!(d.content.size, 12, "doc.content.size");

    let rp0 = ResolvedPos::resolve(&d, 0).unwrap();
    // pos=0: depth=0, in doc, index=0, before p1
    assert_eq!(rp0.depth(), 0);
    assert_eq!(rp0.parent().node_type.name, "doc");
    assert_eq!(rp0.index(None), 0);

    let rp1 = ResolvedPos::resolve(&d, 1).unwrap();
    // pos=1: depth=1, in p1, parentOffset=0
    assert_eq!(rp1.depth(), 1);
    assert_eq!(rp1.parent().node_type.name, "paragraph");
    assert_eq!(rp1.parent_offset, 0);
    assert_eq!(rp1.start(Some(1)), 1);
    assert_eq!(rp1.end(Some(1)), 3); // 1+2=3

    let rp4 = ResolvedPos::resolve(&d, 4).unwrap();
    // pos=4: depth=0, after p1
    assert_eq!(rp4.depth(), 0);
    assert_eq!(rp4.parent_offset, 4);

    let rp5 = ResolvedPos::resolve(&d, 5).unwrap();
    // pos=5: depth=1, in blockquote, before inner_p
    assert_eq!(rp5.depth(), 1);
    assert_eq!(rp5.parent().node_type.name, "blockquote");
    assert_eq!(rp5.start(Some(1)), 5);
    assert_eq!(rp5.end(Some(1)), 11); // 5+6=11

    let rp6 = ResolvedPos::resolve(&d, 6).unwrap();
    // pos=6: depth=2, in inner_p
    assert_eq!(rp6.depth(), 2);
    assert_eq!(rp6.parent().node_type.name, "paragraph");
    assert_eq!(rp6.start(Some(2)), 6);
    assert_eq!(rp6.end(Some(2)), 10); // 6+4=10

    // before/after at depth 1 and 2
    // before(2) = start(2) - 1 = 5
    assert_eq!(rp6.before(Some(2)).unwrap(), 5);
    // after(2) = end(2) + 1 = 11
    assert_eq!(rp6.after(Some(2)).unwrap(), 11);

    // before(1) = start(1) - 1 = 4
    assert_eq!(rp6.before(Some(1)).unwrap(), 4);
    // after(1) = end(1) + 1 = 12
    assert_eq!(rp6.after(Some(1)).unwrap(), 12);

    let rp12 = ResolvedPos::resolve(&d, 12).unwrap();
    // pos=12: depth=0, after blockquote
    assert_eq!(rp12.depth(), 0);
    assert_eq!(rp12.parent_offset, 12);
}

#[test]
fn resolve_pos_at_index() {
    // TS: "has a working posAtIndex method"
    // doc(blockquote(p("one"), blockquote(p("two ", em("three")), p("four"))))
    use super::resolvedpos::ResolvedPos;

    let s = test_schema();
    let em_mark = Arc::new(MarkType { name: "em".into(), rank: 0, excluded: vec![], inclusive: None });
    let three = Node { node_type: Arc::clone(s.get("text").unwrap()), attrs: BTreeMap::new(),
                       content: Fragment::empty(), text: Some("three".into()),
                       marks: vec![Mark { mark_type: em_mark, attrs: BTreeMap::new() }] };
    let d = doc(&s, vec![
        bq(&s, vec![
            p(&s, vec![txt(&s, "one")]),
            bq(&s, vec![
                p(&s, vec![txt(&s, "two "), three]),
                p(&s, vec![txt(&s, "four")]),
            ]),
        ])
    ]);

    // Verify doc content size and then posAtIndex
    // outer bq: contains p("one") and inner bq
    // p("one").nodeSize = 5 (3 chars + 2 tags)
    // inner bq: contains p("two three") and p("four")
    // p("two ") = 4, em("three") = 5 → content.size=9, p.nodeSize=11
    // p("four").nodeSize = 6
    // inner bq.content.size = 11+6=17, nodeSize=19
    // outer bq.content.size = 5+19=24, nodeSize=26
    // doc.content.size = 26

    // Resolve at start of em("three") = pos 12 (approximately)
    // doc[0] = outer bq, bq.start=1
    // p("one").nodeSize=5, so after p("one") = pos 6
    // inner bq.start = 7
    // p("two three").content.size=9, p.start=8
    // "two " = 4 chars, em("three") starts at pos 8+4=12
    let rp = ResolvedPos::resolve(&d, 12).unwrap();

    // posAtIndex(0) at depth=2 = start of inner_p's content = 8
    // posAtIndex(1) at depth=2 = start of em("three") = 12
    assert_eq!(rp.pos_at_index(0, None), 8,  "posAtIndex(0) default depth");
    assert_eq!(rp.pos_at_index(1, None), 12, "posAtIndex(1) default depth");
}

// ═════════════════════════════════════════════
//  test-mark.ts — sameSet, addToSet, removeFromSet
// ═════════════════════════════════════════════

fn mk(mt: Arc<MarkType>) -> Mark { Mark { mark_type: mt, attrs: BTreeMap::new() } }
fn mk_attrs(mt: Arc<MarkType>, attrs: Vec<(&str, &str)>) -> Mark {
    let mut a = BTreeMap::new();
    for (k, v) in attrs { a.insert(k.into(), super::AttrValue::Str(v.into())); }
    Mark { mark_type: mt, attrs: a }
}

// 实际 rank 顺序来自 prosemirror-test-builder schema：link=0, em=1, strong=2, code=3
fn em_mt()     -> Arc<MarkType> { make_mt("em",     1) }
fn strong_mt() -> Arc<MarkType> { make_mt("strong", 2) }
fn link_mt() -> Arc<MarkType> {
    // link 排斥同名类型（自排斥）。用同名哨兵实例填入 excluded；
    // MarkType::excludes 的 name 兜底比较保证测试正确性，无需循环引用。
    let sentinel = Arc::new(MarkType {
        name: "link".into(), rank: 0, excluded: vec![], inclusive: None,
    });
    Arc::new(MarkType {
        name: "link".into(), rank: 0,
        excluded: vec![sentinel],
        inclusive: None,
    })
}
fn code_mt()   -> Arc<MarkType> { make_mt("code",   3) }

fn link_with(lmt: &Arc<MarkType>, href: &str) -> Mark {
    mk_attrs(Arc::clone(lmt), vec![("href", href)])
}
fn link_t_with(lmt: &Arc<MarkType>, href: &str, title: &str) -> Mark {
    mk_attrs(Arc::clone(lmt), vec![("href", href), ("title", title)])
}

// 仅供不涉及跨 Mark 比较的单一调用场景使用
fn link(href: &str) -> Mark { link_with(&link_mt(), href) }
fn link_t(href: &str, title: &str) -> Mark { link_t_with(&link_mt(), href, title) }

#[test]
fn mark_same_set_empty() {
    // TS: "returns true for two empty sets"
    assert!(Mark::same_set(&[], &[]));
}

#[test]
fn mark_same_set_identical() {
    // TS: "returns true for simple identical sets"
    let em = mk(em_mt()); let st = mk(strong_mt());
    assert!(Mark::same_set(&[em.clone(), st.clone()], &[em, st]));
}

#[test]
fn mark_same_set_different() {
    // TS: "returns false for different sets"
    let em = mk(em_mt()); let st = mk(strong_mt()); let co = mk(code_mt());
    assert!(!Mark::same_set(&[em.clone(), st.clone()], &[em, co]));
}

#[test]
fn mark_same_set_size_differs() {
    // TS: "returns false when set size differs"
    let em = mk(em_mt()); let st = mk(strong_mt()); let co = mk(code_mt());
    assert!(!Mark::same_set(&[em.clone(), st.clone()], &[em, st, co]));
}

#[test]
fn mark_same_set_links_equal() {
    // TS: "recognizes identical links in set"
    let lmt = link_mt();
    let co = mk(code_mt());
    assert!(Mark::same_set(
        &[link_with(&lmt, "http://foo"), co.clone()],
        &[link_with(&lmt, "http://foo"), co],
    ));
}

#[test]
fn mark_same_set_links_different() {
    // TS: "recognizes different links in set"
    let co = mk(code_mt());
    assert!(!Mark::same_set(
        &[link("http://foo"), co.clone()],
        &[link("http://bar"), co],
    ));
}

#[test]
fn mark_eq_same_link() {
    // TS: "considers identical links to be the same"
    let lmt = link_mt();
    assert!(link_with(&lmt, "http://foo").eq(&link_with(&lmt, "http://foo")));
}

#[test]
fn mark_eq_different_link() {
    // TS: "considers different links to differ"
    assert!(!link("http://foo").eq(&link("http://bar")));
}

#[test]
fn mark_eq_different_title() {
    // TS: "considers links with different titles to differ"
    assert!(!link_t("http://foo", "A").eq(&link_t("http://foo", "B")));
}

#[test]
fn mark_add_to_empty() {
    // TS: "can add to the empty set"
    let em = mk(em_mt());
    let result = em.add_to_set(&[]);
    assert!(Mark::same_set(&result, &[em]));
}

#[test]
fn mark_add_noop_in_set() {
    // TS: "is a no-op when the added thing is in set"
    let em = mk(em_mt());
    let result = em.add_to_set(&[em.clone()]);
    assert!(Mark::same_set(&result, &[em]));
}

#[test]
fn mark_add_lower_rank_before() {
    // TS: "adds marks with lower rank before others"
    let em = mk(em_mt()); let st = mk(strong_mt());
    let result = em.add_to_set(&[st.clone()]);
    assert!(Mark::same_set(&result, &[em, st]));
}

#[test]
fn mark_add_higher_rank_after() {
    // TS: "adds marks with higher rank after others"
    let em = mk(em_mt()); let st = mk(strong_mt());
    let result = st.add_to_set(&[em.clone()]);
    assert!(Mark::same_set(&result, &[em, st]));
}

#[test]
fn mark_add_replaces_same_type_different_attrs() {
    // TS: "replaces different marks with new attributes"
    let lmt = link_mt();
    let em = mk(em_mt());
    let result = link_with(&lmt, "http://bar").add_to_set(&[link_with(&lmt, "http://foo"), em.clone()]);
    assert!(Mark::same_set(&result, &[link_with(&lmt, "http://bar"), em]));
}

#[test]
fn mark_add_noop_existing_link() {
    // TS: "does nothing when adding an existing link"
    let lmt = link_mt();
    let em = mk(em_mt());
    let result = link_with(&lmt, "http://foo").add_to_set(&[em.clone(), link_with(&lmt, "http://foo")]);
    assert!(Mark::same_set(&result, &[em, link_with(&lmt, "http://foo")]));
}

#[test]
fn mark_add_code_at_end() {
    // TS: "puts code marks at the end"
    let lmt = link_mt();
    let em = mk(em_mt()); let st = mk(strong_mt()); let co = mk(code_mt());
    let result = co.add_to_set(&[em.clone(), st.clone(), link_with(&lmt, "http://foo")]);
    assert!(Mark::same_set(&result, &[em, st, link_with(&lmt, "http://foo"), co]));
}

#[test]
fn mark_add_middle_rank() {
    // TS: "puts marks with middle rank in the middle"
    let em = mk(em_mt()); let st = mk(strong_mt()); let co = mk(code_mt());
    let result = st.add_to_set(&[em.clone(), co.clone()]);
    assert!(Mark::same_set(&result, &[em, st, co]));
}

#[test]
fn mark_remove_noop_empty() {
    // TS: "is a no-op for the empty set"
    let em = mk(em_mt());
    assert!(Mark::same_set(&em.remove_from_set(&[]), &[]));
}

#[test]
fn mark_remove_last_mark() {
    // TS: "can remove the last mark from a set"
    let em = mk(em_mt());
    assert!(Mark::same_set(&em.remove_from_set(&[em.clone()]), &[]));
}

#[test]
fn mark_remove_not_in_set() {
    // TS: "is a no-op when the mark isn't in the set"
    let em = mk(em_mt()); let st = mk(strong_mt());
    assert!(Mark::same_set(&st.remove_from_set(&[em.clone()]), &[em]));
}

#[test]
fn mark_remove_with_attrs() {
    // TS: "can remove a mark with attributes"
    let lmt = link_mt();
    assert!(Mark::same_set(
        &link_with(&lmt, "http://foo").remove_from_set(&[link_with(&lmt, "http://foo")]),
        &[],
    ));
}

#[test]
fn mark_remove_attrs_differ() {
    // TS: "doesn't remove a mark when its attrs differ"
    let lmt = link_mt();
    assert!(Mark::same_set(
        &link_t_with(&lmt, "http://foo", "title").remove_from_set(&[link_with(&lmt, "http://foo")]),
        &[link_with(&lmt, "http://foo")],
    ));
}
