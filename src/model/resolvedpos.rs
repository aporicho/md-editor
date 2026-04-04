use super::mark::Mark;
use super::node::Node;

/// 路径条目：(节点, 在父节点中的索引, 绝对起始位置)
#[derive(Debug, Clone)]
struct PathEntry {
    node: Node,
    index: usize,
    start: usize,
}

/// 已解析的文档位置。持有从根到目标位置的完整祖先链。
///
/// 对应 prosemirror-model/src/resolvedpos.ts ResolvedPos
#[derive(Debug, Clone)]
pub struct ResolvedPos {
    /// 原始绝对位置
    pub pos: usize,
    /// 祖先链（从 doc 到直接父节点）
    path: Vec<PathEntry>,
    /// 在直接父节点内的偏移
    pub parent_offset: usize,
}

impl ResolvedPos {
    /// 解析文档中的位置，返回 ResolvedPos。
    pub fn resolve(doc: &Node, pos: usize) -> Result<Self, String> {
        if pos > doc.content.size {
            return Err(format!("Position {} out of range", pos));
        }

        let mut path = Vec::new();
        let mut start = 0usize;
        let mut parent_offset = pos;
        let mut node = doc.clone();

        loop {
            let (index, offset) = node.content.find_index(parent_offset);
            let rem = parent_offset - offset;
            path.push(PathEntry {
                node: node.clone(),
                index,
                start: start + offset,
            });
            if rem == 0 {
                break;
            }
            let child = node.content.child(index).clone();
            if child.is_text() {
                break;
            }
            parent_offset = rem - 1;
            start += offset + 1;
            node = child;
        }

        Ok(ResolvedPos {
            pos,
            path,
            parent_offset,
        })
    }

    /// 嵌套深度（doc = 0）。
    pub fn depth(&self) -> usize {
        self.path.len() - 1
    }

    fn resolve_depth(&self, depth: Option<isize>) -> usize {
        match depth {
            None => self.depth(),
            Some(d) if d < 0 => {
                // 修复 1.4：负数深度越界时 panic 而非 wrapping
                let result = self.depth() as isize + d;
                assert!(
                    result >= 0,
                    "Depth {} out of range (actual depth: {})",
                    d,
                    self.depth()
                );
                result as usize
            }
            Some(d) => d as usize,
        }
    }

    /// 指定深度的祖先节点。
    pub fn node(&self, depth: Option<isize>) -> &Node {
        &self.path[self.resolve_depth(depth)].node
    }

    /// 直接父节点。
    pub fn parent(&self) -> &Node {
        self.node(None)
    }

    /// 根节点（doc）。
    pub fn doc(&self) -> &Node {
        self.node(Some(0))
    }

    /// 在指定深度祖先节点中的子节点索引。
    pub fn index(&self, depth: Option<isize>) -> usize {
        self.path[self.resolve_depth(depth)].index
    }

    /// 指向此位置之后的子节点索引。
    pub fn index_after(&self, depth: Option<isize>) -> usize {
        let d = self.resolve_depth(depth);
        let idx = self.index(Some(d as isize));
        if d == self.depth() && self.text_offset() == 0 {
            idx
        } else {
            idx + 1
        }
    }

    /// 指定深度节点的绝对起始位置。
    pub fn start(&self, depth: Option<isize>) -> usize {
        let d = self.resolve_depth(depth);
        if d == 0 {
            0
        } else {
            self.path[d - 1].start + 1
        }
    }

    /// 指定深度节点的绝对结束位置。
    pub fn end(&self, depth: Option<isize>) -> usize {
        let d = self.resolve_depth(depth);
        self.start(Some(d as isize)) + self.node(Some(d as isize)).content.size
    }

    /// 指定深度节点之前的位置。
    pub fn before(&self, depth: Option<isize>) -> Result<usize, String> {
        let d = self.resolve_depth(depth);
        if d == 0 {
            return Err("There is no position before the top-level node".into());
        }
        if d == self.depth() + 1 {
            Ok(self.pos)
        } else {
            Ok(self.path[d - 1].start)
        }
    }

    /// 指定深度节点之后的位置。
    ///
    /// 修复 1.2：使用 path[d].node（第 d 层节点本体）而非 path[d-1]（父节点）。
    pub fn after(&self, depth: Option<isize>) -> Result<usize, String> {
        let d = self.resolve_depth(depth);
        if d == 0 {
            return Err("There is no position after the top-level node".into());
        }
        if d == self.depth() + 1 {
            Ok(self.pos)
        } else {
            // path[d-1].start = 第 d 层节点在文档中的绝对起始位置
            // path[d].node    = 第 d 层节点本体，其 node_size 包含开闭标签
            Ok(self.path[d - 1].start + self.path[d].node.node_size())
        }
    }

    /// 在文本节点内部时的字符偏移，否则为 0。
    pub fn text_offset(&self) -> usize {
        let last = self.path.last().unwrap();
        self.pos - last.start
    }

    /// 当前位置之后紧邻的节点（如果有）。
    pub fn node_after(&self) -> Option<Node> {
        let parent = self.parent();
        let index = self.index(None);
        if index == parent.content.child_count() {
            return None;
        }
        let d_off = self.text_offset();
        let child = parent.content.child(index).clone();
        if d_off > 0 {
            Some(child.cut(d_off, child.node_size()))
        } else {
            Some(child)
        }
    }

    /// 当前位置之前紧邻的节点（如果有）。
    pub fn node_before(&self) -> Option<Node> {
        let index = self.index(None);
        let d_off = self.text_offset();
        if d_off > 0 {
            Some(self.parent().content.child(index).cut(0, d_off))
        } else if index == 0 {
            None
        } else {
            Some(self.parent().content.child(index - 1).clone())
        }
    }

    /// 在父节点中指定索引处的绝对位置。
    pub fn pos_at_index(&self, index: usize, depth: Option<isize>) -> usize {
        let d = self.resolve_depth(depth);
        let node = &self.path[d].node;
        let mut pos = if d == 0 { 0 } else { self.path[d - 1].start + 1 };
        for i in 0..index {
            pos += node.content.child(i).node_size();
        }
        pos
    }

    /// 当前位置生效的标记集（含 inclusive 过滤）。
    pub fn marks(&self) -> Vec<Mark> {
        let parent = self.parent();
        let index = self.index(None);

        if parent.content.size == 0 {
            return Mark::none();
        }

        if self.text_offset() > 0 {
            return parent.content.child(index).marks.clone();
        }

        let main = if index > 0 {
            parent.content.maybe_child(index - 1)
        } else {
            None
        };
        let other = parent.content.maybe_child(index);

        let (main, other) = if main.is_none() {
            (other, main)
        } else {
            (main, other)
        };

        let mut result = match main {
            Some(n) => n.marks.clone(),
            None => return Mark::none(),
        };

        // 修复 2.6：过滤 inclusive=false 且在 other 一侧不存在的标记
        let other_marks = other.map(|o| o.marks.as_slice());
        let mut i = result.len();
        while i > 0 {
            i -= 1;
            if result[i].mark_type.inclusive == Some(false) {
                if other_marks.map_or(true, |om| !result[i].is_in_set(om)) {
                    result = result[i].remove_from_set(&result);
                }
            }
        }

        result
    }

    /// 跨越两个位置之间的标记集（行内节点适用）。
    pub fn marks_across(&self, end: &ResolvedPos) -> Option<Vec<Mark>> {
        let after = self.parent().content.maybe_child(self.index(None))?;
        if !after.is_inline() {
            return None;
        }
        let mut marks = after.marks.clone();
        let next = end.parent().content.maybe_child(end.index(None));
        let mut i = marks.len();
        while i > 0 {
            i -= 1;
            if marks[i].mark_type.inclusive == Some(false) {
                if next.map_or(true, |n| !marks[i].is_in_set(&n.marks)) {
                    marks = marks[i].remove_from_set(&marks);
                }
            }
        }
        Some(marks)
    }

    /// 与另一个位置最近的公共祖先深度。
    pub fn shared_depth(&self, pos: usize) -> usize {
        for depth in (1..=self.depth()).rev() {
            if self.start(Some(depth as isize)) <= pos && self.end(Some(depth as isize)) >= pos {
                return depth;
            }
        }
        0
    }

    /// 是否与另一个位置有相同的父节点。
    pub fn same_parent(&self, other: &ResolvedPos) -> bool {
        self.pos - self.parent_offset == other.pos - other.parent_offset
    }

    /// 返回两个位置中较大的。
    pub fn max<'a>(&'a self, other: &'a ResolvedPos) -> &'a ResolvedPos {
        if other.pos > self.pos { other } else { self }
    }

    /// 返回两个位置中较小的。
    pub fn min<'a>(&'a self, other: &'a ResolvedPos) -> &'a ResolvedPos {
        if other.pos < self.pos { other } else { self }
    }

    /// 块级范围：找到包含 self 和 other 的最小公共块节点范围。
    ///
    /// 修复 1.3：使用 node_type.inline_content 替代 content.size == 0 判断。
    pub fn block_range(&self, other: &ResolvedPos) -> Option<NodeRange> {
        if other.pos < self.pos {
            return other.block_range(self);
        }

        // 父节点是 textblock（包含行内内容）或位置相同，从 depth-1 开始搜索
        let start_depth = if self.parent().node_type.inline_content || self.pos == other.pos {
            self.depth().saturating_sub(1)
        } else {
            self.depth()
        };

        for d in (0..=start_depth).rev() {
            if other.pos <= self.end(Some(d as isize)) {
                return Some(NodeRange {
                    from: self.clone(),
                    to: other.clone(),
                    depth: d,
                });
            }
        }
        None
    }
}

/// 文档中一段扁平的块级范围。
#[derive(Debug, Clone)]
pub struct NodeRange {
    pub from: ResolvedPos,
    pub to: ResolvedPos,
    pub depth: usize,
}

impl NodeRange {
    /// 范围的起始位置。
    pub fn start(&self) -> usize {
        self.from.before(Some(self.depth as isize + 1)).unwrap_or(0)
    }

    /// 范围的结束位置。
    pub fn end(&self) -> usize {
        self.to.after(Some(self.depth as isize + 1)).unwrap_or(0)
    }

    /// 父节点。
    pub fn parent(&self) -> &Node {
        self.from.node(Some(self.depth as isize))
    }

    /// 范围在父节点中的起始索引。
    pub fn start_index(&self) -> usize {
        self.from.index(Some(self.depth as isize))
    }

    /// 范围在父节点中的结束索引。
    pub fn end_index(&self) -> usize {
        self.to.index_after(Some(self.depth as isize))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::schema::{NodeType, MarkType};
    use super::super::fragment::Fragment;
    use super::super::node::Node;
    use super::super::mark::Mark;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    fn nt(name: &str, is_block: bool, inline_content: bool) -> Arc<NodeType> {
        Arc::new(NodeType {
            name: name.into(), groups: vec![], is_block,
            is_text: name == "text", inline_content, mark_set: None, content_match: None,
        })
    }

    fn text_node(s: &str) -> Node {
        Node {
            node_type: nt("text", false, false),
            attrs: BTreeMap::new(), content: Fragment::empty(),
            marks: vec![], text: Some(s.into()),
        }
    }

    fn text_with_marks(s: &str, marks: Vec<Mark>) -> Node {
        Node {
            node_type: nt("text", false, false),
            attrs: BTreeMap::new(), content: Fragment::empty(),
            marks, text: Some(s.into()),
        }
    }

    /// doc → paragraph("hello") の簡単な文書構造を作る。
    /// paragraph は textblock（inline_content=true）。
    fn simple_doc() -> Node {
        let txt = text_node("hello");
        let para = Node {
            node_type: nt("paragraph", true, true),
            attrs: BTreeMap::new(),
            content: Fragment::from_array(vec![txt]),
            marks: vec![], text: None,
        };
        Node {
            node_type: nt("doc", true, false),
            attrs: BTreeMap::new(),
            content: Fragment::from_array(vec![para]),
            marks: vec![], text: None,
        }
    }

    // ── resolve / depth / node ───────────────────────────────

    #[test]
    fn resolve_root_pos() {
        let doc = simple_doc();
        let rp = ResolvedPos::resolve(&doc, 0).unwrap();
        assert_eq!(rp.depth(), 0);
        assert_eq!(rp.parent().node_type.name, "doc");
    }

    #[test]
    fn resolve_inside_paragraph() {
        let doc = simple_doc();
        // pos=1 is inside paragraph (start of content)
        let rp = ResolvedPos::resolve(&doc, 1).unwrap();
        assert_eq!(rp.depth(), 1);
        assert_eq!(rp.parent().node_type.name, "paragraph");
    }

    #[test]
    fn resolve_inside_text() {
        let doc = simple_doc();
        // pos=3 is inside "hello" at char offset 2
        let rp = ResolvedPos::resolve(&doc, 3).unwrap();
        assert_eq!(rp.depth(), 1);
        assert_eq!(rp.text_offset(), 2);
    }

    // ── after() fix 1.2 ─────────────────────────────────────

    #[test]
    fn after_returns_correct_position() {
        let doc = simple_doc();
        // paragraph: content.size=5, nodeSize=7
        // after(depth=1) should be 0 + 7 = 7
        let rp = ResolvedPos::resolve(&doc, 3).unwrap();
        let result = rp.after(Some(1)).unwrap();
        assert_eq!(result, 7, "after(1) should equal paragraph.nodeSize = 7");
    }

    #[test]
    fn before_returns_correct_position() {
        let doc = simple_doc();
        let rp = ResolvedPos::resolve(&doc, 3).unwrap();
        let result = rp.before(Some(1)).unwrap();
        assert_eq!(result, 0, "before(1) should be 0 (start of doc content)");
    }

    #[test]
    fn start_end_consistent_with_before_after() {
        let doc = simple_doc();
        let rp = ResolvedPos::resolve(&doc, 3).unwrap();
        assert_eq!(rp.start(Some(1)), 1); // inside paragraph: 0+1=1
        assert_eq!(rp.end(Some(1)), 6);   // 1 + content.size(5) = 6
    }

    // ── resolve_depth negative (fix 1.4) ────────────────────

    #[test]
    fn negative_depth_resolves_correctly() {
        let doc = simple_doc();
        let rp = ResolvedPos::resolve(&doc, 3).unwrap();
        // depth=1, node(Some(-1)) = node at depth 1+(-1)=0 = doc
        assert_eq!(rp.node(Some(-1)).node_type.name, "doc");
    }

    #[test]
    #[should_panic]
    fn negative_depth_out_of_range_panics() {
        let doc = simple_doc();
        let rp = ResolvedPos::resolve(&doc, 3).unwrap();
        // depth=1, Some(-2) → 1+(-2)=-1 → should panic
        rp.node(Some(-2));
    }

    // ── block_range (fix 1.3) ────────────────────────────────

    #[test]
    fn block_range_inside_textblock() {
        let doc = simple_doc();
        // Two positions inside the same textblock paragraph
        let rp1 = ResolvedPos::resolve(&doc, 2).unwrap();
        let rp2 = ResolvedPos::resolve(&doc, 4).unwrap();
        // paragraph is textblock → start_depth = depth-1 = 0
        // block_range should return depth=0 (doc level)
        let range = rp1.block_range(&rp2);
        assert!(range.is_some());
        let range = range.unwrap();
        assert_eq!(range.depth, 0, "should be at doc depth since parent is textblock");
    }

    // ── marks (fix 2.6) ─────────────────────────────────────

    #[test]
    fn marks_at_text_offset() {
        let bold_mt = Arc::new(MarkType {
            name: "bold".into(), rank: 0, excluded: vec![], inclusive: None,
        });
        let bold = Mark { mark_type: Arc::clone(&bold_mt), attrs: BTreeMap::new() };
        let txt = text_with_marks("hello", vec![bold.clone()]);
        let para = Node {
            node_type: nt("paragraph", true, true),
            attrs: BTreeMap::new(),
            content: Fragment::from_array(vec![txt]),
            marks: vec![], text: None,
        };
        let doc = Node {
            node_type: nt("doc", true, false),
            attrs: BTreeMap::new(),
            content: Fragment::from_array(vec![para]),
            marks: vec![], text: None,
        };
        // pos=3 is inside "hello" with bold mark
        let rp = ResolvedPos::resolve(&doc, 3).unwrap();
        let marks = rp.marks();
        assert_eq!(marks.len(), 1);
        assert_eq!(marks[0].mark_type.name, "bold");
    }

    #[test]
    fn marks_inclusive_false_filtered_at_boundary() {
        // Mark with inclusive=Some(false) should be filtered at boundary
        // when not present in the following node
        let exclusive_mt = Arc::new(MarkType {
            name: "link".into(), rank: 0, excluded: vec![], inclusive: Some(false),
        });
        let link = Mark { mark_type: Arc::clone(&exclusive_mt), attrs: BTreeMap::new() };
        // text "hello" with link, text " world" without
        let t1 = text_with_marks("hello", vec![link]);
        let t2 = text_node(" world");
        let para = Node {
            node_type: nt("paragraph", true, true),
            attrs: BTreeMap::new(),
            content: Fragment::from_array(vec![t1, t2]),
            marks: vec![], text: None,
        };
        let doc = Node {
            node_type: nt("doc", true, false),
            attrs: BTreeMap::new(),
            content: Fragment::from_array(vec![para]),
            marks: vec![], text: None,
        };
        // pos = 1+5 = 6 = boundary between "hello" and " world"
        let rp = ResolvedPos::resolve(&doc, 6).unwrap();
        let marks = rp.marks();
        // link has inclusive=false and doesn't appear in " world" → filtered out
        assert!(marks.is_empty(), "link mark should be filtered at boundary");
    }
}
