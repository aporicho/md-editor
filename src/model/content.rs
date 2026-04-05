// src/model/content.rs
//
// ContentMatch：节点内容表达式的 DFA 匹配状态机。
// 对应 TypeScript 的 content.ts。
//
// 注意：以下方法为占位实现，依赖 Fragment，将在 Phase 3 补全：
//   - match_fragment(&self, frag: &Fragment, start: usize, end: usize) -> Option<Arc<ContentMatch>>
//   - fill_before(&self, after: &Fragment, to_end: bool, start_index: usize) -> Option<Fragment>
//   - find_wrapping(&self, target_type_name: &str) -> Option<Vec<String>>
//
#![allow(dead_code)] // TODO: Phase 3 完成后移除
#[allow(unused_imports)]
use std::sync::Arc;
#[allow(unused_imports)]
use indexmap::IndexMap;

#[allow(unused_imports)]
use crate::model::errors::{RangeError, SyntaxError};

/// An expression node produced by the content expression parser.
#[derive(Debug, Clone)]
#[allow(dead_code)]
enum Expr {
    Choice(Vec<Expr>),
    Seq(Vec<Expr>),
    Plus(Box<Expr>),
    Star(Box<Expr>),
    Opt(Box<Expr>),
    Range { min: usize, max: Option<usize>, expr: Box<Expr> },
    /// A resolved node type name (plus inline flag for the DFA builder).
    Name { type_name: String, is_inline: bool },
}

/// Tokenizer for content expressions such as `"paragraph horizontal_rule paragraph"`.
#[allow(dead_code)]
struct TokenStream<'a> {
    string: &'a str,
    /// Map from node type name to is_inline flag.
    node_types: &'a IndexMap<String, bool>,
    tokens: Vec<String>,
    pos: usize,
}

#[allow(dead_code)]
impl<'a> TokenStream<'a> {
    fn new(
        string: &'a str,
        node_types: &'a IndexMap<String, bool>,
    ) -> Self {
        // Split on whitespace boundaries and non-word characters (mirrors TS: /\s*(?=\b|\W|$)/)
        let mut tokens: Vec<String> = vec![];
        let chars: Vec<char> = string.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i].is_whitespace() {
                i += 1;
                continue;
            }
            if chars[i].is_alphanumeric() || chars[i] == '_' {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                tokens.push(chars[start..i].iter().collect());
            } else {
                tokens.push(chars[i].to_string());
                i += 1;
            }
        }
        TokenStream { string, node_types, tokens, pos: 0 }
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

    fn err(&self, msg: &str) -> SyntaxError {
        SyntaxError(format!("{} (in content expression '{}')", msg, self.string))
    }
}

#[allow(dead_code)]
fn parse_expr(
    stream: &mut TokenStream,
    groups: &IndexMap<String, Vec<String>>,
) -> Result<Expr, SyntaxError> {
    let mut exprs = vec![parse_expr_seq(stream, groups)?];
    while stream.eat("|") {
        exprs.push(parse_expr_seq(stream, groups)?);
    }
    Ok(if exprs.len() == 1 { exprs.remove(0) } else { Expr::Choice(exprs) })
}

#[allow(dead_code)]
fn parse_expr_seq(
    stream: &mut TokenStream,
    groups: &IndexMap<String, Vec<String>>,
) -> Result<Expr, SyntaxError> {
    let mut exprs = vec![parse_expr_subscript(stream, groups)?];
    while stream.next().is_some_and(|t| t != ")" && t != "|") {
        exprs.push(parse_expr_subscript(stream, groups)?);
    }
    Ok(if exprs.len() == 1 { exprs.remove(0) } else { Expr::Seq(exprs) })
}

#[allow(dead_code)]
fn parse_expr_subscript(
    stream: &mut TokenStream,
    groups: &IndexMap<String, Vec<String>>,
) -> Result<Expr, SyntaxError> {
    let mut expr = parse_expr_atom(stream, groups)?;
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

#[allow(dead_code)]
fn parse_num(stream: &mut TokenStream) -> Result<usize, SyntaxError> {
    let tok = stream
        .next()
        .ok_or_else(|| stream.err("Expected number, got end of expression"))?
        .to_string();
    let n = tok
        .parse::<usize>()
        .map_err(|_| stream.err(&format!("Expected number, got '{}'", tok)))?;
    stream.pos += 1;
    Ok(n)
}

#[allow(dead_code)]
fn parse_expr_range(stream: &mut TokenStream, expr: Expr) -> Result<Expr, SyntaxError> {
    let min = parse_num(stream)?;
    let max = if stream.eat(",") {
        if stream.next() == Some("}") {
            None // open range: {n,}
        } else {
            Some(parse_num(stream)?)
        }
    } else {
        Some(min)
    };
    if !stream.eat("}") {
        return Err(stream.err("Unclosed braced range"));
    }
    Ok(Expr::Range { min, max, expr: Box::new(expr) })
}

/// Resolves a name token to (type_name, is_inline) pairs.
/// The name may be a direct type name or a group name.
#[allow(dead_code)]
fn resolve_name(
    stream: &TokenStream,
    name: &str,
    groups: &IndexMap<String, Vec<String>>,
) -> Result<Vec<(String, bool)>, SyntaxError> {
    // Direct type name
    if let Some(&is_inline) = stream.node_types.get(name) {
        return Ok(vec![(name.to_string(), is_inline)]);
    }
    // Group name
    if let Some(members) = groups.get(name) {
        if members.is_empty() {
            return Err(stream.err(&format!("No node type or group '{}' found", name)));
        }
        return Ok(members
            .iter()
            .map(|n| (n.clone(), *stream.node_types.get(n).unwrap_or(&false)))
            .collect());
    }
    Err(stream.err(&format!("No node type or group '{}' found", name)))
}

#[allow(dead_code)]
fn parse_expr_atom(
    stream: &mut TokenStream,
    groups: &IndexMap<String, Vec<String>>,
) -> Result<Expr, SyntaxError> {
    if stream.eat("(") {
        let expr = parse_expr(stream, groups)?;
        if !stream.eat(")") {
            return Err(stream.err("Missing closing paren"));
        }
        return Ok(expr);
    }
    let tok = stream
        .next()
        .ok_or_else(|| stream.err("Unexpected end of expression"))?
        .to_string();
    // Word token
    if tok.chars().all(|c| c.is_alphanumeric() || c == '_') {
        let resolved = resolve_name(stream, &tok, groups)?;
        stream.pos += 1;
        // Check mixing inline/block (mirror TS behavior)
        let first_inline = resolved[0].1;
        for (_, is_inline) in &resolved {
            if *is_inline != first_inline {
                return Err(stream.err("Mixing inline and block content"));
            }
        }
        let exprs: Vec<Expr> = resolved
            .into_iter()
            .map(|(type_name, is_inline)| Expr::Name { type_name, is_inline })
            .collect();
        Ok(if exprs.len() == 1 {
            exprs.into_iter().next().unwrap()
        } else {
            Expr::Choice(exprs)
        })
    } else {
        Err(stream.err(&format!("Unexpected token '{}'", tok)))
    }
}

#[derive(Debug, Clone)]
struct NfaEdge {
    /// None = ε-edge (epsilon/null transition)
    term: Option<String>,
    /// Target state index. usize::MAX means "dangling" (not yet connected).
    to: usize,
}

fn nfa_new_state(nfa: &mut Vec<Vec<NfaEdge>>) -> usize {
    nfa.push(vec![]);
    nfa.len() - 1
}

fn nfa_connect(nfa: &mut [Vec<NfaEdge>], dangling: &[(usize, usize)], to: usize) {
    for &(state, edge_idx) in dangling {
        nfa[state][edge_idx].to = to;
    }
}

/// Compile expr from `from` state.
/// Returns list of (state, edge_index) for "dangling" edges (their `to` = usize::MAX).
fn nfa_compile(
    expr: &Expr,
    from: usize,
    nfa: &mut Vec<Vec<NfaEdge>>,
) -> Vec<(usize, usize)> {
    match expr {
        Expr::Choice(exprs) => {
            exprs
                .iter()
                .flat_map(|e| nfa_compile(e, from, nfa))
                .collect()
        }
        Expr::Seq(exprs) => {
            let mut cur = from;
            for (i, e) in exprs.iter().enumerate() {
                let dangling = nfa_compile(e, cur, nfa);
                if i == exprs.len() - 1 {
                    return dangling;
                }
                let next = nfa_new_state(nfa);
                nfa_connect(nfa, &dangling, next);
                cur = next;
            }
            // Seq with 0 exprs shouldn't happen (parser guarantees >= 1)
            vec![]
        }
        Expr::Star(inner) => {
            let loop_state = nfa_new_state(nfa);
            // ε from `from` to loop_state
            nfa[from].push(NfaEdge { term: None, to: loop_state });
            let dangling_inner = nfa_compile(inner, loop_state, nfa);
            nfa_connect(nfa, &dangling_inner, loop_state);
            // Exit ε-edge from loop_state
            let exit_idx = nfa[loop_state].len();
            nfa[loop_state].push(NfaEdge { term: None, to: usize::MAX });
            vec![(loop_state, exit_idx)]
        }
        Expr::Plus(inner) => {
            let loop_state = nfa_new_state(nfa);
            let first = nfa_compile(inner, from, nfa);
            nfa_connect(nfa, &first, loop_state);
            let second = nfa_compile(inner, loop_state, nfa);
            nfa_connect(nfa, &second, loop_state);
            // Exit ε-edge from loop_state
            let exit_idx = nfa[loop_state].len();
            nfa[loop_state].push(NfaEdge { term: None, to: usize::MAX });
            vec![(loop_state, exit_idx)]
        }
        Expr::Opt(inner) => {
            // ε-skip edge from `from`
            let skip_idx = nfa[from].len();
            nfa[from].push(NfaEdge { term: None, to: usize::MAX });
            let mut dangling = vec![(from, skip_idx)];
            dangling.extend(nfa_compile(inner, from, nfa));
            dangling
        }
        Expr::Range { min, max, expr: inner } => {
            let mut cur = from;
            // Mandatory `min` repetitions
            for _ in 0..*min {
                let next = nfa_new_state(nfa);
                let d = nfa_compile(inner, cur, nfa);
                nfa_connect(nfa, &d, next);
                cur = next;
            }
            match max {
                None => {
                    // Open range {n,}: loop back after min
                    let d = nfa_compile(inner, cur, nfa);
                    nfa_connect(nfa, &d, cur);
                    let exit_idx = nfa[cur].len();
                    nfa[cur].push(NfaEdge { term: None, to: usize::MAX });
                    vec![(cur, exit_idx)]
                }
                Some(m) => {
                    let mut dangling: Vec<(usize, usize)> = vec![];
                    for _ in *min..*m {
                        let next = nfa_new_state(nfa);
                        let skip_idx = nfa[cur].len();
                        nfa[cur].push(NfaEdge { term: None, to: usize::MAX });
                        dangling.push((cur, skip_idx));
                        let d = nfa_compile(inner, cur, nfa);
                        nfa_connect(nfa, &d, next);
                        cur = next;
                    }
                    let exit_idx = nfa[cur].len();
                    nfa[cur].push(NfaEdge { term: None, to: usize::MAX });
                    dangling.push((cur, exit_idx));
                    dangling
                }
            }
        }
        Expr::Name { type_name, .. } => {
            let edge_idx = nfa[from].len();
            nfa[from].push(NfaEdge {
                term: Some(type_name.clone()),
                to: usize::MAX,
            });
            vec![(from, edge_idx)]
        }
    }
}

/// Build an NFA from a parsed Expr.
///
/// Construct an NFA from an expression as returned by the parser. The
/// NFA is represented as an array of states, which are themselves
/// arrays of edges, which are `{term, to}` objects. The first state is
/// the entry state and the last node is the success state.
///
/// Note that unlike typical NFAs, the edge ordering in this one is
/// significant, in that it is used to construct filler content when necessary.
fn build_nfa(expr: &Expr) -> Vec<Vec<NfaEdge>> {
    let mut nfa: Vec<Vec<NfaEdge>> = vec![vec![]]; // state 0 = entry
    // Compile first so intermediate states get their indices before the success state.
    let dangling = nfa_compile(expr, 0, &mut nfa);
    // Create success state last: it will be at index nfa.len() - 1 after push.
    let success = nfa_new_state(&mut nfa);
    nfa_connect(&mut nfa, &dangling, success);
    nfa
}

fn null_from_scan(nfa: &[Vec<NfaEdge>], node: usize, result: &mut Vec<usize>) {
    let edges = &nfa[node];
    // Collapse single-ε chains (TS omits these to avoid needless duplicates)
    if edges.len() == 1 && edges[0].term.is_none() {
        null_from_scan(nfa, edges[0].to, result);
        return;
    }
    if result.contains(&node) {
        return;
    }
    result.push(node);
    for e in edges {
        if e.term.is_none() && e.to != usize::MAX && !result.contains(&e.to) {
            null_from_scan(nfa, e.to, result);
        }
    }
}

/// Returns the set of NFA states reachable from `state` via ε-edges only,
/// sorted in descending order.
///
/// Get the set of nodes reachable by null edges from `node`. Omit
/// nodes with only a single null-out-edge, since they may lead to
/// needless duplicated nodes.
fn null_from(nfa: &[Vec<NfaEdge>], state: usize) -> Vec<usize> {
    let mut result: Vec<usize> = vec![];
    null_from_scan(nfa, state, &mut result);
    result.sort_by(|a, b| b.cmp(a)); // descending
    result
}

/// Intermediate indexed DFA state, used before converting to Arc<ContentMatch>.
struct DfaState {
    valid_end: bool,
    inline_content: bool,
    /// Edges: (type_name, target_state_index).
    edges: Vec<(String, usize)>,
}

/// Build a DFA from an NFA using subset construction, producing an indexed state table.
///
/// Returns (states, start_index) where `states[start_index]` is the initial state.
fn build_dfa_indexed(
    nfa: &[Vec<NfaEdge>],
    node_types: &IndexMap<String, bool>,
) -> Vec<DfaState> {
    use std::collections::HashMap;
    let accept_state = nfa.len() - 1;

    // Map from sorted state-set key → index in `states` vec
    let mut labeled: HashMap<String, usize> = HashMap::new();
    let mut states: Vec<DfaState> = Vec::new();

    fn key(ss: &[usize]) -> String {
        let mut v = ss.to_vec();
        v.sort_by(|a, b| b.cmp(a));
        v.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(",")
    }

    fn explore(
        state_set: Vec<usize>,
        nfa: &[Vec<NfaEdge>],
        accept_state: usize,
        node_types: &IndexMap<String, bool>,
        labeled: &mut HashMap<String, usize>,
        states: &mut Vec<DfaState>,
    ) -> usize {
        let k = key(&state_set);
        if let Some(&idx) = labeled.get(&k) {
            return idx;
        }

        // Collect outgoing edges grouped by term
        let mut out: Vec<(String, Vec<usize>)> = vec![];
        for &node in &state_set {
            for edge in &nfa[node] {
                if let Some(term) = &edge.term {
                    let set_idx = if let Some(pos) = out.iter().position(|(t, _)| t == term) {
                        pos
                    } else {
                        out.push((term.clone(), vec![]));
                        out.len() - 1
                    };
                    for ns in null_from(nfa, edge.to) {
                        if !out[set_idx].1.contains(&ns) {
                            out[set_idx].1.push(ns);
                        }
                    }
                }
            }
        }

        let valid_end = state_set.contains(&accept_state);
        let inline_content = out
            .first()
            .and_then(|(t, _)| node_types.get(t).copied())
            .unwrap_or(false);

        // Register before recursing to handle cycles
        let idx = states.len();
        labeled.insert(k, idx);
        states.push(DfaState { valid_end, inline_content, edges: vec![] });

        // Build edges by recursing
        let mut edges: Vec<(String, usize)> = vec![];
        for (term, mut target) in out {
            target.sort_by(|a, b| b.cmp(a));
            let child_idx = explore(target, nfa, accept_state, node_types, labeled, states);
            edges.push((term, child_idx));
        }
        states[idx].edges = edges;

        idx
    }

    let start = null_from(nfa, 0);
    explore(start, nfa, accept_state, node_types, &mut labeled, &mut states);
    states
}

/// Convert an indexed DFA state table to Arc<ContentMatch> using two-phase construction.
///
/// Phase 1: create all Arcs with uninitialized edges (refcount = 1 each).
/// Phase 2: fill edges via OnceLock::set — safe even when refcount > 1 due to cycles.
fn build_dfa(
    nfa: &[Vec<NfaEdge>],
    node_types: &IndexMap<String, bool>,
) -> Arc<ContentMatch> {
    let states = build_dfa_indexed(nfa, node_types);

    // Phase 1: allocate all Arcs with empty OnceLock edges
    let arcs: Vec<Arc<ContentMatch>> = states
        .iter()
        .map(|s| {
            Arc::new(ContentMatch {
                valid_end: s.valid_end,
                inline_content: s.inline_content,
                edges: std::sync::OnceLock::new(),
                wrap_cache: std::sync::Mutex::new(std::collections::HashMap::new()),
            })
        })
        .collect();

    // Phase 2: fill edges via OnceLock::set (unlike Arc::get_mut, OnceLock::set
    // is safe regardless of refcount — it only requires the lock is unset)
    for (i, state) in states.iter().enumerate() {
        let edges: Vec<(String, Arc<ContentMatch>)> = state
            .edges
            .iter()
            .map(|(name, idx)| (name.clone(), arcs[*idx].clone()))
            .collect();
        arcs[i].edges.set(edges).ok(); // panics only if set twice, which cannot happen here
    }

    arcs.into_iter().next().unwrap()
}

/// Check that every non-valid-end state has at least one generatable outgoing type.
///
/// A generatable type is one that is not "text" and has no required attrs.
fn check_for_dead_ends(
    start: &Arc<ContentMatch>,
    expr_str: &str,
    has_required: &IndexMap<String, bool>,
) -> Result<(), SyntaxError> {
    let mut work: Vec<Arc<ContentMatch>> = vec![start.clone()];
    let mut i = 0;
    while i < work.len() {
        let state = work[i].clone();
        i += 1;
        let mut dead = !state.valid_end;
        let mut node_names: Vec<String> = vec![];
        let state_edges = state.edges.get().map(|v| v.as_slice()).unwrap_or(&[]);
        for (name, next) in state_edges {
            node_names.push(name.clone());
            let is_text = name == "text";
            let required = has_required.get(name).copied().unwrap_or(false);
            if !(is_text || required) {
                dead = false;
            }
            if !work.iter().any(|w| Arc::ptr_eq(w, next)) {
                work.push(next.clone());
            }
        }
        if dead && !node_names.is_empty() {
            return Err(SyntaxError(format!(
                "Only non-generatable nodes ({}) in a required position (in content expression '{}')",
                node_names.join(", "),
                expr_str
            )));
        }
    }
    Ok(())
}

/// Represents a match state of a node type's content expression.
///
/// Instances of this class represent a match state of a node type's
/// content expression, and can be used to find out whether further
/// content matches here, and whether a given position is a valid end
/// of the node.
pub struct ContentMatch {
    /// True when this match state represents a valid end of the node.
    pub valid_end: bool,
    /// True when this match position expects inline content.
    pub inline_content: bool,
    /// Outgoing edges: (type_name, next_state).
    /// OnceLock enables safe two-phase construction: set once, then read-only.
    edges: std::sync::OnceLock<Vec<(String, Arc<ContentMatch>)>>,
    /// Cache for find_wrapping results. Mutex for Send+Sync compatibility.
    wrap_cache: std::sync::Mutex<std::collections::HashMap<String, Option<Vec<String>>>>,
}

impl ContentMatch {
    /// The empty content match (valid end, no edges). Used for leaf nodes.
    pub fn empty() -> Arc<ContentMatch> {
        let cm = Arc::new(ContentMatch {
            valid_end: true,
            inline_content: false,
            edges: std::sync::OnceLock::new(),
            wrap_cache: std::sync::Mutex::new(std::collections::HashMap::new()),
        });
        let _ = cm.edges.set(vec![]);
        cm
    }

    /// Match a node type by name, returning the next match state if successful.
    ///
    /// Match a node type, returning a match after that node if successful.
    pub fn match_type(&self, type_name: &str) -> Option<Arc<ContentMatch>> {
        self.edges
            .get()
            .and_then(|edges| edges.iter().find(|(name, _)| name == type_name))
            .map(|(_, next)| next.clone())
    }

    /// The number of outgoing edges this node has in the finite automaton
    /// that describes the content expression.
    pub fn edge_count(&self) -> usize {
        self.edges.get().map_or(0, |e| e.len())
    }

    /// Get the nth outgoing edge from this node in the finite automaton
    /// that describes the content expression.
    ///
    /// Returns `(type_name, next_state)`, or `Err(RangeError)` if n is out of range.
    pub fn edge(&self, n: usize) -> Result<(String, Arc<ContentMatch>), RangeError> {
        self.edges
            .get()
            .and_then(|edges| edges.get(n))
            .map(|(name, next)| (name.clone(), next.clone()))
            .ok_or_else(|| RangeError(format!("There's no {}th edge in this content match", n)))
    }

    /// Returns true if this match and `other` share at least one outgoing type name.
    ///
    /// Indicates whether this node allows some of the same content as the given match.
    pub fn compatible(&self, other: &ContentMatch) -> bool {
        let my_edges = self.edges.get().map(|v| v.as_slice()).unwrap_or(&[]);
        let other_edges = other.edges.get().map(|v| v.as_slice()).unwrap_or(&[]);
        for (name, _) in my_edges {
            for (other_name, _) in other_edges {
                if name == other_name {
                    return true;
                }
            }
        }
        false
    }

    /// Parse a content expression string into a DFA.
    ///
    /// `node_types`: map from type name to `is_inline` flag.
    /// `groups`: map from group name to list of type names in that group.
    /// `has_required_attrs`: map from type name to whether it has required attrs (no default).
    pub fn parse(
        expr: &str,
        node_types: &IndexMap<String, bool>,
        groups: &IndexMap<String, Vec<String>>,
        has_required_attrs: &IndexMap<String, bool>,
    ) -> Result<Arc<ContentMatch>, SyntaxError> {
        if expr.trim().is_empty() {
            return Ok(ContentMatch::empty());
        }
        let mut stream = TokenStream::new(expr, node_types);
        let ast = parse_expr(&mut stream, groups)?;
        if stream.next().is_some() {
            return Err(stream.err("Unexpected trailing text"));
        }
        let nfa = build_nfa(&ast);
        let dfa = build_dfa(&nfa, node_types);
        check_for_dead_ends(&dfa, expr, has_required_attrs)?;
        Ok(dfa)
    }

    /// Get the name of the first outgoing node type at this match position that can be generated.
    ///
    /// Get the first matching node type at this match position that can
    /// be generated. A type is generatable if it is not "text".
    /// Returns the type name, or None if no generatable type exists.
    pub fn default_type(&self) -> Option<String> {
        self.edges
            .get()
            .and_then(|edges| edges.iter().find(|(name, _)| name != "text"))
            .map(|(name, _)| name.clone())
    }

    // ---- Phase 3 stubs (依赖 Fragment，届时补全) ----

    /// Try to match a fragment against this content match state.
    ///
    /// Returns the resulting match state if successful, or `None` if
    /// the fragment does not fit.
    pub fn match_fragment(
        self: &Arc<Self>,
        frag: &crate::model::fragment::Fragment,
        start: usize,
        end: usize,
    ) -> Option<Arc<ContentMatch>> {
        let mut cur = self.clone();
        for i in start..end {
            let child = frag.child(i).ok()?;
            cur = cur.match_type(&child.node_type.name)?;
        }
        Some(cur)
    }

    /// Try to match the given fragment, and if that fails, see if it can
    /// be made to match by inserting nodes in front of it.
    ///
    /// Returns a fragment of inserted nodes (may be empty if nothing needs
    /// to be inserted). When `to_end` is true, only returns a fragment if
    /// the resulting match goes to the end of the content expression.
    pub fn fill_before(
        self: &Arc<Self>,
        schema: &Arc<crate::model::schema::Schema>,
        after: &crate::model::fragment::Fragment,
        to_end: bool,
        start_index: usize,
    ) -> Option<crate::model::fragment::Fragment> {
        todo!("fill_before: implement in Phase 3.4-b")
    }

    /// Find a set of wrapping node type names that would allow a node of the given type
    /// to appear at this position.
    ///
    /// NOTE: stub — requires NodeType.create_and_fill (Phase 3). Will be implemented in Phase 3.
    #[allow(unused)]
    pub fn find_wrapping_stub(&self) -> ! {
        todo!("find_wrapping requires NodeType.create_and_fill — implement in Phase 3")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state(valid_end: bool, edges_input: Vec<(&str, Arc<ContentMatch>)>) -> Arc<ContentMatch> {
        let cm = Arc::new(ContentMatch {
            valid_end,
            inline_content: false,
            edges: std::sync::OnceLock::new(),
            wrap_cache: std::sync::Mutex::new(std::collections::HashMap::new()),
        });
        let _ = cm.edges.set(
            edges_input.into_iter().map(|(n, s)| (n.to_string(), s)).collect()
        );
        cm
    }

    // state_a --"p"--> state_b (valid_end=true)
    fn two_state_dfa() -> Arc<ContentMatch> {
        let state_b = make_state(true, vec![]);
        make_state(false, vec![("p", state_b)])
    }

    #[test]
    fn match_type_hit() {
        let m = two_state_dfa();
        let next = m.match_type("p");
        assert!(next.is_some());
        assert!(next.unwrap().valid_end);
    }

    #[test]
    fn match_type_miss() {
        let m = two_state_dfa();
        assert!(m.match_type("h1").is_none());
    }

    #[test]
    fn empty_valid_end() {
        let m = ContentMatch::empty();
        assert!(m.valid_end);
        assert_eq!(m.edge_count(), 0);
    }

    #[test]
    fn edge_count_and_edge() {
        let state_b = make_state(true, vec![]);
        let state_c = make_state(false, vec![]);
        let m = make_state(false, vec![("p", state_b), ("h1", state_c)]);
        assert_eq!(m.edge_count(), 2);
        let (name, _) = m.edge(0).unwrap();
        assert_eq!(name, "p");
        let (name, _) = m.edge(1).unwrap();
        assert_eq!(name, "h1");
    }

    #[test]
    fn edge_out_of_range() {
        let m = ContentMatch::empty();
        assert!(m.edge(0).is_err());
    }

    #[test]
    fn compatible_same_type() {
        let state_b = make_state(true, vec![]);
        let m1 = make_state(false, vec![("p", state_b.clone())]);
        let m2 = make_state(false, vec![("p", state_b.clone())]);
        assert!(m1.compatible(&m2));
    }

    #[test]
    fn compatible_different_types() {
        let sb = make_state(true, vec![]);
        let sc = make_state(true, vec![]);
        let m1 = make_state(false, vec![("p", sb)]);
        let m2 = make_state(false, vec![("h1", sc)]);
        assert!(!m1.compatible(&m2));
    }

    // ---- helpers for parse tests ----

    fn make_node_types(names: &[(&str, bool)]) -> IndexMap<String, bool> {
        names.iter().map(|(n, b)| (n.to_string(), *b)).collect()
    }

    fn make_groups(g: &[(&str, &[&str])]) -> IndexMap<String, Vec<String>> {
        g.iter()
            .map(|(group, members)| {
                (group.to_string(), members.iter().map(|m| m.to_string()).collect())
            })
            .collect()
    }

    fn make_required(names: &[&str]) -> IndexMap<String, bool> {
        names.iter().map(|n| (n.to_string(), false)).collect()
    }

    fn node_types_basic() -> IndexMap<String, bool> {
        make_node_types(&[
            ("paragraph", false),
            ("heading", false),
            ("horizontal_rule", false),
            ("image", true),
            ("text", true),
            ("hard_break", true),
            ("code_block", false),
        ])
    }

    fn groups_basic() -> IndexMap<String, Vec<String>> {
        make_groups(&[
            ("block", &["paragraph", "heading", "horizontal_rule", "code_block"]),
            ("inline", &["image", "text", "hard_break"]),
        ])
    }

    fn required_basic() -> IndexMap<String, bool> {
        make_required(&["paragraph", "heading", "horizontal_rule", "image", "text", "hard_break", "code_block"])
    }

    fn parse_ok(expr: &str) -> Arc<ContentMatch> {
        ContentMatch::parse(expr, &node_types_basic(), &groups_basic(), &required_basic())
            .expect("parse should succeed")
    }

    fn matches(m: &Arc<ContentMatch>, types: &[&str]) -> bool {
        let mut cur: Option<Arc<ContentMatch>> = Some(m.clone());
        for &t in types {
            cur = cur.and_then(|state| state.match_type(t));
        }
        cur.map_or(false, |s| s.valid_end)
    }

    #[test]
    fn parse_empty_expr() {
        let m = parse_ok("");
        assert!(m.valid_end);
        assert_eq!(m.edge_count(), 0);
        assert!(m.match_type("paragraph").is_none());
    }

    #[test]
    fn parse_star() {
        let m = parse_ok("image*");
        assert!(matches(&m, &[]));
        assert!(matches(&m, &["image"]));
        assert!(matches(&m, &["image", "image", "image", "image"]));
        assert!(!matches(&m, &["image", "text"]));
    }

    #[test]
    fn parse_plus() {
        let m = parse_ok("paragraph+");
        assert!(!matches(&m, &[]));
        assert!(matches(&m, &["paragraph"]));
        assert!(matches(&m, &["paragraph", "paragraph"]));
    }

    #[test]
    fn parse_opt() {
        let m = parse_ok("image?");
        assert!(matches(&m, &[]));
        assert!(matches(&m, &["image"]));
        assert!(!matches(&m, &["image", "image"]));
    }

    #[test]
    fn parse_seq() {
        let m = parse_ok("paragraph horizontal_rule paragraph");
        assert!(matches(&m, &["paragraph", "horizontal_rule", "paragraph"]));
        assert!(!matches(&m, &["paragraph", "horizontal_rule"]));
        assert!(!matches(&m, &["horizontal_rule", "paragraph", "horizontal_rule"]));
    }

    #[test]
    fn parse_choice() {
        let m = parse_ok("(paragraph | heading)");
        assert!(matches(&m, &["paragraph"]));
        assert!(matches(&m, &["heading"]));
        assert!(!matches(&m, &["image"]));
    }

    #[test]
    fn parse_group() {
        let m = parse_ok("inline*");
        assert!(matches(&m, &["image", "text"]));
        assert!(!matches(&m, &["paragraph"]));
    }

    #[test]
    fn parse_count_exact() {
        let m = parse_ok("hard_break{2}");
        assert!(!matches(&m, &["hard_break"]));
        assert!(matches(&m, &["hard_break", "hard_break"]));
        assert!(!matches(&m, &["hard_break", "hard_break", "hard_break"]));
    }

    #[test]
    fn parse_count_range() {
        let m = parse_ok("hard_break{2, 4}");
        assert!(!matches(&m, &["hard_break"]));
        assert!(matches(&m, &["hard_break", "hard_break"]));
        assert!(matches(&m, &["hard_break", "hard_break", "hard_break"]));
        assert!(matches(&m, &["hard_break", "hard_break", "hard_break", "hard_break"]));
        assert!(!matches(&m, &["hard_break", "hard_break", "hard_break", "hard_break", "hard_break"]));
    }

    #[test]
    fn parse_open_range() {
        let m = parse_ok("hard_break{2,}");
        assert!(!matches(&m, &["hard_break"]));
        assert!(matches(&m, &["hard_break", "hard_break"]));
        assert!(matches(&m, &["hard_break", "hard_break", "hard_break", "hard_break"]));
    }

    #[test]
    fn parse_nested_plus() {
        let m = parse_ok("(heading paragraph+)+");
        assert!(matches(&m, &["heading", "paragraph"]));
        assert!(matches(&m, &["heading", "paragraph", "heading", "paragraph", "paragraph"]));
        assert!(!matches(&m, &["heading", "paragraph", "heading", "paragraph", "paragraph", "horizontal_rule"]));
    }

    #[test]
    fn parse_unknown_type_error() {
        let res = ContentMatch::parse(
            "unknown_type",
            &node_types_basic(),
            &groups_basic(),
            &required_basic(),
        );
        assert!(res.is_err());
    }

    #[test]
    fn inline_content_flag_true() {
        let m = parse_ok("image*");
        assert!(m.inline_content, "image is inline, so inline_content should be true");
    }

    #[test]
    fn inline_content_flag_false() {
        let m = parse_ok("paragraph*");
        assert!(!m.inline_content, "paragraph is block, so inline_content should be false");
    }

    #[test]
    fn inline_content_empty() {
        let m = parse_ok("");
        assert!(!m.inline_content);
    }

    #[test]
    fn default_type_returns_first_generatable() {
        let m = parse_ok("paragraph+");
        assert_eq!(m.default_type().as_deref(), Some("paragraph"));
    }

    #[test]
    fn default_type_empty_has_none() {
        assert_eq!(ContentMatch::empty().default_type(), None);
    }

    #[test]
    fn sequence_star_zero_and_many() {
        // "heading paragraph*" — heading required, paragraphs optional
        let m = parse_ok("heading paragraph*");
        assert!(matches(&m, &["heading"]));
        assert!(matches(&m, &["heading", "paragraph", "paragraph"]));
        assert!(!matches(&m, &[]));
        assert!(!matches(&m, &["paragraph"]));
    }

    #[test]
    fn sequence_plus_required() {
        // "heading paragraph+" — both required
        let m = parse_ok("heading paragraph+");
        assert!(!matches(&m, &["heading"]));
        assert!(matches(&m, &["heading", "paragraph"]));
        assert!(matches(&m, &["heading", "paragraph", "paragraph"]));
        assert!(!matches(&m, &["paragraph", "paragraph"]));
    }

    // ---- match_fragment 测试 ----

    #[test]
    fn match_fragment_empty_returns_self() {
        use crate::model::fragment::Fragment;
        use indexmap::indexmap;
        use crate::model::schema::{Schema, SchemaSpec, NodeSpec, MarkSpec};
        let s = Schema::new(SchemaSpec {
            nodes: indexmap! {
                "doc".to_string() => NodeSpec { content: Some("paragraph+".to_string()), ..NodeSpec::default() },
                "paragraph".to_string() => NodeSpec { content: Some("text*".to_string()), ..NodeSpec::default() },
                "text".to_string() => NodeSpec { inline: true, ..NodeSpec::default() },
            },
            marks: indexmap! {},
            top_node: None,
        }).unwrap();
        let cm = &s.nodes["doc"].content_match;
        // Empty fragment: returns Some(cm itself)
        let result = cm.match_fragment(&Fragment::empty(), 0, 0);
        assert!(result.is_some());
    }

    #[test]
    fn match_fragment_valid_content() {
        use crate::model::fragment::Fragment;
        use crate::model::node::Node;
        use indexmap::indexmap;
        use crate::model::schema::{Schema, SchemaSpec, NodeSpec, MarkSpec};
        use std::sync::Arc;
        let s = Schema::new(SchemaSpec {
            nodes: indexmap! {
                "doc".to_string() => NodeSpec { content: Some("paragraph+".to_string()), ..NodeSpec::default() },
                "paragraph".to_string() => NodeSpec { content: Some("text*".to_string()), ..NodeSpec::default() },
                "text".to_string() => NodeSpec { inline: true, ..NodeSpec::default() },
            },
            marks: indexmap! {},
            top_node: None,
        }).unwrap();
        let para = Node::new(s.nodes["paragraph"].clone(), Arc::new(Default::default()), Fragment::empty(), vec![]);
        let frag = Fragment::from_node(para);
        let cm = &s.nodes["doc"].content_match;
        // "paragraph+" can match one paragraph
        let result = cm.match_fragment(&frag, 0, frag.child_count());
        assert!(result.is_some());
        assert!(result.unwrap().valid_end);
    }

    #[test]
    fn match_fragment_invalid_content() {
        use crate::model::fragment::Fragment;
        use crate::model::node::Node;
        use indexmap::indexmap;
        use crate::model::schema::{Schema, SchemaSpec, NodeSpec, MarkSpec};
        use std::sync::Arc;
        let s = Schema::new(SchemaSpec {
            nodes: indexmap! {
                "doc".to_string() => NodeSpec { content: Some("paragraph+".to_string()), ..NodeSpec::default() },
                "paragraph".to_string() => NodeSpec { content: Some("text*".to_string()), ..NodeSpec::default() },
                "text".to_string() => NodeSpec { inline: true, ..NodeSpec::default() },
            },
            marks: indexmap! {},
            top_node: None,
        }).unwrap();
        // Create a "text" node (not a paragraph) — should not match "paragraph+"
        let text = s.text("hello".to_string(), vec![]);
        let frag = Fragment::from_node(text);
        let cm = &s.nodes["doc"].content_match;
        let result = cm.match_fragment(&frag, 0, frag.child_count());
        assert!(result.is_none());
    }

    #[test]
    fn valid_content_true_for_valid_fragment() {
        use crate::model::fragment::Fragment;
        use crate::model::node::Node;
        use indexmap::indexmap;
        use crate::model::schema::{Schema, SchemaSpec, NodeSpec, MarkSpec};
        use std::sync::Arc;
        let s = Schema::new(SchemaSpec {
            nodes: indexmap! {
                "doc".to_string() => NodeSpec { content: Some("paragraph+".to_string()), ..NodeSpec::default() },
                "paragraph".to_string() => NodeSpec { content: Some("text*".to_string()), ..NodeSpec::default() },
                "text".to_string() => NodeSpec { inline: true, ..NodeSpec::default() },
            },
            marks: indexmap! {},
            top_node: None,
        }).unwrap();
        let para = Node::new(s.nodes["paragraph"].clone(), Arc::new(Default::default()), Fragment::empty(), vec![]);
        let frag = Fragment::from_node(para);
        assert!(s.nodes["doc"].valid_content(&frag));
    }

    #[test]
    fn valid_content_false_for_empty_when_required() {
        use crate::model::fragment::Fragment;
        use indexmap::indexmap;
        use crate::model::schema::{Schema, SchemaSpec, NodeSpec, MarkSpec};
        let s = Schema::new(SchemaSpec {
            nodes: indexmap! {
                "doc".to_string() => NodeSpec { content: Some("paragraph+".to_string()), ..NodeSpec::default() },
                "paragraph".to_string() => NodeSpec { content: Some("text*".to_string()), ..NodeSpec::default() },
                "text".to_string() => NodeSpec { inline: true, ..NodeSpec::default() },
            },
            marks: indexmap! {},
            top_node: None,
        }).unwrap();
        // "paragraph+" requires at least one — empty fragment should fail valid_end
        assert!(!s.nodes["doc"].valid_content(&Fragment::empty()));
    }
}
