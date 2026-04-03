use std::sync::Arc;

// Node 在 node.rs 中定义，此处前向声明避免循环依赖
// Fragment 和 Node 互相引用，通过 Arc 解决
use super::node::Node;

/// 节点子节点的不可变有序序列。
/// 所有节点都持有一个片段（无子节点时为空片段）。
/// 修改总是返回新实例。
///
/// 对应 prosemirror-model/src/fragment.ts
#[derive(Debug, Clone)]
pub struct Fragment {
    /// 子节点列表
    pub content: Arc<Vec<Node>>,
    /// 所有子节点的 node_size 之和
    pub size: usize,
}

impl Fragment {
    /// 从子节点列表和预计算的 size 构造片段（内部用）。
    fn new(content: Vec<Node>, size: usize) -> Self {
        Self {
            content: Arc::new(content),
            size,
        }
    }

    /// 空片段单例。
    pub fn empty() -> Self {
        Self::new(vec![], 0)
    }

    /// 从节点数组创建片段，自动合并相邻的同标记文本节点。
    pub fn from_array(array: Vec<Node>) -> Self {
        if array.is_empty() {
            return Self::empty();
        }
        let mut joined: Option<Vec<Node>> = None;
        let mut size = 0usize;

        for (i, node) in array.iter().enumerate() {
            size += node.node_size();
            if i > 0 && node.is_text() {
                let prev = joined.as_ref().map(|j| &j[j.len() - 1]).unwrap_or(&array[i - 1]);
                if prev.same_markup(node) {
                    if joined.is_none() {
                        joined = Some(array[..i].to_vec());
                    }
                    let j = joined.as_mut().unwrap();
                    let last = j.last_mut().unwrap();
                    *last = last.with_text(
                        last.text().unwrap_or("").to_string()
                            + node.text().unwrap_or(""),
                    );
                    continue;
                }
            }
            if let Some(ref mut j) = joined {
                j.push(node.clone());
            }
        }

        let content = joined.unwrap_or(array);
        Self::new(content, size)
    }

    /// 从单个节点、节点数组或片段创建片段。
    pub fn from(nodes: FragmentInput) -> Self {
        match nodes {
            FragmentInput::None => Self::empty(),
            FragmentInput::Fragment(f) => f,
            FragmentInput::Node(n) => {
                let size = n.node_size();
                Self::new(vec![n], size)
            }
            FragmentInput::Nodes(ns) => Self::from_array(ns),
        }
    }

    /// 子节点数量。
    pub fn child_count(&self) -> usize {
        self.content.len()
    }

    /// 取第 index 个子节点，越界 panic。
    pub fn child(&self, index: usize) -> &Node {
        &self.content[index]
    }

    /// 取第 index 个子节点，越界返回 None。
    pub fn maybe_child(&self, index: usize) -> Option<&Node> {
        self.content.get(index)
    }

    /// 第一个子节点。
    pub fn first_child(&self) -> Option<&Node> {
        self.content.first()
    }

    /// 最后一个子节点。
    pub fn last_child(&self) -> Option<&Node> {
        self.content.last()
    }

    /// 遍历所有子节点，回调参数：(node, offset, index)。
    pub fn for_each<F: FnMut(&Node, usize, usize)>(&self, mut f: F) {
        let mut pos = 0;
        for (i, node) in self.content.iter().enumerate() {
            f(node, pos, i);
            pos += node.node_size();
        }
    }

    /// 遍历 from..to 范围内的所有后代节点。
    /// 回调返回 false 时跳过该节点的子节点。
    pub fn nodes_between<F>(
        &self,
        from: usize,
        to: usize,
        f: &mut F,
        node_start: usize,
        parent: Option<&Node>,
    ) where
        F: FnMut(&Node, usize, Option<&Node>, usize) -> bool,
    {
        let mut pos = 0usize;
        for (i, child) in self.content.iter().enumerate() {
            let end = pos + child.node_size();
            if end > from {
                if f(child, node_start + pos, parent, i) && child.content.size > 0 {
                    let start = pos + 1;
                    child.content.nodes_between(
                        from.saturating_sub(start),
                        (child.content.size).min(to.saturating_sub(start)),
                        f,
                        node_start + start,
                        Some(child),
                    );
                }
            }
            pos = end;
            if pos >= to {
                break;
            }
        }
    }

    /// 拼接两个片段，返回新片段。相邻的同标记文本节点自动合并。
    pub fn append(&self, other: &Fragment) -> Fragment {
        if other.size == 0 {
            return self.clone();
        }
        if self.size == 0 {
            return other.clone();
        }

        let last = self.content.last().unwrap();
        let first = other.content.first().unwrap();

        let mut content: Vec<Node> = (*self.content).clone();
        let mut i = 0;

        // 尝试合并末尾和开头的文本节点
        if last.is_text() && last.same_markup(first) {
            let merged = last.with_text(
                last.text().unwrap_or("").to_string() + first.text().unwrap_or(""),
            );
            *content.last_mut().unwrap() = merged;
            i = 1;
        }

        for node in other.content.iter().skip(i) {
            content.push(node.clone());
        }

        Fragment::new(content, self.size + other.size)
    }

    /// 按位置截取子片段 [from, to)。to 为 None 时截到末尾。
    pub fn cut(&self, from: usize, to: Option<usize>) -> Fragment {
        let to = to.unwrap_or(self.size);
        if from == 0 && to == self.size {
            return self.clone();
        }
        let mut result = Vec::new();
        let mut size = 0usize;

        if to > from {
            let mut pos = 0usize;
            for child in self.content.iter() {
                let end = pos + child.node_size();
                if end > from {
                    let node = if pos < from || end > to {
                        if child.is_text() {
                            child.cut(
                                from.saturating_sub(pos),
                                child.text().unwrap_or("").len().min(to - pos),
                            )
                        } else {
                            child.cut(
                                from.saturating_sub(pos).saturating_sub(1),
                                child.content.size.min(to.saturating_sub(pos + 1)),
                            )
                        }
                    } else {
                        child.clone()
                    };
                    size += node.node_size();
                    result.push(node);
                }
                pos = end;
                if pos >= to {
                    break;
                }
            }
        }

        Fragment::new(result, size)
    }

    /// 按子节点索引截取 [from, to)。
    pub fn cut_by_index(&self, from: usize, to: usize) -> Fragment {
        if from == to {
            return Fragment::empty();
        }
        if from == 0 && to == self.content.len() {
            return self.clone();
        }
        Fragment::from_array(self.content[from..to].to_vec())
    }

    /// 替换第 index 个子节点，返回新片段。
    pub fn replace_child(&self, index: usize, node: Node) -> Fragment {
        let current = &self.content[index];
        if std::ptr::eq(current, &node) {
            return self.clone();
        }
        let size = self.size + node.node_size() - current.node_size();
        let mut content = (*self.content).clone();
        content[index] = node;
        Fragment::new(content, size)
    }

    /// 在开头插入节点，返回新片段。
    pub fn add_to_start(&self, node: Node) -> Fragment {
        let size = self.size + node.node_size();
        let mut content = vec![node];
        content.extend_from_slice(&self.content);
        Fragment::new(content, size)
    }

    /// 在末尾插入节点，返回新片段。
    pub fn add_to_end(&self, node: Node) -> Fragment {
        let size = self.size + node.node_size();
        let mut content = (*self.content).clone();
        content.push(node);
        Fragment::new(content, size)
    }

    /// 结构相等比较。
    pub fn eq(&self, other: &Fragment) -> bool {
        if self.content.len() != other.content.len() {
            return false;
        }
        self.content.iter().zip(other.content.iter()).all(|(a, b)| a.eq(b))
    }

    /// 按位置找到对应的子节点索引和偏移。
    pub fn find_index(&self, pos: usize) -> (usize, usize) {
        if pos == 0 {
            return (0, 0);
        }
        if pos == self.size {
            return (self.content.len(), pos);
        }
        let mut cur_pos = 0usize;
        for (i, child) in self.content.iter().enumerate() {
            let end = cur_pos + child.node_size();
            if end >= pos {
                if end == pos {
                    return (i + 1, end);
                }
                return (i, cur_pos);
            }
            cur_pos = end;
        }
        panic!("Position {} outside of fragment (size {})", pos, self.size);
    }

    /// 找到两个片段第一个不同的位置。
    pub fn find_diff_start(&self, other: &Fragment, pos: usize) -> Option<usize> {
        crate::model::diff::find_diff_start(self, other, pos)
    }

    /// 找到两个片段最后一个不同的位置。
    pub fn find_diff_end(
        &self,
        other: &Fragment,
        pos: usize,
        other_pos: usize,
    ) -> Option<(usize, usize)> {
        crate::model::diff::find_diff_end(self, other, pos, other_pos)
    }
}

/// Fragment::from 的输入类型。
pub enum FragmentInput {
    None,
    Fragment(Fragment),
    Node(Node),
    Nodes(Vec<Node>),
}
