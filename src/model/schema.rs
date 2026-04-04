use std::collections::HashMap;
use std::sync::Arc;

use super::content::ContentMatch;
use super::fragment::Fragment;
use super::mark::Mark;
use super::node::Node;
use super::Attrs;

/// Schema 构建错误
#[derive(Debug, Clone)]
pub enum SchemaError {
    /// 节点列表为空，Schema 必须至少有一个节点类型
    EmptyNodes,
    /// content 表达式解析失败
    ContentParseError(String),
    /// 引用了未知的 mark 类型名称
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
#[derive(Debug, Clone)]
pub struct SchemaSpec {
    /// 有序节点列表；第一个为 topNode
    pub nodes: Vec<(String, NodeSpec)>,
    /// 有序 mark 列表
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

/// 中央类型注册表。所有 NodeType / MarkType 均通过此处统一创建。
pub struct Schema {
    pub nodes: HashMap<String, Arc<NodeType>>,
    pub marks: HashMap<String, Arc<MarkType>>,
    /// 文档根节点类型（spec.nodes 第一个）
    pub top_node_type: Arc<NodeType>,
}

impl Schema {
    pub fn new(spec: SchemaSpec) -> Result<Arc<Self>, SchemaError> {
        if spec.nodes.is_empty() {
            return Err(SchemaError::EmptyNodes);
        }

        // ── 第一遍：注册所有 MarkType（excluded 先留空）────────────
        let mut mark_types: HashMap<String, Arc<MarkType>> = HashMap::new();
        for (name, ms) in &spec.marks {
            mark_types.insert(name.clone(), Arc::new(MarkType {
                name: name.clone(),
                rank: ms.rank,
                excluded: vec![],
                inclusive: ms.inclusive,
            }));
        }

        // 填充 excluded：先收集所有 excluded 名称列表，再重建最终 Arc
        // （不能在持有 Arc clone 的同时调用 Arc::get_mut，需先把旧 Arc 全部丢掉）
        let excluded_names: Vec<(String, Vec<String>)> = spec.marks.iter()
            .map(|(name, ms)| {
                let names = Self::resolve_excluded_names(name, ms, &mark_types)?;
                Ok((name.clone(), names))
            })
            .collect::<Result<Vec<_>, SchemaError>>()?;

        // 用最终 excluded 重建每个 MarkType：
        // 先在不可变借用下把所有 excluded Arc 全部收集成 Vec<(name, Vec<Arc>)>，
        // 再统一 remove / re-insert（此时旧 Arc 已 drop，引用计数为 0）。
        let resolved_excluded: Vec<(String, Vec<Arc<MarkType>>)> = excluded_names.iter()
            .map(|(name, exc_names)| {
                let arcs: Vec<Arc<MarkType>> = exc_names.iter()
                    .map(|n| Arc::clone(mark_types.get(n).unwrap()))
                    .collect();
                (name.clone(), arcs)
            })
            .collect();

        // 现在把旧 Arc 全部从 map 移出，引用计数降回 1，随即 drop
        let old_marks: Vec<(String, Arc<MarkType>)> = resolved_excluded.iter()
            .map(|(name, _)| (name.clone(), mark_types.remove(name).unwrap()))
            .collect();

        // 创建新 Arc（带 excluded）并重新插入
        for ((name, arcs), (_, old)) in resolved_excluded.into_iter().zip(old_marks.into_iter()) {
            mark_types.insert(name, Arc::new(MarkType {
                name: old.name.clone(),
                rank: old.rank,
                inclusive: old.inclusive,
                excluded: arcs,
            }));
        }

        // ── 第一遍：注册所有 NodeType（content_match = None）────────
        let mut node_types: HashMap<String, Arc<NodeType>> = HashMap::new();
        for (name, ns) in &spec.nodes {
            node_types.insert(name.clone(), Arc::new(NodeType {
                name: name.clone(),
                groups: ns.group.as_deref()
                    .map(|g| g.split_whitespace().map(String::from).collect())
                    .unwrap_or_default(),
                is_block: !ns.inline,
                is_text: ns.is_text,
                inline_content: false,
                mark_set: None,
                content_match: None,
            }));
        }

        // ── 第二遍：先解析所有 content 表达式（只需 &node_types 不可变借用）──
        let parsed_content: Vec<Option<Arc<ContentMatch>>> = spec.nodes.iter()
            .map(|(_, ns)| {
                if let Some(ref expr) = ns.content {
                    ContentMatch::parse(expr, &node_types)
                        .map(Some)
                        .map_err(SchemaError::ContentParseError)
                } else {
                    Ok(None)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        // ── 第三遍：填充 inline_content / content_match / mark_set ──
        // ContentMatch::parse 内部会 Arc::clone NodeType，使引用计数 > 1，
        // 无法用 Arc::get_mut；改为重建整个 NodeType Arc 并替换 HashMap 中的旧值。
        //
        // 注意：重建后 schema.nodes 中的 NodeType Arc 与 ContentMatch DFA 内
        // MatchEdge 持有的 Arc 是不同实例，ptr_eq 会为 false。
        // 因此 ContentMatch::match_type / compatible 必须继续使用 name 字符串比较，
        // 不能改为 ptr_eq。ptr_eq 只对 MarkType（excluded / mark_set）有效。
        let mark_sets: Vec<(String, Option<Vec<Arc<MarkType>>>)> = spec.nodes.iter()
            .map(|(name, ns)| {
                let ms = Self::resolve_mark_set(&ns.marks, &mark_types)?;
                Ok((name.clone(), ms))
            })
            .collect::<Result<Vec<_>, SchemaError>>()?;

        for ((name, ns), cm_opt, (_, mark_set)) in
            spec.nodes.iter()
                .zip(parsed_content.into_iter())
                .zip(mark_sets.into_iter())
                .map(|((a, b), c)| (a, b, c))
        {
            let old = node_types.remove(name).unwrap();
            let inline_content = cm_opt.as_ref().map(|cm| cm.inline_content()).unwrap_or(false);
            node_types.insert(name.clone(), Arc::new(NodeType {
                name: old.name.clone(),
                groups: old.groups.clone(),
                is_block: old.is_block,
                is_text: old.is_text,
                inline_content,
                mark_set,
                content_match: cm_opt,
            }));
        }

        let top_node_type = Arc::clone(
            node_types.get(&spec.nodes[0].0).unwrap()
        );

        Ok(Arc::new(Schema { nodes: node_types, marks: mark_types, top_node_type }))
    }

    fn resolve_mark_set(
        marks: &Option<String>,
        mark_types: &HashMap<String, Arc<MarkType>>,
    ) -> Result<Option<Vec<Arc<MarkType>>>, SchemaError> {
        match marks.as_deref() {
            None | Some("_") => Ok(None),
            Some("") => Ok(Some(vec![])),
            Some(s) => {
                let set = s.split_whitespace()
                    .map(|name| {
                        mark_types.get(name)
                            .map(Arc::clone)
                            .ok_or_else(|| SchemaError::UnknownMarkRef(name.to_string()))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Some(set))
            }
        }
    }

    /// 返回 excluded mark 的名称列表（不持有 Arc，避免引用计数干扰 Arc::get_mut）
    fn resolve_excluded_names(
        self_name: &str,
        ms: &MarkSpec,
        mark_types: &HashMap<String, Arc<MarkType>>,
    ) -> Result<Vec<String>, SchemaError> {
        match ms.excludes.as_deref() {
            None | Some("") => {
                if mark_types.contains_key(self_name) {
                    Ok(vec![self_name.to_string()])
                } else {
                    Ok(vec![])
                }
            }
            Some("_") => {
                Ok(mark_types.keys().cloned().collect())
            }
            Some(s) => {
                s.split_whitespace()
                    .map(|name| {
                        if mark_types.contains_key(name) {
                            Ok(name.to_string())
                        } else {
                            Err(SchemaError::UnknownMarkRef(name.to_string()))
                        }
                    })
                    .collect()
            }
        }
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

    // ── Schema::new ──────────────────────────────────────────

    #[test]
    fn schema_new_basic() {
        let spec = SchemaSpec {
            nodes: vec![
                ("doc".into(), NodeSpec { content: Some("paragraph+".into()), ..Default::default() }),
                ("paragraph".into(), NodeSpec {
                    content: Some("text*".into()),
                    group: Some("block".into()),
                    ..Default::default()
                }),
                ("text".into(), NodeSpec { inline: true, is_text: true, ..Default::default() }),
            ],
            marks: vec![
                ("bold".into(), MarkSpec { rank: 0, ..Default::default() }),
            ],
        };
        let schema = Schema::new(spec).expect("schema build should succeed");
        assert!(schema.nodes.contains_key("doc"));
        assert!(schema.nodes.contains_key("paragraph"));
        assert!(schema.nodes.contains_key("text"));
        assert!(schema.marks.contains_key("bold"));
        assert_eq!(schema.top_node_type.name, "doc");
    }

    #[test]
    fn schema_new_empty_nodes_fails() {
        let spec = SchemaSpec { nodes: vec![], marks: vec![] };
        assert!(Schema::new(spec).is_err());
    }

    #[test]
    fn schema_same_type_ptr_eq() {
        let spec = SchemaSpec {
            nodes: vec![
                ("doc".into(), NodeSpec { content: Some("paragraph+".into()), ..Default::default() }),
                ("paragraph".into(), NodeSpec { group: Some("block".into()), ..Default::default() }),
            ],
            marks: vec![],
        };
        let schema = Schema::new(spec).unwrap();
        let p1 = schema.nodes.get("paragraph").unwrap();
        let p2 = schema.nodes.get("paragraph").unwrap();
        assert!(Arc::ptr_eq(p1, p2), "同名类型应是同一 Arc 实例");
    }
}
