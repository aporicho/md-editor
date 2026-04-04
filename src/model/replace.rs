use super::fragment::Fragment;
use super::node::Node;
use super::resolvedpos::ResolvedPos;

/// replace 操作失败时的错误类型。
#[derive(Debug, Clone)]
pub struct ReplaceError(pub String);

impl std::fmt::Display for ReplaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ReplaceError: {}", self.0)
    }
}

/// 文档的局部片段，记录内容和两侧的开放深度。
///
/// 对应 prosemirror-model/src/replace.ts Slice
#[derive(Debug, Clone)]
pub struct Slice {
    /// 切片的内容
    pub content: Fragment,
    /// 开头的开放深度
    pub open_start: usize,
    /// 结尾的开放深度
    pub open_end: usize,
}

impl Slice {
    pub fn new(content: Fragment, open_start: usize, open_end: usize) -> Self {
        Self { content, open_start, open_end }
    }

    /// 空切片。
    pub fn empty() -> Self {
        Self::new(Fragment::empty(), 0, 0)
    }

    /// 切片在文档中插入时净增加的大小。
    pub fn size(&self) -> usize {
        self.content.size.saturating_sub(self.open_start + self.open_end)
    }

    /// 相等比较。
    pub fn eq(&self, other: &Slice) -> bool {
        self.content.eq(&other.content)
            && self.open_start == other.open_start
            && self.open_end == other.open_end
    }

    /// 从片段创建最大开放深度的切片。
    pub fn max_open(fragment: &Fragment) -> Self {
        let mut open_start = 0;
        let mut n = fragment.first_child();
        while let Some(node) = n {
            if node.is_leaf() {
                break;
            }
            open_start += 1;
            n = node.content.first_child();
        }

        let mut open_end = 0;
        let mut n = fragment.last_child();
        while let Some(node) = n {
            if node.is_leaf() {
                break;
            }
            open_end += 1;
            n = node.content.last_child();
        }

        Self::new(fragment.clone(), open_start, open_end)
    }

    /// 在切片中的 pos 位置插入 fragment（内部用）。
    pub fn insert_at(&self, pos: usize, fragment: &Fragment) -> Option<Slice> {
        let content = insert_into(&self.content, pos + self.open_start, fragment, None)?;
        Some(Slice::new(content, self.open_start, self.open_end))
    }

    /// 移除切片中 from..to 范围的内容（内部用）。
    pub fn remove_between(&self, from: usize, to: usize) -> Result<Slice, ReplaceError> {
        let content =
            remove_range(&self.content, from + self.open_start, to + self.open_start)?;
        Ok(Slice::new(content, self.open_start, self.open_end))
    }
}

/// 核心 replace 算法：用 slice 替换 $from..$to 之间的内容。
pub fn replace(
    from: &ResolvedPos,
    to: &ResolvedPos,
    slice: &Slice,
) -> Result<Node, ReplaceError> {
    if slice.open_start > from.depth() {
        return Err(ReplaceError("Inserted content deeper than insertion position".into()));
    }
    if from.depth() - slice.open_start != to.depth() - slice.open_end {
        return Err(ReplaceError("Inconsistent open depths".into()));
    }
    replace_outer(from, to, slice, 0)
}

fn replace_outer(
    from: &ResolvedPos,
    to: &ResolvedPos,
    slice: &Slice,
    depth: usize,
) -> Result<Node, ReplaceError> {
    let index = from.index(Some(depth as isize));
    let node = from.node(Some(depth as isize)).clone();

    if index == to.index(Some(depth as isize)) && depth < from.depth() - slice.open_start {
        let inner = replace_outer(from, to, slice, depth + 1)?;
        let new_content = node.content.replace_child(index, inner);
        return Ok(node.copy(new_content));
    }

    if slice.content.size == 0 {
        let content = replace_two_way(from, to, depth);
        return close(&node, content);
    }

    if slice.open_start == 0
        && slice.open_end == 0
        && from.depth() == depth
        && to.depth() == depth
    {
        let parent = from.parent().clone();
        let content = parent.content.clone();
        let new_content = content
            .cut(0, Some(from.parent_offset))
            .append(&slice.content)
            .append(&content.cut(to.parent_offset, None));
        return close(&parent, new_content);
    }

    let (start, end) = prepare_slice_for_replace(slice, from)?;
    let content = replace_three_way(from, &start, &end, to, depth)?;
    close(&node, content)
}

/// 修复 2.7：使用 NodeType::compatible_content 替代名称比较。
fn check_join(main: &Node, sub: &Node) -> Result<(), ReplaceError> {
    if !main.node_type.compatible_content(&sub.node_type) {
        return Err(ReplaceError(format!(
            "Cannot join {} onto {}",
            sub.node_type.name, main.node_type.name
        )));
    }
    Ok(())
}

fn joinable<'a>(
    before: &'a ResolvedPos,
    after: &'a ResolvedPos,
    depth: usize,
) -> Result<Node, ReplaceError> {
    let node = before.node(Some(depth as isize)).clone();
    check_join(&node, after.node(Some(depth as isize)))?;
    Ok(node)
}

fn add_node(child: Node, target: &mut Vec<Node>) {
    if let Some(last) = target.last_mut() {
        if child.is_text() && child.same_markup(last) {
            let merged_text = last.text().unwrap_or("").to_string()
                + child.text().unwrap_or("");
            *last = last.with_text(merged_text);
            return;
        }
    }
    target.push(child);
}

fn add_range(
    start: Option<&ResolvedPos>,
    end: Option<&ResolvedPos>,
    depth: usize,
    target: &mut Vec<Node>,
) {
    let node = end.or(start).unwrap().node(Some(depth as isize));
    let start_index = if let Some(s) = start {
        let mut idx = s.index(Some(depth as isize));
        if s.depth() > depth {
            idx += 1;
        } else if s.text_offset() > 0 {
            if let Some(n) = s.node_after() {
                add_node(n, target);
            }
            idx += 1;
        }
        idx
    } else {
        0
    };
    let end_index = if let Some(e) = end {
        e.index(Some(depth as isize))
    } else {
        node.content.child_count()
    };

    for i in start_index..end_index {
        add_node(node.content.child(i).clone(), target);
    }

    if let Some(e) = end {
        if e.depth() == depth && e.text_offset() > 0 {
            if let Some(n) = e.node_before() {
                add_node(n, target);
            }
        }
    }
}

/// 修复 2.7：调用 node_type.check_content 验证内容合法性。
fn close(node: &Node, content: Fragment) -> Result<Node, ReplaceError> {
    node.node_type
        .check_content(&content)
        .map_err(|e| ReplaceError(e))?;
    Ok(node.copy(content))
}

fn replace_three_way(
    from: &ResolvedPos,
    start: &ResolvedPos,
    end: &ResolvedPos,
    to: &ResolvedPos,
    depth: usize,
) -> Result<Fragment, ReplaceError> {
    let open_start = if from.depth() > depth {
        Some(joinable(from, start, depth + 1)?)
    } else {
        None
    };
    let open_end = if to.depth() > depth {
        Some(joinable(end, to, depth + 1)?)
    } else {
        None
    };

    let mut content = Vec::new();
    add_range(None, Some(from), depth, &mut content);

    let has_open_start = open_start.is_some();
    let has_open_end_same_index = open_end.is_some()
        && start.index(Some(depth as isize)) == end.index(Some(depth as isize));

    if has_open_start && has_open_end_same_index {
        let os = open_start.as_ref().unwrap();
        let oe = open_end.as_ref().unwrap();
        check_join(os, oe)?;
        let inner = replace_three_way(from, start, end, to, depth + 1)?;
        add_node(close(os, inner)?, &mut content);
    } else if has_open_start {
        let os = open_start.as_ref().unwrap();
        let inner = replace_two_way(from, start, depth + 1);
        add_node(close(os, inner)?, &mut content);
        add_range(Some(start), Some(end), depth, &mut content);
        if let Some(ref oe) = open_end {
            let inner = replace_two_way(end, to, depth + 1);
            add_node(close(oe, inner)?, &mut content);
        }
    } else {
        add_range(Some(start), Some(end), depth, &mut content);
        if let Some(ref oe) = open_end {
            let inner = replace_two_way(end, to, depth + 1);
            add_node(close(oe, inner)?, &mut content);
        }
    }

    add_range(Some(to), None, depth, &mut content);
    Ok(Fragment::from_array(content))
}

fn replace_two_way(from: &ResolvedPos, to: &ResolvedPos, depth: usize) -> Fragment {
    let mut content = Vec::new();
    add_range(None, Some(from), depth, &mut content);
    if from.depth() > depth {
        if let Ok(node) = joinable(from, to, depth + 1) {
            let inner = replace_two_way(from, to, depth + 1);
            if let Ok(closed) = close(&node, inner) {
                add_node(closed, &mut content);
            }
        }
    }
    add_range(Some(to), None, depth, &mut content);
    Fragment::from_array(content)
}

fn prepare_slice_for_replace(
    slice: &Slice,
    along: &ResolvedPos,
) -> Result<(ResolvedPos, ResolvedPos), ReplaceError> {
    let extra = along.depth() - slice.open_start;
    let parent = along.node(Some(extra as isize)).clone();
    let mut node = parent.copy(slice.content.clone());
    for i in (0..extra).rev() {
        let wrapper = along.node(Some(i as isize)).clone();
        node = wrapper.copy(Fragment::from_array(vec![node]));
    }
    let start_pos = slice.open_start + extra;
    let end_pos = node.content.size - slice.open_end - extra;
    let start = ResolvedPos::resolve(&node, start_pos).map_err(ReplaceError)?;
    let end = ResolvedPos::resolve(&node, end_pos).map_err(ReplaceError)?;
    Ok((start, end))
}

/// 从内容中移除 from..to 范围。
///
/// 修复 2.7：非平坦范围返回 Err 而非静默返回原内容。
fn remove_range(content: &Fragment, from: usize, to: usize) -> Result<Fragment, ReplaceError> {
    let (index, offset) = content.find_index(from);
    let child = content.maybe_child(index);
    let (index_to, _offset_to) = content.find_index(to);

    if offset == from || child.map(|c| c.is_text()).unwrap_or(false) {
        return Ok(content.cut(0, Some(from)).append(&content.cut(to, None)));
    }

    if index != index_to {
        return Err(ReplaceError("Removing non-flat range".into()));
    }

    if let Some(child) = child {
        let inner = remove_range(&child.content, from - offset - 1, to - offset - 1)?;
        return Ok(content.replace_child(index, child.copy(inner)));
    }

    Ok(content.clone())
}

/// 在 content 的 dist 位置插入 insert。
///
/// 修复 2.7：当 parent 存在时调用 can_replace 检查。
fn insert_into(
    content: &Fragment,
    dist: usize,
    insert: &Fragment,
    parent: Option<&Node>,
) -> Option<Fragment> {
    let (index, offset) = content.find_index(dist);
    let child = content.maybe_child(index);

    if offset == dist || child.map(|c| c.is_text()).unwrap_or(false) {
        if let Some(p) = parent {
            if !p.can_replace(index, index, insert, 0, insert.child_count()) {
                return None;
            }
        }
        return Some(
            content
                .cut(0, Some(dist))
                .append(insert)
                .append(&content.cut(dist, None)),
        );
    }

    if let Some(child) = child {
        let inner = insert_into(&child.content, dist - offset - 1, insert, Some(child))?;
        return Some(content.replace_child(index, child.copy(inner)));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::schema::{NodeType, MarkType};
    use super::super::node::Node;
    use super::super::fragment::Fragment;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    fn block_nt(name: &str) -> Arc<NodeType> {
        Arc::new(NodeType {
            name: name.into(), groups: vec![], is_block: true,
            is_text: false, inline_content: false, mark_set: None, content_match: None,
        })
    }

    fn text_nt() -> Arc<NodeType> {
        Arc::new(NodeType {
            name: "text".into(), groups: vec![], is_block: false,
            is_text: true, inline_content: false, mark_set: None, content_match: None,
        })
    }

    fn text(s: &str) -> Node {
        Node {
            node_type: text_nt(), attrs: BTreeMap::new(),
            content: Fragment::empty(), marks: vec![], text: Some(s.into()),
        }
    }

    fn block(nt: Arc<NodeType>, children: Vec<Node>) -> Node {
        Node {
            node_type: nt, attrs: BTreeMap::new(),
            content: Fragment::from_array(children),
            marks: vec![], text: None,
        }
    }

    // ── Slice ────────────────────────────────────────────────

    #[test]
    fn slice_empty() {
        let s = Slice::empty();
        assert_eq!(s.open_start, 0);
        assert_eq!(s.open_end, 0);
        assert_eq!(s.content.size, 0);
    }

    #[test]
    fn slice_max_open_leaf() {
        let txt = text("hello");
        let f = Fragment::from_array(vec![txt]);
        let s = Slice::max_open(&f);
        assert_eq!(s.open_start, 0);
        assert_eq!(s.open_end, 0);
    }

    #[test]
    fn slice_max_open_block() {
        use super::super::content::ContentMatch;
        use std::collections::HashMap;

        // inner 用 content_match=None（leaf），outer 用真正的 non-leaf ContentMatch
        let inner_nt = block_nt("inner");
        let mut types: HashMap<String, Arc<NodeType>> = HashMap::new();
        types.insert("inner".into(), Arc::clone(&inner_nt));
        let cm = ContentMatch::parse("inner*", &types).unwrap();
        let outer_nt = Arc::new(NodeType {
            name: "outer".into(), groups: vec![], is_block: true,
            is_text: false, inline_content: false, mark_set: None,
            content_match: Some(cm),
        });

        let inner = block(Arc::clone(&inner_nt), vec![]);
        let outer = Node {
            node_type: Arc::clone(&outer_nt),
            attrs: std::collections::BTreeMap::new(),
            content: Fragment::from_array(vec![inner]),
            marks: vec![], text: None,
        };
        let f = Fragment::from_array(vec![outer]);
        let s = Slice::max_open(&f);
        // outer is non-leaf, inner is leaf → open depth = 1
        assert_eq!(s.open_start, 1);
        assert_eq!(s.open_end, 1);
    }

    // ── remove_range ────────────────────────────────────────

    #[test]
    fn remove_range_flat() {
        // Content: [text "hello"] (size=5)
        // Remove 1..3 → flat case (text node)
        let f = Fragment::from_array(vec![text("hello")]);
        let result = remove_range(&f, 1, 3);
        assert!(result.is_ok());
        let r = result.unwrap();
        // "h" + "lo" = "hlo" (merged as same type)
        assert_eq!(r.child(0).text(), Some("hlo"));
    }

    #[test]
    fn remove_range_non_flat_returns_err() {
        // Content: [p1(nodeSize=4), p2(nodeSize=4)]
        // from=1 (inside p1), to=5 (inside p2) → non-flat
        let p1 = block(block_nt("p"), vec![text("ab")]);
        let p2 = block(block_nt("p"), vec![text("cd")]);
        let f = Fragment::from_array(vec![p1, p2]);
        // p1.nodeSize = 2+2 = 4, p2.nodeSize = 4
        // from=1 (inside p1 at content start), to=5 (inside p2)
        let result = remove_range(&f, 1, 5);
        assert!(result.is_err());
        assert!(result.unwrap_err().0.contains("non-flat"));
    }

    // ── check_join via compatible_content ────────────────────

    #[test]
    fn check_join_same_type_passes() {
        let nt = block_nt("p");
        let n1 = block(Arc::clone(&nt), vec![]);
        let n2 = block(Arc::clone(&nt), vec![]);
        // Same Arc instance → ptr_eq passes
        assert!(check_join(&n1, &n2).is_ok());
    }

    #[test]
    fn check_join_different_types_fails() {
        let n1 = block(block_nt("p"), vec![]);
        let n2 = block(block_nt("div"), vec![]);
        assert!(check_join(&n1, &n2).is_err());
    }
}
