use std::sync::Arc;

use super::content::ContentMatch;
use super::fragment::Fragment;
use super::mark::Mark;
use super::node::Node;
use super::Attrs;

/// Schema 构建错误
#[derive(Debug, Clone)]
pub enum SchemaError {
    EmptyNodes,
    ContentParseError(String),
    UnknownMarkRef(String),
}

impl std::fmt::Display for SchemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaError::EmptyNodes => write!(f, "Schema must have at least one node type"),
            SchemaError::ContentParseError(e) => write!(f, "Content parse error: {}", e),
            SchemaError::UnknownMarkRef(n) => write!(f, "Unknown mark type: {}", n),
        }
    }
}

/// 节点类型构建描述
#[derive(Debug, Clone, Default)]
pub struct NodeSpec {
    /// content 表达式，如 "paragraph+" 或 "inline*"；None 表示叶节点
    pub content: Option<String>,
    /// 允许的 mark："_"=全部，""=无，空格分隔名称列表
    pub marks: Option<String>,
    /// 所属分组，空格分隔，如 "block" 或 "block inline"
    pub group: Option<String>,
    /// 是否行内节点（默认 false = 块级）
    pub inline: bool,
    /// 是否文本节点
    pub is_text: bool,
}

/// 标记类型构建描述
#[derive(Debug, Clone, Default)]
pub struct MarkSpec {
    /// 排序优先级（越小越靠前）
    pub rank: usize,
    /// 排斥的 mark："_"=排斥所有，""=仅排斥自身同类，空格分隔名称列表
    pub excludes: Option<String>,
    /// 光标到边界时是否延伸；None = 默认 true
    pub inclusive: Option<bool>,
}

/// Schema 构建规格
pub struct SchemaSpec {
    /// 有序节点列表；第一个为 topNode
    pub nodes: Vec<(String, NodeSpec)>,
    pub marks: Vec<(String, MarkSpec)>,
}

/// 节点类型。定义节点的名称、内容规则、属性等。
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
    /// 是否包含行内内容（textblock）
    pub inline_content: bool,
    /// 允许的标记类型集合；None = 全部允许
    pub mark_set: Option<Vec<Arc<MarkType>>>,
    /// content 表达式的起始匹配状态
    pub content_match: Option<Arc<ContentMatch>>,
}

impl NodeType {
    /// 是否是行内节点。
    pub fn is_inline(&self) -> bool {
        !self.is_block
    }

    /// 是否是叶子节点（content_match 为 empty 或无内容）。
    pub fn is_leaf(&self) -> bool {
        match &self.content_match {
            Some(cm) => cm.next.get().map(|v| v.is_empty()).unwrap_or(true) && cm.valid_end,
            None => true,
        }
    }

    /// 是否是 textblock（块级且包含行内内容）。
    pub fn is_textblock(&self) -> bool {
        self.is_block && self.inline_content
    }

    /// 是否属于指定分组。
    pub fn is_in_group(&self, group: &str) -> bool {
        self.groups.iter().any(|g| g == group)
    }

    /// 是否有必需属性（无默认值的属性）。
    pub fn has_required_attrs(&self) -> bool {
        false
    }

    /// content expression 兼容性检查。
    pub fn compatible_content(&self, other: &NodeType) -> bool {
        std::ptr::eq(self, other)
            || match (&self.content_match, &other.content_match) {
                (Some(a), Some(b)) => a.compatible(b),
                _ => false,
            }
    }

    /// 检查 fragment 是否为合法内容。
    pub fn valid_content(&self, content: &Fragment) -> bool {
        let cm = match &self.content_match {
            Some(cm) => cm,
            None => return content.size == 0,
        };
        let result = cm.match_fragment(content, 0, content.child_count());
        match result {
            Some(ref state) if state.valid_end => {}
            _ => return false,
        }
        for i in 0..content.child_count() {
            if !self.allows_marks(&content.child(i).marks) {
                return false;
            }
        }
        true
    }

    /// 断言 fragment 为合法内容，否则返回错误。
    pub fn check_content(&self, content: &Fragment) -> Result<(), String> {
        if !self.valid_content(content) {
            Err(format!("Invalid content for node {}", self.name))
        } else {
            Ok(())
        }
    }

    /// 是否允许特定 MarkType（按 name 比较）。
    pub fn allows_mark_type(&self, mt: &Arc<MarkType>) -> bool {
        match &self.mark_set {
            None => true,
            Some(set) => set.iter().any(|m| m.name == mt.name),
        }
    }

    /// 是否允许整个标记集合中的所有标记。
    pub fn allows_marks(&self, marks: &[Mark]) -> bool {
        self.mark_set.is_none()
            || marks.iter().all(|m| self.allows_mark_type(&m.mark_type))
    }

    /// 创建并自动填充一个此类型的节点（需要 &Arc<Self> 以构造 Node）。
    pub fn create_and_fill(self: &Arc<Self>) -> Option<Node> {
        if self.has_required_attrs() {
            return None;
        }
        let content = if let Some(ref cm) = self.content_match {
            if cm.valid_end {
                Fragment::empty()
            } else {
                // content match 要求至少一个子节点，尝试默认类型递归填充
                let default_nt = cm.default_type()?;
                let child = default_nt.create_and_fill()?;
                Fragment::from_array(vec![child])
            }
        } else {
            Fragment::empty()
        };
        Some(Node {
            node_type: Arc::clone(self),
            attrs: Attrs::new(),
            content,
            marks: vec![],
            text: None,
        })
    }
}

impl PartialEq for NodeType {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// 标记类型。定义标记的名称、排序优先级、排斥关系。
#[derive(Debug)]
pub struct MarkType {
    /// 标记类型名称（如 "bold"、"italic"）
    pub name: String,
    /// 排序优先级，决定标记集中的顺序
    pub rank: usize,
    /// 被此标记排斥的标记类型列表
    pub excluded: Vec<Arc<MarkType>>,
    /// 光标到达边界时是否延伸此标记；None = 默认 true
    pub inclusive: Option<bool>,
}

impl MarkType {
    /// 检查此标记类型是否排斥另一个标记类型。
    /// 按名称判断排斥（支持不同 Arc 实例的同名类型）。
    pub fn excludes(&self, other: &MarkType) -> bool {
        self.excluded.iter().any(|e| e.name == other.name)
    }
}

impl PartialEq for MarkType {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::content::ContentMatch;
    use super::super::fragment::Fragment;
    use super::super::node::Node;
    use super::super::mark::Mark;
    use std::collections::{BTreeMap, HashMap};

    fn make_nt(name: &str, is_block: bool) -> Arc<NodeType> {
        Arc::new(NodeType {
            name: name.into(), groups: vec![], is_block,
            is_text: name == "text", inline_content: false,
            mark_set: None, content_match: None,
        })
    }

    fn make_mt(name: &str, rank: usize) -> Arc<MarkType> {
        Arc::new(MarkType { name: name.into(), rank, excluded: vec![], inclusive: None })
    }

    fn make_mark(mt: Arc<MarkType>) -> Mark {
        Mark { mark_type: mt, attrs: BTreeMap::new() }
    }

    fn block_node(nt: Arc<NodeType>) -> Node {
        Node {
            node_type: nt, attrs: BTreeMap::new(),
            content: Fragment::empty(), marks: vec![], text: None,
        }
    }

    // ── compatible_content ───────────────────────────────────

    #[test]
    fn compatible_content_same_instance() {
        let nt = make_nt("p", true);
        // ptr_eq: same Arc → true
        assert!(nt.compatible_content(&nt));
    }

    #[test]
    fn compatible_content_different_names() {
        let p = make_nt("p", true);
        let div = make_nt("div", true);
        // Different instances, no content_match → false
        assert!(!p.compatible_content(&div));
    }

    // ── valid_content / check_content ────────────────────────

    #[test]
    fn valid_content_no_match_empty() {
        let nt = make_nt("p", true);
        // content_match = None → only empty is valid
        assert!(nt.valid_content(&Fragment::empty()));
    }

    #[test]
    fn valid_content_with_parse() {
        let p_type = make_nt("p", true);
        let mut types: HashMap<String, Arc<NodeType>> = HashMap::new();
        types.insert("p".into(), Arc::clone(&p_type));
        let cm = ContentMatch::parse("p*", &types).unwrap();
        let doc_nt = Arc::new(NodeType {
            name: "doc".into(), groups: vec![], is_block: true,
            is_text: false, inline_content: false, mark_set: None,
            content_match: Some(cm),
        });
        // Empty content matches "p*"
        assert!(doc_nt.valid_content(&Fragment::empty()));
        // One p node matches "p*"
        let frag = Fragment::from_array(vec![block_node(Arc::clone(&p_type))]);
        assert!(doc_nt.valid_content(&frag));
    }

    #[test]
    fn check_content_error_message() {
        let nt = make_nt("p", true);
        let frag = Fragment::from_array(vec![block_node(make_nt("inner", true))]);
        // content_match=None, non-empty content → invalid
        let result = nt.check_content(&frag);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("p"));
    }

    // ── allows_marks ────────────────────────────────────────

    #[test]
    fn allows_marks_none_allows_all() {
        let nt = make_nt("p", true); // mark_set = None
        let bold = make_mark(make_mt("bold", 0));
        assert!(nt.allows_marks(&[bold]));
    }

    #[test]
    fn allows_marks_restricted_set() {
        let bold_mt = make_mt("bold", 0);
        let em_mt = make_mt("em", 1);
        let nt = Arc::new(NodeType {
            name: "p".into(), groups: vec![], is_block: true,
            is_text: false, inline_content: false,
            mark_set: Some(vec![Arc::clone(&bold_mt)]),
            content_match: None,
        });
        let bold = make_mark(Arc::clone(&bold_mt));
        let em = make_mark(Arc::clone(&em_mt));
        assert!(nt.allows_marks(&[bold]));
        assert!(!nt.allows_marks(&[em]));
    }

    // ── is_textblock ─────────────────────────────────────────

    #[test]
    fn is_textblock() {
        let nt = Arc::new(NodeType {
            name: "p".into(), groups: vec![], is_block: true,
            is_text: false, inline_content: true, mark_set: None, content_match: None,
        });
        assert!(nt.is_textblock());
        assert!(!make_nt("div", true).is_textblock());
    }
}
