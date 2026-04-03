use std::sync::Arc;

use super::fragment::Fragment;
use super::mark::Mark;
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
            // 文本节点大小 = 字符数（Unicode 字符）
            t.chars().count()
        } else if self.node_type.is_leaf() {
            1
        } else {
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

    /// 获取文本内容（仅文本节点）。
    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    /// 类型和属性相同（不比较内容）。
    pub fn same_markup(&self, other: &Node) -> bool {
        Arc::ptr_eq(&self.node_type, &other.node_type)
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
}
