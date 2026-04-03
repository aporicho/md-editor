use super::fragment::Fragment;

/// 找到两个片段第一个不同的位置。
///
/// 对应 prosemirror-model/src/diff.ts findDiffStart
pub fn find_diff_start(a: &Fragment, b: &Fragment, pos: usize) -> Option<usize> {
    let mut pos = pos;
    for i in 0.. {
        if i == a.child_count() || i == b.child_count() {
            return if a.child_count() == b.child_count() {
                None
            } else {
                Some(pos)
            };
        }

        let child_a = a.child(i);
        let child_b = b.child(i);

        if std::ptr::eq(child_a, child_b) {
            pos += child_a.node_size();
            continue;
        }

        if !child_a.same_markup(child_b) {
            return Some(pos);
        }

        if child_a.is_text() && child_a.text() != child_b.text() {
            let ta: Vec<char> = child_a.text().unwrap_or("").chars().collect();
            let tb: Vec<char> = child_b.text().unwrap_or("").chars().collect();
            let mut j = 0;
            while j < ta.len() && j < tb.len() && ta[j] == tb[j] {
                j += 1;
                pos += 1;
            }
            return Some(pos);
        }

        if child_a.content.size > 0 || child_b.content.size > 0 {
            if let Some(inner) = find_diff_start(&child_a.content, &child_b.content, pos + 1) {
                return Some(inner);
            }
        }

        pos += child_a.node_size();
    }
    unreachable!()
}

/// 找到两个片段最后一个不同的位置（从末尾搜索）。
///
/// 对应 prosemirror-model/src/diff.ts findDiffEnd
pub fn find_diff_end(
    a: &Fragment,
    b: &Fragment,
    pos_a: usize,
    pos_b: usize,
) -> Option<(usize, usize)> {
    let mut ia = a.child_count();
    let mut ib = b.child_count();
    let mut pos_a = pos_a;
    let mut pos_b = pos_b;

    loop {
        if ia == 0 || ib == 0 {
            return if ia == ib {
                None
            } else {
                Some((pos_a, pos_b))
            };
        }

        ia -= 1;
        ib -= 1;
        let child_a = a.child(ia);
        let child_b = b.child(ib);
        let size = child_a.node_size();

        if std::ptr::eq(child_a, child_b) {
            pos_a -= size;
            pos_b -= size;
            continue;
        }

        if !child_a.same_markup(child_b) {
            return Some((pos_a, pos_b));
        }

        if child_a.is_text() && child_a.text() != child_b.text() {
            let ta: Vec<char> = child_a.text().unwrap_or("").chars().collect();
            let tb: Vec<char> = child_b.text().unwrap_or("").chars().collect();
            let min_size = ta.len().min(tb.len());
            let mut same = 0;
            while same < min_size
                && ta[ta.len() - same - 1] == tb[tb.len() - same - 1]
            {
                same += 1;
                pos_a -= 1;
                pos_b -= 1;
            }
            return Some((pos_a, pos_b));
        }

        if child_a.content.size > 0 || child_b.content.size > 0 {
            if let Some(inner) = find_diff_end(
                &child_a.content,
                &child_b.content,
                pos_a - 1,
                pos_b - 1,
            ) {
                return Some(inner);
            }
        }

        pos_a -= size;
        pos_b -= size;
    }
}
