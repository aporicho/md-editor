use super::fragment::Fragment;
use super::mark::Mark;
use super::node::Node;

/// 路径条目：(节点, 在父节点中的索引, 绝对起始位置)
/// 对应 ProseMirror 中 path 数组的三元组结构。
#[derive(Debug, Clone)]
struct PathEntry {
    node: Node,
    index: usize,
    start: usize, // 该节点在文档中的绝对起始位置（节点开标签前的位置）
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
            Some(d) if d < 0 => (self.depth() as isize + d) as usize,
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
    pub fn after(&self, depth: Option<isize>) -> Result<usize, String> {
        let d = self.resolve_depth(depth);
        if d == 0 {
            return Err("There is no position after the top-level node".into());
        }
        if d == self.depth() + 1 {
            Ok(self.pos)
        } else {
            let entry = &self.path[d - 1];
            Ok(entry.start + entry.node.node_size())
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

    /// 当前位置生效的标记集。
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

        let mut marks = match main {
            Some(n) => n.marks.clone(),
            None => return Mark::none(),
        };

        // 移除 inclusive=false 且在 other 中不存在的标记
        // TODO: 等 MarkSpec 移植完后补充 inclusive 检查
        let _ = other;

        marks
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
    pub fn block_range(&self, other: &ResolvedPos) -> Option<NodeRange> {
        let other = if other.pos < self.pos {
            return other.block_range(self);
        } else {
            other
        };

        let start_depth = if self.parent().content.size == 0 || self.pos == other.pos {
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
