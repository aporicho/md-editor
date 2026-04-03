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
    pub fn remove_between(&self, from: usize, to: usize) -> Slice {
        Slice::new(
            remove_range(&self.content, from + self.open_start, to + self.open_start),
            self.open_start,
            self.open_end,
        )
    }
}

/// 核心 replace 算法：用 slice 替换 $from..$to 之间的内容。
///
/// 对应 prosemirror-model/src/replace.ts replace()
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

fn check_join(main: &Node, sub: &Node) -> Result<(), ReplaceError> {
    if !sub.node_type.name.eq(&main.node_type.name) {
        // TODO: 等 NodeType.compatible_content 移植完后用正确检查
        // 暂时只检查类型名是否一致
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

fn close(node: &Node, content: Fragment) -> Result<Node, ReplaceError> {
    // TODO: node.type.check_content(content) — 等 Schema 移植后补全
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
    let start = ResolvedPos::resolve(&node, start_pos)
        .map_err(|e| ReplaceError(e))?;
    let end = ResolvedPos::resolve(&node, end_pos)
        .map_err(|e| ReplaceError(e))?;
    Ok((start, end))
}

/// 从内容中移除 from..to 范围。
fn remove_range(content: &Fragment, from: usize, to: usize) -> Fragment {
    let (index, offset) = content.find_index(from);
    let child = content.maybe_child(index);
    let (index_to, offset_to) = content.find_index(to);

    if offset == from || child.map(|c| c.is_text()).unwrap_or(false) {
        // 平坦情况
        return content
            .cut(0, Some(from))
            .append(&content.cut(to, None));
    }

    if index != index_to {
        // 非平坦范围，不应发生
        return content.clone();
    }

    if let Some(child) = child {
        let inner = remove_range(
            &child.content,
            from - offset - 1,
            to - offset - 1,
        );
        return content.replace_child(index, child.copy(inner));
    }

    content.clone()
}

/// 在 content 的 dist 位置插入 insert。
fn insert_into(
    content: &Fragment,
    dist: usize,
    insert: &Fragment,
    _parent: Option<&Node>,
) -> Option<Fragment> {
    let (index, offset) = content.find_index(dist);
    let child = content.maybe_child(index);

    if offset == dist || child.map(|c| c.is_text()).unwrap_or(false) {
        // TODO: parent.can_replace 检查等 Node 移植完后补全
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
