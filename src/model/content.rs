use std::collections::HashMap;
use std::sync::Arc;

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
    /// 出边列表
    pub next: Vec<MatchEdge>,
    // wrap_cache 省略，可在需要时添加
}

impl ContentMatch {
    fn new(valid_end: bool) -> Self {
        Self {
            valid_end,
            next: Vec::new(),
        }
    }

    /// 空匹配——叶子节点（无子节点）使用。
    pub fn empty() -> Arc<ContentMatch> {
        Arc::new(ContentMatch::new(true))
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
        for edge in &self.next {
            if Arc::ptr_eq(&edge.node_type, node_type) {
                return Some(Arc::clone(&edge.next));
            }
        }
        None
    }

    /// 此状态的子节点是否为行内内容。
    pub fn inline_content(&self) -> bool {
        !self.next.is_empty() && self.next[0].node_type.is_inline()
    }

    /// 获取此状态第一个可自动生成的默认节点类型。
    pub fn default_type(&self) -> Option<Arc<NodeType>> {
        for edge in &self.next {
            if !edge.node_type.is_text && !edge.node_type.has_required_attrs() {
                return Some(Arc::clone(&edge.node_type));
            }
        }
        None
    }

    /// 检查两个 ContentMatch 是否有兼容的出边。
    pub fn compatible(&self, other: &ContentMatch) -> bool {
        for a in &self.next {
            for b in &other.next {
                if Arc::ptr_eq(&a.node_type, &b.node_type) {
                    return true;
                }
            }
        }
        false
    }

    /// 出边数量。
    pub fn edge_count(&self) -> usize {
        self.next.len()
    }

    /// 获取第 n 条出边。
    pub fn edge(&self, n: usize) -> Option<&MatchEdge> {
        self.next.get(n)
    }

    /// 查找使目标节点类型合法的包裹节点类型序列。
    pub fn find_wrapping(&self, target: &Arc<NodeType>) -> Option<Vec<Arc<NodeType>>> {
        // BFS 搜索
        struct Active {
            content_match: Arc<ContentMatch>,
            node_type: Option<Arc<NodeType>>,
            via: Option<Box<Active>>,
        }

        let mut seen = HashMap::new();
        let mut queue = vec![Active {
            content_match: Arc::new(ContentMatch {
                valid_end: self.valid_end,
                next: self.next.clone(),
            }),
            node_type: None,
            via: None,
        }];

        while let Some(current) = queue.first() {
            // 检查当前状态是否能直接匹配目标
            if current.content_match.match_type(target).is_some() {
                let mut result = Vec::new();
                let mut obj = &queue[0];
                while let Some(ref nt) = obj.node_type {
                    result.push(Arc::clone(nt));
                    match &obj.via {
                        Some(v) => obj = v.as_ref(),
                        None => break,
                    }
                }
                result.reverse();
                return Some(result);
            }

            let current = queue.remove(0);
            for edge in &current.content_match.next {
                let nt = &edge.node_type;
                if !nt.is_leaf()
                    && !nt.has_required_attrs()
                    && !seen.contains_key(&nt.name)
                    && (current.node_type.is_none() || edge.next.valid_end)
                {
                    seen.insert(nt.name.clone(), true);
                    if let Some(ref cm) = nt.content_match {
                        queue.push(Active {
                            content_match: Arc::clone(cm),
                            node_type: Some(Arc::clone(nt)),
                            via: Some(Box::new(Active {
                                content_match: current.content_match.clone(),
                                node_type: current.node_type.clone(),
                                via: None, // 简化：不保留完整链
                            })),
                        });
                    }
                }
            }

            if queue.is_empty() {
                break;
            }
        }
        None
    }

    // match_fragment 和 fill_before 依赖 Fragment，后续移植 fragment.rs 后补全。
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
                // 将特殊字符拆分为独立 token
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
            None // 无上限
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
    // 先查直接名称
    if let Some(nt) = stream.node_types.get(name) {
        return Ok(vec![Arc::clone(nt)]);
    }
    // 再查分组
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

    // 检查是否是标识符（非特殊字符）
    if token.chars().all(|c| c.is_alphanumeric() || c == '_') {
        let types = resolve_name(stream, &token)?;
        stream.pos += 1;

        // 检查 inline/block 一致性
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
    to: usize, // 0 = 未连接占位
}

fn build_nfa(expr: &Expr) -> Vec<Vec<NfaEdge>> {
    let mut states: Vec<Vec<NfaEdge>> = vec![vec![]]; // state 0 = 起始

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

    // 返回未连接的边的 (state_idx, edge_idx) 列表
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
                // epsilon from -> loop
                add_edge(states, from, loop_state, None);
                let inner_edges = compile(inner, loop_state, states);
                connect(states, &inner_edges, loop_state);
                // 一条未连接的 epsilon 边从 loop_state 出去
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
                        // 无上限
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

    let end = new_state(&mut states);
    let dangling = compile(expr, 0, &mut states);
    connect(&mut states, &dangling, end);
    states
}

// ============================================================
// DFA 构建（NFA → DFA 子集构造法）
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

fn build_dfa(nfa: &[Vec<NfaEdge>]) -> Arc<ContentMatch> {
    let mut labeled: HashMap<String, Arc<ContentMatch>> = HashMap::new();
    explore(nfa, &null_from(nfa, 0), &mut labeled, nfa.len() - 1)
}

fn states_key(states: &[usize]) -> String {
    states
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn explore(
    nfa: &[Vec<NfaEdge>],
    states: &[usize],
    labeled: &mut HashMap<String, Arc<ContentMatch>>,
    accept: usize,
) -> Arc<ContentMatch> {
    // 收集所有从当前状态集可达的 (NodeType, 目标状态集)
    let mut out: Vec<(Arc<NodeType>, Vec<usize>)> = Vec::new();
    for &node in states {
        for edge in &nfa[node] {
            if let Some(ref term) = edge.term {
                let mut found = false;
                for entry in &mut out {
                    if Arc::ptr_eq(&entry.0, term) {
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

    let key = states_key(states);
    let valid_end = states.contains(&accept);
    let state = Arc::new(ContentMatch::new(valid_end));
    labeled.insert(key, Arc::clone(&state));

    // 构建出边——需要 unsafe 来修改 Arc 内部的 next
    // 由于 ContentMatch 在构建阶段需要可变，用 Arc::get_mut
    let state_mut = Arc::into_inner(state).unwrap();
    let mut state_mut = state_mut;

    for (node_type, mut target_states) in out {
        target_states.sort_unstable_by(|a, b| b.cmp(a));
        let target_key = states_key(&target_states);
        let next = if let Some(existing) = labeled.get(&target_key) {
            Arc::clone(existing)
        } else {
            explore(nfa, &target_states, labeled, accept)
        };
        state_mut.next.push(MatchEdge { node_type, next });
    }

    let result = Arc::new(state_mut);
    labeled.insert(states_key(states), Arc::clone(&result));
    result
}

fn check_for_dead_ends(start: &Arc<ContentMatch>, expr: &str) -> Result<(), String> {
    let mut work: Vec<Arc<ContentMatch>> = vec![Arc::clone(start)];
    let mut i = 0;
    while i < work.len() {
        let state = Arc::clone(&work[i]);
        i += 1;

        let mut dead = !state.valid_end;
        let mut nodes = Vec::new();

        for edge in &state.next {
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
