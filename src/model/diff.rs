// src/model/diff.rs
use crate::model::fragment::Fragment;

/// Find the first position at which fragment `a` and fragment `b` differ, or `None` if equal.
///
/// Walks children in order; for text nodes with the same markup, finds the
/// first differing character.
pub fn find_diff_start(a: &Fragment, b: &Fragment, pos: usize) -> Option<usize> {
    let mut pos = pos;
    let mut i = 0usize;
    loop {
        let a_count = a.child_count();
        let b_count = b.child_count();
        if i == a_count || i == b_count {
            return if a_count == b_count { None } else { Some(pos) };
        }
        let child_a = &a.content[i];
        let child_b = &b.content[i];

        // Same pointer: identical node, skip
        if child_a.ptr_eq(child_b) {
            pos += child_a.node_size();
            i += 1;
            continue;
        }

        if !child_a.same_markup(child_b) {
            return Some(pos);
        }

        if child_a.is_text() {
            let ta = child_a.text.as_deref().unwrap_or("");
            let tb = child_b.text.as_deref().unwrap_or("");
            if ta != tb {
                let common = ta.chars().zip(tb.chars()).take_while(|(a, b)| a == b).count();
                return Some(pos + common);
            }
        }

        if child_a.content.size > 0 || child_b.content.size > 0 {
            if let Some(inner) = find_diff_start(&child_a.content, &child_b.content, pos + 1) {
                return Some(inner);
            }
        }

        pos += child_a.node_size();
        i += 1;
    }
}

/// Find the first position from the end at which fragment `a` and fragment `b` differ.
///
/// Returns `(pos_in_a, pos_in_b)` or `None` if equal.
pub fn find_diff_end(
    a: &Fragment,
    b: &Fragment,
    pos_a: usize,
    pos_b: usize,
) -> Option<(usize, usize)> {
    let mut pos_a = pos_a;
    let mut pos_b = pos_b;
    let mut ia = a.child_count();
    let mut ib = b.child_count();

    loop {
        if ia == 0 || ib == 0 {
            return if ia == ib { None } else { Some((pos_a, pos_b)) };
        }
        ia -= 1;
        ib -= 1;
        let child_a = &a.content[ia];
        let child_b = &b.content[ib];
        let size = child_a.node_size();

        if child_a.ptr_eq(child_b) {
            pos_a -= size;
            pos_b -= size;
            continue;
        }

        if !child_a.same_markup(child_b) {
            return Some((pos_a, pos_b));
        }

        if child_a.is_text() {
            let ta = child_a.text.as_deref().unwrap_or("");
            let tb = child_b.text.as_deref().unwrap_or("");
            if ta != tb {
                let ta_chars: Vec<char> = ta.chars().collect();
                let tb_chars: Vec<char> = tb.chars().collect();
                let min_size = ta_chars.len().min(tb_chars.len());
                let mut same = 0usize;
                while same < min_size
                    && ta_chars[ta_chars.len() - same - 1]
                        == tb_chars[tb_chars.len() - same - 1]
                {
                    same += 1;
                    pos_a -= 1;
                    pos_b -= 1;
                }
                return Some((pos_a, pos_b));
            }
        }

        if child_a.content.size > 0 || child_b.content.size > 0 {
            if let Some(inner) =
                find_diff_end(&child_a.content, &child_b.content, pos_a - 1, pos_b - 1)
            {
                return Some(inner);
            }
        }

        pos_a -= size;
        pos_b -= size;
    }
}
