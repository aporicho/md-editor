use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, OnceLock};

use super::fragment::Fragment;
use super::node::Node;
use super::schema::NodeType;

/// content 表达式匹配的一条出边。
#[derive(Debug, Clone)]
pub struct MatchEdge {
    pub node_type: Arc<NodeType>,
    pub next: Arc<ContentMatch>,
}

/// content 表达式编译后的 DFA 状态。
/// 用于验证节点的子节点序列是否合法。
///
/// 对应 prosemirror-model/src/content.ts ContentMatch
#[derive(Debug)]
pub struct ContentMatch {
    /// 此状态是否是合法的结束状态
    pub valid_end: bool,
    /// 出边列表（OnceLock 保证构建完成后一次性初始化）
    pub next: OnceLock<Vec<MatchEdge>>,
}

impl ContentMatch {
    fn new(valid_end: bool) -> Self {
        Self {
            valid_end,
            next: OnceLock::new(),
        }
    }

    /// 空匹配——叶子节点（无子节点）使用。
    pub fn empty() -> Arc<ContentMatch> {
        let cm = Arc::new(ContentMatch::new(true));
        cm.next.set(vec![]).expect("ContentMatch::empty: OnceLock already set");
        cm
    }

    /// 编译 content 表达式字符串为 ContentMatch DFA。
    pub fn parse(
        expr: &str,
        node_types: &HashMap<String, Arc<NodeType>>,
    ) -> Result<Arc<ContentMatch>, String> {
        let mut stream = TokenStream::new(expr, node_types);
        if stream.next().is_none() {
            return Ok(Self::empty());
        }
        let ast = parse_expr(&mut stream)?;
        if stream.next().is_some() {
            return Err(format!(
                "Unexpected trailing text (in content expression '{}')",
                expr
            ));
        }
        let nfa_states = build_nfa(&ast);
        let result = build_dfa(&nfa_states);
        check_for_dead_ends(&result, expr)?;
        Ok(result)
    }

    /// 匹配一个节点类型，返回下一个状态。
    pub fn match_type(&self, node_type: &Arc<NodeType>) -> Option<Arc<ContentMatch>> {
        for edge in self.next.get().map(|v| v.as_slice()).unwrap_or(&[]) {
            if edge.node_type.name == node_type.name {
                return Some(Arc::clone(&edge.next));
            }
        }
        None
    }

    /// 此状态的子节点是否为行内内容。
    pub fn inline_content(&self) -> bool {
        let next = self.next.get().map(|v| v.as_slice()).unwrap_or(&[]);
        !next.is_empty() && next[0].node_type.is_inline()
    }

    /// 获取此状态第一个可自动生成的默认节点类型。
    pub fn default_type(&self) -> Option<Arc<NodeType>> {
        for edge in self.next.get().map(|v| v.as_slice()).unwrap_or(&[]) {
            if !edge.node_type.is_text && !edge.node_type.has_required_attrs() {
                return Some(Arc::clone(&edge.node_type));
            }
        }
        None
    }

    /// 检查两个 ContentMatch 是否有兼容的出边。
    pub fn compatible(&self, other: &ContentMatch) -> bool {
        let a_next = self.next.get().map(|v| v.as_slice()).unwrap_or(&[]);
        let b_next = other.next.get().map(|v| v.as_slice()).unwrap_or(&[]);
        for a in a_next {
            for b in b_next {
                if a.node_type.name == b.node_type.name {
                    return true;
                }
            }
        }
        false
    }

    /// 出边数量。
    pub fn edge_count(&self) -> usize {
        self.next.get().map(|v| v.len()).unwrap_or(0)
    }

    /// 获取第 n 条出边。
    pub fn edge(&self, n: usize) -> Option<&MatchEdge> {
        self.next.get().and_then(|v| v.get(n))
    }

    /// 从 start..end 范围的 frag 子节点顺序匹配，返回最终状态。
    pub fn match_fragment(
        self: &Arc<Self>,
        frag: &Fragment,
        start: usize,
        end: usize,
    ) -> Option<Arc<ContentMatch>> {
        let mut cur = Arc::clone(self);
        for i in start..end {
            cur = cur.match_type(&frag.child(i).node_type)?;
        }
        Some(cur)
    }

    /// BFS 查找最短填充序列，使得插入后接上 after[start_index..] 仍合法。
    ///
    /// `to_end`：要求最终状态也是 valid_end。
    pub fn fill_before(
        self: &Arc<Self>,
        after: &Fragment,
        to_end: bool,
        start_index: usize,
    ) -> Option<Fragment> {
        struct Entry {
            state: Arc<ContentMatch>,
            types: Vec<Arc<NodeType>>,
        }

        let mut seen: HashSet<usize> = HashSet::new();
        seen.insert(Arc::as_ptr(self) as usize);
        let mut queue: VecDeque<Entry> = VecDeque::new();
        queue.push_back(Entry { state: Arc::clone(self), types: vec![] });

        while let Some(Entry { state, types }) = queue.pop_front() {
            let finished = state.match_fragment(after, start_index, after.child_count());
            if let Some(ref f) = finished {
                if !to_end || f.valid_end {
                    let nodes: Vec<Node> = types.iter()
                        .filter_map(|t| t.create_and_fill())
                        .collect();
                    return Some(Fragment::from_array(nodes));
                }
            }
            for edge in state.next.get().map(|v| v.as_slice()).unwrap_or(&[]) {
                let nt = &edge.node_type;
                let ptr = Arc::as_ptr(&edge.next) as usize;
                if !nt.is_text && !nt.has_required_attrs() && !seen.contains(&ptr) {
                    seen.insert(ptr);
                    let mut new_types = types.clone();
                    new_types.push(Arc::clone(nt));
                    queue.push_back(Entry {
                        state: Arc::clone(&edge.next),
                        types: new_types,
                    });
                }
            }
        }
        None
    }

    /// 查找使目标节点类型合法的包裹节点类型序列（索引回溯 BFS）。
    pub fn find_wrapping(self: &Arc<Self>, target: &Arc<NodeType>) -> Option<Vec<Arc<NodeType>>> {
        struct Active {
            content_match: Arc<ContentMatch>,
            node_type: Option<Arc<NodeType>>,
            via: Option<usize>, // 指向 active 数组中前驱的下标
        }

        let mut seen: HashMap<String, bool> = HashMap::new();
        let mut active: Vec<Active> = vec![Active {
            content_match: Arc::clone(self),
            node_type: None,
            via: None,
        }];
        let mut head = 0;

        while head < active.len() {
            let match_ref = Arc::clone(&active[head].content_match);
            if match_ref.match_type(target).is_some() {
                // 沿 via 链回溯，收集包裹节点类型
                let mut result = vec![];
                let mut idx = head;
                loop {
                    match active[idx].node_type.clone() {
                        Some(nt) => {
                            result.push(nt);
                            idx = active[idx].via.unwrap();
                        }
                        None => break,
                    }
                }
                result.reverse();
                return Some(result);
            }

            let is_root = active[head].node_type.is_none();
            // match_ref 是 Arc::clone，不持有 active 借用，for 循环内 push 安全
            for edge in match_ref.next.get().map(|v| v.as_slice()).unwrap_or(&[]) {
                let nt = &edge.node_type;
                if !nt.is_leaf()
                    && !nt.has_required_attrs()
                    && !seen.contains_key(&nt.name)
                    && (is_root || edge.next.valid_end)
                {
                    seen.insert(nt.name.clone(), true);
                    if let Some(ref cm) = nt.content_match {
                        active.push(Active {
                            content_match: Arc::clone(cm),
                            node_type: Some(Arc::clone(nt)),
                            via: Some(head),
                        });
                    }
                }
            }
            head += 1;
        }
        None
    }
}

// ============================================================
// 以下为 content 表达式的解析器和编译器（内部实现）
// ============================================================

/// content 表达式的 AST 节点
#[derive(Debug, Clone)]
enum Expr {
    Name(Arc<NodeType>),
    Choice(Vec<Expr>),
    Seq(Vec<Expr>),
    Plus(Box<Expr>),
    Star(Box<Expr>),
    Opt(Box<Expr>),
    Range {
        min: usize,
        max: Option<usize>, // None = 无上限
        expr: Box<Expr>,
    },
}

/// 词法分析器
struct TokenStream<'a> {
    tokens: Vec<String>,
    pos: usize,
    string: &'a str,
    node_types: &'a HashMap<String, Arc<NodeType>>,
    inline: Option<bool>,
}

impl<'a> TokenStream<'a> {
    fn new(string: &'a str, node_types: &'a HashMap<String, Arc<NodeType>>) -> Self {
        let tokens: Vec<String> = string
            .split_whitespace()
            .flat_map(|s| {
                let mut result = Vec::new();
                let mut current = String::new();
                for ch in s.chars() {
                    if "(){}|+*?,".contains(ch) {
                        if !current.is_empty() {
                            result.push(current.clone());
                            current.clear();
                        }
                        result.push(ch.to_string());
                    } else {
                        current.push(ch);
                    }
                }
                if !current.is_empty() {
                    result.push(current);
                }
                result
            })
            .collect();
        Self {
            tokens,
            pos: 0,
            string,
            node_types,
            inline: None,
        }
    }

    fn next(&self) -> Option<&str> {
        self.tokens.get(self.pos).map(|s| s.as_str())
    }

    fn eat(&mut self, tok: &str) -> bool {
        if self.next() == Some(tok) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn err(&self, msg: &str) -> String {
        format!("{} (in content expression '{}')", msg, self.string)
    }
}

fn parse_expr(stream: &mut TokenStream) -> Result<Expr, String> {
    let mut exprs = vec![parse_expr_seq(stream)?];
    while stream.eat("|") {
        exprs.push(parse_expr_seq(stream)?);
    }
    if exprs.len() == 1 {
        Ok(exprs.remove(0))
    } else {
        Ok(Expr::Choice(exprs))
    }
}

fn parse_expr_seq(stream: &mut TokenStream) -> Result<Expr, String> {
    let mut exprs = vec![parse_expr_subscript(stream)?];
    while stream.next().is_some()
        && stream.next() != Some(")")
        && stream.next() != Some("|")
    {
        exprs.push(parse_expr_subscript(stream)?);
    }
    if exprs.len() == 1 {
        Ok(exprs.remove(0))
    } else {
        Ok(Expr::Seq(exprs))
    }
}

fn parse_expr_subscript(stream: &mut TokenStream) -> Result<Expr, String> {
    let mut expr = parse_expr_atom(stream)?;
    loop {
        if stream.eat("+") {
            expr = Expr::Plus(Box::new(expr));
        } else if stream.eat("*") {
            expr = Expr::Star(Box::new(expr));
        } else if stream.eat("?") {
            expr = Expr::Opt(Box::new(expr));
        } else if stream.eat("{") {
            expr = parse_expr_range(stream, expr)?;
        } else {
            break;
        }
    }
    Ok(expr)
}

fn parse_num(stream: &mut TokenStream) -> Result<usize, String> {
    match stream.next() {
        Some(s) => match s.parse::<usize>() {
            Ok(n) => {
                stream.pos += 1;
                Ok(n)
            }
            Err(_) => Err(stream.err(&format!("Expected number, got '{}'", s))),
        },
        None => Err(stream.err("Expected number")),
    }
}

fn parse_expr_range(stream: &mut TokenStream, expr: Expr) -> Result<Expr, String> {
    let min = parse_num(stream)?;
    let max = if stream.eat(",") {
        if stream.next() != Some("}") {
            Some(parse_num(stream)?)
        } else {
            None
        }
    } else {
        Some(min)
    };
    if !stream.eat("}") {
        return Err(stream.err("Unclosed braced range"));
    }
    Ok(Expr::Range {
        min,
        max,
        expr: Box::new(expr),
    })
}

fn resolve_name(stream: &mut TokenStream, name: &str) -> Result<Vec<Arc<NodeType>>, String> {
    if let Some(nt) = stream.node_types.get(name) {
        return Ok(vec![Arc::clone(nt)]);
    }
    let mut result: Vec<Arc<NodeType>> = Vec::new();
    for nt in stream.node_types.values() {
        if nt.is_in_group(name) {
            result.push(Arc::clone(nt));
        }
    }
    if result.is_empty() {
        Err(stream.err(&format!("No node type or group '{}' found", name)))
    } else {
        Ok(result)
    }
}

fn parse_expr_atom(stream: &mut TokenStream) -> Result<Expr, String> {
    if stream.eat("(") {
        let expr = parse_expr(stream)?;
        if !stream.eat(")") {
            return Err(stream.err("Missing closing paren"));
        }
        return Ok(expr);
    }

    let token = match stream.next() {
        Some(s) => s.to_string(),
        None => return Err(stream.err("Unexpected end of expression")),
    };

    if token.chars().all(|c| c.is_alphanumeric() || c == '_') {
        let types = resolve_name(stream, &token)?;
        stream.pos += 1;

        for nt in &types {
            let is_inline = nt.is_inline();
            match stream.inline {
                None => stream.inline = Some(is_inline),
                Some(prev) if prev != is_inline => {
                    return Err(stream.err("Mixing inline and block content"));
                }
                _ => {}
            }
        }

        let exprs: Vec<Expr> = types.into_iter().map(Expr::Name).collect();
        if exprs.len() == 1 {
            Ok(exprs.into_iter().next().unwrap())
        } else {
            Ok(Expr::Choice(exprs))
        }
    } else {
        Err(stream.err(&format!("Unexpected token '{}'", token)))
    }
}

// ============================================================
// NFA 构建
// ============================================================

#[derive(Clone)]
struct NfaEdge {
    term: Option<Arc<NodeType>>,
    to: usize,
}

fn build_nfa(expr: &Expr) -> Vec<Vec<NfaEdge>> {
    let mut states: Vec<Vec<NfaEdge>> = vec![vec![]];

    fn new_state(states: &mut Vec<Vec<NfaEdge>>) -> usize {
        states.push(vec![]);
        states.len() - 1
    }

    fn add_edge(
        states: &mut Vec<Vec<NfaEdge>>,
        from: usize,
        to: usize,
        term: Option<Arc<NodeType>>,
    ) -> usize {
        let idx = states[from].len();
        states[from].push(NfaEdge { term, to });
        idx
    }

    fn compile(
        expr: &Expr,
        from: usize,
        states: &mut Vec<Vec<NfaEdge>>,
    ) -> Vec<(usize, usize)> {
        match expr {
            Expr::Name(nt) => {
                let idx = add_edge(states, from, 0, Some(Arc::clone(nt)));
                vec![(from, idx)]
            }
            Expr::Choice(exprs) => {
                let mut out = Vec::new();
                for e in exprs {
                    out.extend(compile(e, from, states));
                }
                out
            }
            Expr::Seq(exprs) => {
                let mut cur = from;
                for (i, e) in exprs.iter().enumerate() {
                    let edges = compile(e, cur, states);
                    if i < exprs.len() - 1 {
                        let next = new_state(states);
                        connect(states, &edges, next);
                        cur = next;
                    } else {
                        return edges;
                    }
                }
                vec![]
            }
            Expr::Star(inner) => {
                let loop_state = new_state(states);
                add_edge(states, from, loop_state, None);
                let inner_edges = compile(inner, loop_state, states);
                connect(states, &inner_edges, loop_state);
                let idx = add_edge(states, loop_state, 0, None);
                vec![(loop_state, idx)]
            }
            Expr::Plus(inner) => {
                let loop_state = new_state(states);
                let edges1 = compile(inner, from, states);
                connect(states, &edges1, loop_state);
                let edges2 = compile(inner, loop_state, states);
                connect(states, &edges2, loop_state);
                let idx = add_edge(states, loop_state, 0, None);
                vec![(loop_state, idx)]
            }
            Expr::Opt(inner) => {
                let idx = add_edge(states, from, 0, None);
                let mut edges = vec![(from, idx)];
                edges.extend(compile(inner, from, states));
                edges
            }
            Expr::Range { min, max, expr } => {
                let mut cur = from;
                for _ in 0..*min {
                    let next = new_state(states);
                    let edges = compile(expr, cur, states);
                    connect(states, &edges, next);
                    cur = next;
                }
                match max {
                    None => {
                        let edges = compile(expr, cur, states);
                        connect(states, &edges, cur);
                    }
                    Some(m) => {
                        for _ in *min..*m {
                            let next = new_state(states);
                            add_edge(states, cur, next, None);
                            let edges = compile(expr, cur, states);
                            connect(states, &edges, next);
                            cur = next;
                        }
                    }
                }
                let idx = add_edge(states, cur, 0, None);
                vec![(cur, idx)]
            }
        }
    }

    fn connect(states: &mut Vec<Vec<NfaEdge>>, edges: &[(usize, usize)], to: usize) {
        for &(state_idx, edge_idx) in edges {
            states[state_idx][edge_idx].to = to;
        }
    }

    // 先 compile（创建中间状态），再创建 end（保证 nfa.len()-1 == accept）
    let dangling = compile(expr, 0, &mut states);
    let end = new_state(&mut states);
    connect(&mut states, &dangling, end);
    states
}

// ============================================================
// DFA 构建（NFA → DFA 子集构造法，arena 方案）
// ============================================================

fn null_from(nfa: &[Vec<NfaEdge>], node: usize) -> Vec<usize> {
    let mut result = Vec::new();
    let mut visited = vec![false; nfa.len()];
    scan(nfa, node, &mut result, &mut visited);
    result.sort_unstable_by(|a, b| b.cmp(a));
    result
}

fn scan(nfa: &[Vec<NfaEdge>], node: usize, result: &mut Vec<usize>, visited: &mut Vec<bool>) {
    if visited[node] {
        return;
    }
    visited[node] = true;
    let edges = &nfa[node];
    if edges.len() == 1 && edges[0].term.is_none() {
        return scan(nfa, edges[0].to, result, visited);
    }
    result.push(node);
    for edge in edges {
        if edge.term.is_none() {
            scan(nfa, edge.to, result, visited);
        }
    }
}

/// arena 中的 DFA 构建节点（出边用下标索引，无 Arc 循环）
struct BuildNode {
    valid_end: bool,
    edges: Vec<(Arc<NodeType>, usize)>, // (NodeType, 目标 arena 下标)
}

fn states_key(states: &[usize]) -> String {
    states
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

/// 递归构建 DFA 状态到 arena，返回起始节点的 arena 下标。
fn explore_into_arena(
    nfa: &[Vec<NfaEdge>],
    states: &[usize],
    accept: usize,
    key_to_idx: &mut HashMap<String, usize>,
    arena: &mut Vec<BuildNode>,
) -> usize {
    let key = states_key(states);
    if let Some(&idx) = key_to_idx.get(&key) {
        return idx;
    }
    let idx = arena.len();
    arena.push(BuildNode { valid_end: states.contains(&accept), edges: vec![] });
    key_to_idx.insert(key, idx);

    // 收集所有从当前状态集可达的 (NodeType, 目标状态集)
    let mut out: Vec<(Arc<NodeType>, Vec<usize>)> = Vec::new();
    for &node in states {
        for edge in &nfa[node] {
            if let Some(ref term) = edge.term {
                let mut found = false;
                for entry in &mut out {
                    if entry.0.name == term.name {
                        for s in null_from(nfa, edge.to) {
                            if !entry.1.contains(&s) {
                                entry.1.push(s);
                            }
                        }
                        found = true;
                        break;
                    }
                }
                if !found {
                    out.push((Arc::clone(term), null_from(nfa, edge.to)));
                }
            }
        }
    }

    for (node_type, mut target_states) in out {
        target_states.sort_unstable_by(|a, b| b.cmp(a));
        let target_idx = explore_into_arena(nfa, &target_states, accept, key_to_idx, arena);
        arena[idx].edges.push((node_type, target_idx));
    }
    idx
}

/// 将 arena 中的 BuildNode 转换为 Arc<ContentMatch> 节点（使用 OnceLock 一次性填充出边）。
fn assemble(arena: Vec<BuildNode>) -> Vec<Arc<ContentMatch>> {
    // 步骤 1：为每个 BuildNode 创建对应的 Arc<ContentMatch>（next 未初始化）
    let nodes: Vec<Arc<ContentMatch>> = arena
        .iter()
        .map(|b| Arc::new(ContentMatch::new(b.valid_end)))
        .collect();
    // 步骤 2：填充出边。此时每个 Arc 引用计数 = 1（仅 nodes 持有）。
    // OnceLock::set 只能成功一次，此处每节点只填充一次，安全。
    for (i, build_node) in arena.iter().enumerate() {
        let edges: Vec<MatchEdge> = build_node
            .edges
            .iter()
            .map(|(nt, target_idx)| MatchEdge {
                node_type: Arc::clone(nt),
                next: Arc::clone(&nodes[*target_idx]),
            })
            .collect();
        nodes[i]
            .next
            .set(edges)
            .expect("assemble: duplicate edge initialization");
    }
    nodes
}

fn build_dfa(nfa: &[Vec<NfaEdge>]) -> Arc<ContentMatch> {
    let initial_states = null_from(nfa, 0);
    let accept = nfa.len() - 1;
    let mut key_to_idx = HashMap::new();
    let mut arena = Vec::new();
    let root_idx = explore_into_arena(nfa, &initial_states, accept, &mut key_to_idx, &mut arena);
    let nodes = assemble(arena);
    Arc::clone(&nodes[root_idx])
}

fn check_for_dead_ends(start: &Arc<ContentMatch>, expr: &str) -> Result<(), String> {
    let mut work: Vec<Arc<ContentMatch>> = vec![Arc::clone(start)];
    let mut i = 0;
    while i < work.len() {
        let state = Arc::clone(&work[i]);
        i += 1;

        let mut dead = !state.valid_end;
        let mut nodes = Vec::new();

        for edge in state.next.get().map(|v| v.as_slice()).unwrap_or(&[]) {
            nodes.push(edge.node_type.name.clone());
            if dead && (!edge.node_type.is_text && !edge.node_type.has_required_attrs()) {
                dead = false;
            }
            if !work.iter().any(|w| Arc::ptr_eq(w, &edge.next)) {
                work.push(Arc::clone(&edge.next));
            }
        }

        if dead {
            return Err(format!(
                "Only non-generatable nodes ({}) in a required position (in content expression '{}')",
                nodes.join(", "),
                expr
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::schema::{NodeType, MarkType};
    use super::super::node::Node;
    use super::super::fragment::Fragment;
    use std::collections::BTreeMap;

    fn node_type(name: &str, is_block: bool) -> Arc<NodeType> {
        Arc::new(NodeType {
            name: name.into(), groups: vec![], is_block,
            is_text: name == "text", inline_content: false,
            mark_set: None, content_match: None,
        })
    }

    fn schema_with(types: &[(&str, bool)]) -> HashMap<String, Arc<NodeType>> {
        types.iter().map(|(name, is_block)| {
            (name.to_string(), node_type(name, *is_block))
        }).collect()
    }

    fn text_node(s: &str, nt: &Arc<NodeType>) -> Node {
        Node {
            node_type: Arc::clone(nt),
            attrs: BTreeMap::new(),
            content: Fragment::empty(),
            marks: vec![],
            text: Some(s.into()),
        }
    }

    fn block_node(nt: Arc<NodeType>) -> Node {
        Node {
            node_type: nt,
            attrs: BTreeMap::new(),
            content: Fragment::empty(),
            marks: vec![],
            text: None,
        }
    }

    // ── parse ────────────────────────────────────────────────

    #[test]
    fn parse_empty_expr() {
        let types = schema_with(&[]);
        let cm = ContentMatch::parse("", &types).unwrap();
        assert!(cm.valid_end);
        assert_eq!(cm.edge_count(), 0);
    }

    #[test]
    fn parse_single_no_panic() {
        // 覆盖 fix 1.1：DFA 构建不 panic
        let types = schema_with(&[("paragraph", true)]);
        let cm = ContentMatch::parse("paragraph", &types);
        assert!(cm.is_ok());
    }

    #[test]
    fn parse_plus_no_panic() {
        let types = schema_with(&[("paragraph", true)]);
        let cm = ContentMatch::parse("paragraph+", &types);
        assert!(cm.is_ok());
    }

    #[test]
    fn parse_star_valid_end() {
        let types = schema_with(&[("paragraph", true)]);
        let cm = ContentMatch::parse("paragraph*", &types).unwrap();
        assert!(cm.valid_end, "star expr should allow empty");
    }

    #[test]
    fn parse_range() {
        let types = schema_with(&[("heading", true)]);
        assert!(ContentMatch::parse("heading{1,3}", &types).is_ok());
    }

    #[test]
    fn parse_choice() {
        let types = schema_with(&[("p", true), ("h", true)]);
        assert!(ContentMatch::parse("p | h", &types).is_ok());
    }

    // ── match_type ───────────────────────────────────────────

    #[test]
    fn match_type_advances_state() {
        let types = schema_with(&[("p", true)]);
        let p_type = Arc::clone(types.get("p").unwrap());
        let cm = ContentMatch::parse("p+", &types).unwrap();
        let next = cm.match_type(&p_type);
        assert!(next.is_some(), "should match 'p'");
    }

    #[test]
    fn match_type_unknown_returns_none() {
        let types = schema_with(&[("p", true)]);
        let cm = ContentMatch::parse("p", &types).unwrap();
        let other = node_type("other", true);
        assert!(cm.match_type(&other).is_none());
    }

    // ── match_fragment ───────────────────────────────────────

    #[test]
    fn match_fragment_empty() {
        let types = schema_with(&[("p", true)]);
        let cm = ContentMatch::parse("p*", &types).unwrap();
        let frag = Fragment::empty();
        let result = cm.match_fragment(&frag, 0, 0);
        assert!(result.is_some());
        assert!(result.unwrap().valid_end);
    }

    #[test]
    fn match_fragment_one_node() {
        let types = schema_with(&[("p", true)]);
        let p_type = Arc::clone(types.get("p").unwrap());
        let cm = ContentMatch::parse("p+", &types).unwrap();
        let frag = Fragment::from_array(vec![block_node(p_type)]);
        let result = cm.match_fragment(&frag, 0, 1);
        assert!(result.is_some());
        assert!(result.unwrap().valid_end);
    }

    #[test]
    fn match_fragment_no_match() {
        let types = schema_with(&[("p", true), ("h", true)]);
        let h_type = Arc::clone(types.get("h").unwrap());
        let cm = ContentMatch::parse("p+", &types).unwrap();
        let frag = Fragment::from_array(vec![block_node(h_type)]);
        let result = cm.match_fragment(&frag, 0, 1);
        assert!(result.is_none());
    }

    // ── find_wrapping ────────────────────────────────────────

    #[test]
    fn find_wrapping_direct_match() {
        let types = schema_with(&[("p", true)]);
        let p_type = Arc::clone(types.get("p").unwrap());
        let cm = ContentMatch::parse("p+", &types).unwrap();
        let result = cm.find_wrapping(&p_type);
        assert_eq!(result, Some(vec![]));
    }
}
