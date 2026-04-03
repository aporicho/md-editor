use std::sync::Arc;

use super::content::ContentMatch;

/// 节点类型。定义节点的名称、内容规则、属性等。
/// 后续移植 schema.ts 时补全完整实现。
#[derive(Debug)]
pub struct NodeType {
    /// 节点类型名称（如 "doc"、"paragraph"、"heading"）
    pub name: String,
    /// 所属分组（如 ["block"]、["inline"]）
    pub groups: Vec<String>,
    /// 是否是块级节点
    pub is_block: bool,
    /// 是否是文本节点
    pub is_text: bool,
    /// content 表达式的起始匹配状态
    pub content_match: Option<Arc<ContentMatch>>,
}

impl NodeType {
    /// 是否是行内节点。
    pub fn is_inline(&self) -> bool {
        !self.is_block
    }

    /// 是否是叶子节点（content_match 为 empty）。
    pub fn is_leaf(&self) -> bool {
        match &self.content_match {
            Some(cm) => cm.next.is_empty() && cm.valid_end,
            None => true,
        }
    }

    /// 是否属于指定分组。
    pub fn is_in_group(&self, group: &str) -> bool {
        self.groups.iter().any(|g| g == group)
    }

    /// 是否有必需属性（无默认值的属性）。
    /// TODO: 移植 schema.ts 时补全。
    pub fn has_required_attrs(&self) -> bool {
        false
    }

    /// 创建并自动填充一个此类型的节点。
    /// TODO: 移植 node.ts 时补全。
    pub fn create_and_fill(&self) -> Option<()> {
        // 占位，返回 Some(()) 表示总是能创建
        Some(())
    }
}

impl PartialEq for NodeType {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// 标记类型。定义标记的名称、排序优先级、排斥关系。
/// 后续移植 schema.ts 时补全完整实现。
#[derive(Debug)]
pub struct MarkType {
    /// 标记类型名称（如 "bold"、"italic"）
    pub name: String,
    /// 排序优先级，决定标记集中的顺序
    pub rank: usize,
    /// 被此标记排斥的标记类型列表
    pub excluded: Vec<Arc<MarkType>>,
}

impl MarkType {
    /// 检查此标记类型是否排斥另一个标记类型。
    pub fn excludes(&self, other: &MarkType) -> bool {
        self.excluded.iter().any(|e| std::ptr::eq(e.as_ref(), other))
    }
}

impl PartialEq for MarkType {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
