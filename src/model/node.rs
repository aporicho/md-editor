use std::sync::Arc;

use super::fragment::Fragment;
use super::mark::Mark;
use super::replace::Slice;
use super::resolvedpos::ResolvedPos;
use super::schema::NodeType;
use super::Attrs;

/// 文档树的基本单元，不可变。
///
/// 对应 prosemirror-model/src/node.ts
#[derive(Debug, Clone)]
pub struct Node {
    /// 节点类型
    pub node_type: Arc<NodeType>,
    /// 节点属性
    pub attrs: Attrs,
    /// 子节点片段
    pub content: Fragment,
    /// 标记（仅文本节点使用）
    pub marks: Vec<Mark>,
    /// 文本内容（仅文本节点）
    pub text: Option<String>,
}

impl Node {
    /// 节点在位置空间中占据的大小。
    pub fn node_size(&self) -> usize {
        if let Some(ref t) = self.text {
            t.chars().count()
        } else if self.content.size == 0 && self.node_type.is_leaf() {
            // 真正的原子叶节点：无子节点且类型声明为 leaf
            1
        } else {
            // 有子节点的容器节点，或非 leaf 类型的空节点
            self.content.size + 2
        }
    }

    pub fn is_text(&self) -> bool {
        self.node_type.is_text
    }

    pub fn is_block(&self) -> bool {
        self.node_type.is_block
    }

    pub fn is_inline(&self) -> bool {
        self.node_type.is_inline()
    }

    pub fn is_leaf(&self) -> bool {
        self.node_type.is_leaf()
    }

    /// 是否包含行内内容（委托到 NodeType）。
    pub fn inline_content(&self) -> bool {
        self.node_type.inline_content
    }

    /// 获取文本内容（仅文本节点）。
    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    /// 类型和属性相同（不比较内容）。
    pub fn same_markup(&self, other: &Node) -> bool {
        self.node_type.name == other.node_type.name
            && self.attrs == other.attrs
            && Mark::same_set(&self.marks, &other.marks)
    }

    /// 结构相等比较。
    pub fn eq(&self, other: &Node) -> bool {
        self.same_markup(other) && self.content.eq(&other.content)
    }

    /// 创建同类型同属性的节点，替换内容。
    pub fn copy(&self, content: Fragment) -> Node {
        Node {
            node_type: Arc::clone(&self.node_type),
            attrs: self.attrs.clone(),
            content,
            marks: self.marks.clone(),
            text: None,
        }
    }

    /// 截取节点内容范围（文本节点按字符，块级节点按内容位置）。
    pub fn cut(&self, from: usize, to: usize) -> Node {
        if let Some(ref t) = self.text {
            let chars: Vec<char> = t.chars().collect();
            let slice: String = chars[from..to].iter().collect();
            Node {
                node_type: Arc::clone(&self.node_type),
                attrs: self.attrs.clone(),
                content: Fragment::empty(),
                marks: self.marks.clone(),
                text: Some(slice),
            }
        } else {
            self.copy(self.content.cut(from, Some(to)))
        }
    }

    /// 创建相同类型和标记但文本不同的节点（仅文本节点）。
    pub fn with_text(&self, text: String) -> Node {
        Node {
            node_type: Arc::clone(&self.node_type),
            attrs: self.attrs.clone(),
            content: Fragment::empty(),
            marks: self.marks.clone(),
            text: Some(text),
        }
    }

    /// 在子节点索引 index 处的 ContentMatch 状态。
    pub fn content_match_at(&self, index: usize) -> Option<Arc<super::content::ContentMatch>> {
        let cm = self.node_type.content_match.as_ref()?;
        let mut state = Arc::clone(cm);
        for i in 0..index {
            state = state.match_type(&self.content.child(i).node_type)?;
        }
        Some(state)
    }

    /// 检查将 from..to（子节点索引）替换为 replacement[start..end] 是否合法。
    pub fn can_replace(
        &self,
        from: usize,
        to: usize,
        replacement: &Fragment,
        start: usize,
        end: usize,
    ) -> bool {
        let one = match self.content_match_at(from) {
            Some(cm) => cm.match_fragment(replacement, start, end),
            None => return false,
        };
        let two = one.and_then(|s| {
            s.match_fragment(&self.content, to, self.content.child_count())
        });
        match two {
            Some(ref s) if s.valid_end => {}
            _ => return false,
        }
        for i in start..end {
            if !self.node_type.allows_marks(&replacement.child(i).marks) {
                return false;
            }
        }
        true
    }

    /// 从 from 到 to 位置剪出一个 Slice。
    pub fn slice(&self, from: usize, to: usize) -> Result<Slice, String> {
        if from > to || to > self.content.size {
            return Err(format!("slice({}, {}) out of range (size={})", from, to, self.content.size));
        }
        if from == to {
            return Ok(Slice::empty());
        }
        let from_pos = ResolvedPos::resolve(self, from)?;
        let to_pos = ResolvedPos::resolve(self, to)?;
        let depth = from_pos.shared_depth(to);
        let start = from_pos.start(Some(depth as isize));
        let node = from_pos.node(Some(depth as isize));
        let content = node.content.cut(from - start, Some(to - start));
        Ok(Slice::new(content, from_pos.depth() - depth, to_pos.depth() - depth))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_slice_basic() {
        let text_nt = Arc::new(NodeType {
            name: "text".into(), groups: vec![], is_block: false,
            is_text: true, inline_content: false, mark_set: None, content_match: None,
        });
        let para_nt = Arc::new(NodeType {
            name: "paragraph".into(), groups: vec![], is_block: true,
            is_text: false, inline_content: true, mark_set: None, content_match: None,
        });
        let doc_nt = Arc::new(NodeType {
            name: "doc".into(), groups: vec![], is_block: true,
            is_text: false, inline_content: false, mark_set: None, content_match: None,
        });
        let txt = Node { node_type: Arc::clone(&text_nt), attrs: std::collections::BTreeMap::new(),
                         content: super::Fragment::empty(), marks: vec![], text: Some("hello".into()) };
        let para = Node { node_type: Arc::clone(&para_nt), attrs: std::collections::BTreeMap::new(),
                          content: super::Fragment::from_array(vec![txt]), marks: vec![], text: None };
        let doc = Node { node_type: Arc::clone(&doc_nt), attrs: std::collections::BTreeMap::new(),
                         content: super::Fragment::from_array(vec![para]), marks: vec![], text: None };
        // doc pos 布局: 0[1 h e l l o 6]7  (doc.content.size=7)
        // slice(1,7): from=para 内部开始, to=doc 末尾
        // sharedDepth(7): start(1)=1, end(1)=6, 6 < 7 不满足 → depth=0
        // open_start = from_pos.depth(1) - depth(0) = 1
        // open_end = to_pos.depth(0) - depth(0) = 0 → 但 to=7 在 doc 末尾，depth=0
        // content = doc.content.cut(1-0, 7-0) = doc.content.cut(1, 7) = para fragment
        let slice = doc.slice(1, 7).unwrap();
        assert_eq!(slice.open_start, 1);
        assert_eq!(slice.open_end, 0);
        assert_eq!(slice.content.child_count(), 1);
    }

    #[test]
    fn node_slice_empty_range() {
        let doc_nt = Arc::new(NodeType {
            name: "doc".into(), groups: vec![], is_block: true,
            is_text: false, inline_content: false, mark_set: None, content_match: None,
        });
        let doc = Node { node_type: doc_nt, attrs: std::collections::BTreeMap::new(),
                         content: super::Fragment::empty(), marks: vec![], text: None };
        let slice = doc.slice(0, 0).unwrap();
        assert_eq!(slice.size(), 0);
    }
}
