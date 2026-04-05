// src/model/tests_content.rs
//
// fillBefore tests ported from prosemirror-model/test/test-content.ts
// describe("fillBefore") block: 17 fill + 7 fill3 = 24 tests
//
#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use indexmap::IndexMap;

    use crate::model::content::ContentMatch;
    use crate::model::fragment::Fragment;
    use crate::model::test_builder::{
        test_schema, br, doc, h1, hr, img, p, pre,
    };

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Parse a content expression against the test_schema node types.
    fn get(expr: &str) -> Arc<ContentMatch> {
        let schema = test_schema();
        let is_inline: IndexMap<String, bool> = schema
            .nodes
            .iter()
            .map(|(n, nt)| (n.clone(), nt.is_inline()))
            .collect();
        let groups: IndexMap<String, Vec<String>> = {
            let mut m: IndexMap<String, Vec<String>> = IndexMap::new();
            for (name, nt) in &schema.nodes {
                for g in &nt.groups {
                    m.entry(g.clone()).or_insert_with(Vec::new).push(name.clone());
                }
            }
            m
        };
        let has_required: IndexMap<String, bool> = schema
            .nodes
            .iter()
            .map(|(n, nt)| (n.clone(), nt.has_required_attrs()))
            .collect();
        ContentMatch::parse(expr, &is_inline, &groups, &has_required)
            .unwrap_or_else(|e| panic!("content parse failed for '{}': {:?}", expr, e))
    }

    /// Port of TS fill():
    /// get(expr).matchFragment(before_content)!.fillBefore(after_content, true)
    /// then compare result to `expected` (or assert None).
    fn fill(expr: &str, before_content: Fragment, after_content: Fragment, expected: Option<Fragment>) {
        let schema = test_schema();
        let cm = get(expr);
        let matched = cm
            .match_fragment(&before_content, 0, before_content.child_count())
            .unwrap_or_else(|| panic!("fill: matchFragment failed for before_content in '{}'", expr));
        let filled = matched.fill_before(schema, &after_content, true, 0);
        match expected {
            Some(exp) => {
                let filled = filled
                    .unwrap_or_else(|| panic!("fill: expected Some but got None for '{}'", expr));
                assert!(
                    filled.eq(&exp),
                    "fill '{}': result mismatch — got size {}, expected size {}",
                    expr,
                    filled.size,
                    exp.size
                );
            }
            None => assert!(
                filled.is_none(),
                "fill '{}': expected None but got Some",
                expr
            ),
        }
    }

    /// Port of TS fill3():
    /// a = get(expr).matchFragment(before)!.fillBefore(mid, false)
    /// b = a && get(expr).matchFragment(before.append(a).append(mid))!.fillBefore(after, true)
    fn fill3(
        expr: &str,
        before: Fragment,
        mid: Fragment,
        after: Fragment,
        expected_left: Option<Fragment>,
        expected_right: Option<Fragment>,
    ) {
        let schema = test_schema();
        let cm = get(expr);
        let a = cm
            .match_fragment(&before, 0, before.child_count())
            .unwrap_or_else(|| panic!("fill3: matchFragment failed for `before` in '{}'", expr))
            .fill_before(schema, &mid, false, 0);
        let b = a.as_ref().and_then(|fill_a| {
            let combined = before.append(fill_a).append(&mid);
            cm.match_fragment(&combined, 0, combined.child_count())
                .unwrap_or_else(|| panic!("fill3: matchFragment failed for combined in '{}'", expr))
                .fill_before(schema, &after, true, 0)
        });
        match expected_left {
            Some(left) => {
                let a = a.unwrap_or_else(|| panic!("fill3 '{}': a should be Some", expr));
                assert!(
                    a.eq(&left),
                    "fill3 '{}': a mismatch — got size {}, expected size {}",
                    expr,
                    a.size,
                    left.size
                );
                let right =
                    expected_right.unwrap_or_else(|| panic!("fill3 '{}': expected_right is None when expected_left is Some", expr));
                let b = b.unwrap_or_else(|| panic!("fill3 '{}': b should be Some", expr));
                assert!(
                    b.eq(&right),
                    "fill3 '{}': b mismatch — got size {}, expected size {}",
                    expr,
                    b.size,
                    right.size
                );
            }
            None => assert!(b.is_none(), "fill3 '{}': b should be None", expr),
        }
    }

    // -----------------------------------------------------------------------
    // fill() tests (17)
    // -----------------------------------------------------------------------

    #[test]
    fn fill_returns_empty_when_things_match() {
        fill(
            "paragraph horizontal_rule paragraph",
            doc(&[p(&[]), hr()]).content,
            doc(&[p(&[])]).content,
            Some(doc(&[]).content),
        )
    }

    #[test]
    fn fill_adds_a_node_when_necessary() {
        fill(
            "paragraph horizontal_rule paragraph",
            doc(&[p(&[])]).content,
            doc(&[p(&[])]).content,
            Some(doc(&[hr()]).content),
        )
    }

    #[test]
    fn fill_accepts_asterisk_across_the_bound() {
        fill(
            "hard_break*",
            p(&[br()]).content,
            p(&[br()]).content,
            Some(p(&[]).content),
        )
    }

    #[test]
    fn fill_accepts_asterisk_only_on_the_left() {
        fill(
            "hard_break*",
            p(&[br()]).content,
            p(&[]).content,
            Some(p(&[]).content),
        )
    }

    #[test]
    fn fill_accepts_asterisk_only_on_the_right() {
        fill(
            "hard_break*",
            p(&[]).content,
            p(&[br()]).content,
            Some(p(&[]).content),
        )
    }

    #[test]
    fn fill_accepts_asterisk_with_no_elements() {
        fill(
            "hard_break*",
            p(&[]).content,
            p(&[]).content,
            Some(p(&[]).content),
        )
    }

    #[test]
    fn fill_accepts_plus_across_the_bound() {
        fill(
            "hard_break+",
            p(&[br()]).content,
            p(&[br()]).content,
            Some(p(&[]).content),
        )
    }

    #[test]
    fn fill_adds_element_for_content_less_plus() {
        fill(
            "hard_break+",
            p(&[]).content,
            p(&[]).content,
            Some(p(&[br()]).content),
        )
    }

    #[test]
    fn fill_fails_for_mismatched_plus() {
        fill(
            "hard_break+",
            p(&[]).content,
            p(&[img()]).content,
            None,
        )
    }

    #[test]
    fn fill_accepts_asterisk_with_content_on_both_sides() {
        fill(
            "heading* paragraph*",
            doc(&[h1(&[])]).content,
            doc(&[p(&[])]).content,
            Some(doc(&[]).content),
        )
    }

    #[test]
    fn fill_accepts_asterisk_with_no_content_after() {
        fill(
            "heading* paragraph*",
            doc(&[h1(&[])]).content,
            doc(&[]).content,
            Some(doc(&[]).content),
        )
    }

    #[test]
    fn fill_accepts_plus_with_content_on_both_sides() {
        fill(
            "heading+ paragraph+",
            doc(&[h1(&[])]).content,
            doc(&[p(&[])]).content,
            Some(doc(&[]).content),
        )
    }

    #[test]
    fn fill_accepts_plus_with_no_content_after() {
        fill(
            "heading+ paragraph+",
            doc(&[h1(&[])]).content,
            doc(&[]).content,
            Some(doc(&[p(&[])]).content),
        )
    }

    #[test]
    fn fill_adds_elements_to_match_a_count() {
        fill(
            "hard_break{3}",
            p(&[br()]).content,
            p(&[br()]).content,
            Some(p(&[br()]).content),
        )
    }

    #[test]
    fn fill_fails_when_too_many_elements() {
        fill(
            "hard_break{3}",
            p(&[br(), br()]).content,
            p(&[br(), br()]).content,
            None,
        )
    }

    #[test]
    fn fill_adds_elements_for_two_counted_groups() {
        fill(
            "code_block{2} paragraph{2}",
            doc(&[pre(&[])]).content,
            doc(&[p(&[])]).content,
            Some(doc(&[pre(&[]), p(&[])]).content),
        )
    }

    #[test]
    fn fill_does_not_include_optional_elements() {
        fill(
            "heading paragraph? horizontal_rule",
            doc(&[h1(&[])]).content,
            doc(&[]).content,
            Some(doc(&[hr()]).content),
        )
    }

    // -----------------------------------------------------------------------
    // fill3() tests (7)
    // -----------------------------------------------------------------------

    #[test]
    fn fill3_completes_a_sequence() {
        fill3(
            "paragraph horizontal_rule paragraph horizontal_rule paragraph",
            doc(&[p(&[])]).content,
            doc(&[p(&[])]).content,
            doc(&[p(&[])]).content,
            Some(doc(&[hr()]).content),
            Some(doc(&[hr()]).content),
        )
    }

    #[test]
    fn fill3_accepts_plus_across_two_bounds() {
        fill3(
            "code_block+ paragraph+",
            doc(&[pre(&[])]).content,
            doc(&[pre(&[])]).content,
            doc(&[p(&[])]).content,
            Some(doc(&[]).content),
            Some(doc(&[]).content),
        )
    }

    #[test]
    fn fill3_fills_a_plus_from_empty_input() {
        fill3(
            "code_block+ paragraph+",
            doc(&[]).content,
            doc(&[]).content,
            doc(&[]).content,
            Some(doc(&[]).content),
            Some(doc(&[pre(&[]), p(&[])]).content),
        )
    }

    #[test]
    fn fill3_completes_a_count() {
        fill3(
            "code_block{3} paragraph{3}",
            doc(&[pre(&[])]).content,
            doc(&[p(&[])]).content,
            doc(&[]).content,
            Some(doc(&[pre(&[]), pre(&[])]).content),
            Some(doc(&[p(&[]), p(&[])]).content),
        )
    }

    #[test]
    fn fill3_fails_on_non_matching_elements() {
        fill3(
            "paragraph*",
            doc(&[p(&[])]).content,
            doc(&[pre(&[])]).content,
            doc(&[p(&[])]).content,
            None,
            None,
        )
    }

    #[test]
    fn fill3_completes_a_plus_across_two_bounds() {
        fill3(
            "paragraph{4}",
            doc(&[p(&[])]).content,
            doc(&[p(&[])]).content,
            doc(&[p(&[])]).content,
            Some(doc(&[]).content),
            Some(doc(&[p(&[])]).content),
        )
    }

    #[test]
    fn fill3_refuses_to_complete_overflown_count() {
        fill3(
            "paragraph{2}",
            doc(&[p(&[])]).content,
            doc(&[p(&[])]).content,
            doc(&[p(&[])]).content,
            None,
            None,
        )
    }
}
