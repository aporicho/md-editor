# model Phase 2.1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 ContentMatch 的核心数据结构和 NFA→DFA 解析引擎，以及所有不依赖 Fragment 的方法。

**Architecture:** 单文件 `src/model/content.rs`。内部包含 TokenStream、Expr 枚举、NFA/DFA 构建函数（私有），对外暴露 ContentMatch struct 和 parse()。依赖 Fragment 的方法（match_fragment、fill_before、find_wrapping）留 stub，Phase 3 补全。

**Tech Stack:** Rust 2024 edition, `std::sync::Arc`, Phase 1 的 SyntaxError/RangeError

**TS 参考：** `reference/prosemirror-model/src/content.ts`

---

## 文件结构

```
src/model/content.rs    ← 新建：ContentMatch + 解析引擎
src/model/mod.rs        ← 修改：添加 pub mod content;
```

---

## Task 1: 声明模块 + 骨架

**Files:**
- Modify: `src/model/mod.rs`
- Create: `src/model/content.rs`（骨架，先让代码可编译）

- [ ] **Step 1: 在 mod.rs 加入模块声明**

```rust
// src/model/mod.rs
// prosemirror-model Rust 重设计
// 参见 docs/superpowers/specs/rust-prosemirror-porting-guide.md

pub mod errors;
pub mod compare_deep;
pub mod render_output;
pub mod schema;
pub mod content;
```

- [ ] **Step 2: 创建 content.rs 骨架（可编译但功能为空）**

```rust
// src/model/content.rs
//
// ContentMatch：节点内容表达式的 DFA 匹配状态机。
// 对应 TypeScript 的 content.ts。
//
// 注意：以下方法为占位实现，依赖 Fragment，将在 Phase 3 补全：
//   - match_fragment
//   - fill_before
//   - find_wrapping
//
use std::sync::Arc;

use crate::model::errors::{RangeError, SyntaxError};

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
    /// Computed at parse time from the first edge's type.
    pub inline_content: bool,
    /// Outgoing edges: (type_name, next_state).
    edges: Vec<(String, Arc<ContentMatch>)>,
    /// Cache for find_wrapping results: alternating (type_name, result) pairs.
    /// Each result is None (no wrapping) or Some(Vec<String>) (list of wrapper type names).
    wrap_cache: std::cell::RefCell<Vec<WrapCacheEntry>>,
}

enum WrapCacheEntry {
    Query(String),
    Result(Option<Vec<String>>),
}

impl ContentMatch {
    /// The empty content match (valid end, no edges). Used for leaf nodes.
    pub fn empty() -> Arc<ContentMatch> {
        Arc::new(ContentMatch {
            valid_end: true,
            inline_content: false,
            edges: vec![],
            wrap_cache: std::cell::RefCell::new(vec![]),
        })
    }
}
```

- [ ] **Step 3: 验证编译**

```bash
cargo check
```

Expected: 无报错

- [ ] **Step 4: Commit**

```bash
git add src/model/content.rs src/model/mod.rs
git commit -m "feat(model): add ContentMatch skeleton"
```

---

## Task 2: 写 match_type / edge_count / edge 的测试（TDD）

**Files:**
- Modify: `src/model/content.rs`（添加测试模块）

这些测试只需要手工构造 ContentMatch，不需要 Schema 或 Fragment。

- [ ] **Step 1: 在 content.rs 末尾添加测试模块**

```rust
// src/model/content.rs 末尾追加：

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state(valid_end: bool, edges: Vec<(&str, Arc<ContentMatch>)>) -> Arc<ContentMatch> {
        Arc::new(ContentMatch {
            valid_end,
            inline_content: false,
            edges: edges.into_iter().map(|(n, s)| (n.to_string(), s)).collect(),
            wrap_cache: std::cell::RefCell::new(vec![]),
        })
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
}
```

- [ ] **Step 2: 运行测试，确认失败（方法未定义）**

```bash
cargo test -p md-editor model::content::tests
```

Expected: FAIL（编译错误：match_type、edge_count、edge、compatible 未定义）

---

## Task 3: 实现 match_type / edge_count / edge / compatible

**Files:**
- Modify: `src/model/content.rs`

- [ ] **Step 1: 在 ContentMatch impl 块中添加方法**

```rust
impl ContentMatch {
    // ... empty() 已有 ...

    /// Match a node type by name, returning the next match state if successful.
    ///
    /// Match a node type, returning a match after that node if successful.
    pub fn match_type(&self, type_name: &str) -> Option<&Arc<ContentMatch>> {
        self.edges
            .iter()
            .find(|(name, _)| name == type_name)
            .map(|(_, next)| next)
    }

    /// The number of outgoing edges this node has in the finite automaton
    /// that describes the content expression.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Get the nth outgoing edge from this node in the finite automaton
    /// that describes the content expression.
    ///
    /// Returns `(type_name, next_state)`, or `Err(RangeError)` if n is out of range.
    pub fn edge(&self, n: usize) -> Result<(&str, &Arc<ContentMatch>), RangeError> {
        self.edges
            .get(n)
            .map(|(name, next)| (name.as_str(), next))
            .ok_or_else(|| RangeError(format!("There's no {}th edge in this content match", n)))
    }

    /// Returns true if this match and `other` share at least one outgoing type name.
    ///
    /// Indicates whether this node allows some of the same content as the given match.
    pub fn compatible(&self, other: &ContentMatch) -> bool {
        for (name, _) in &self.edges {
            for (other_name, _) in &other.edges {
                if name == other_name {
                    return true;
                }
            }
        }
        false
    }
}
```

- [ ] **Step 2: 运行测试，确认通过**

```bash
cargo test -p md-editor model::content::tests
```

Expected:
```
running 7 tests
test model::content::tests::match_type_hit ... ok
test model::content::tests::match_type_miss ... ok
test model::content::tests::empty_valid_end ... ok
test model::content::tests::edge_count_and_edge ... ok
test model::content::tests::edge_out_of_range ... ok
test model::content::tests::compatible_same_type ... ok
test model::content::tests::compatible_different_types ... ok
test result: ok. 7 passed; 0 failed
```

- [ ] **Step 3: Commit**

```bash
git add src/model/content.rs
git commit -m "feat(model): add ContentMatch match_type, edge_count, edge, compatible"
```

---

## Task 4: Expr 枚举 + TokenStream（NFA→DFA 预备）

**Files:**
- Modify: `src/model/content.rs`

TS 原文：`reference/prosemirror-model/src/content.ts` 中 `TokenStream` 类和 `Expr` 类型

- [ ] **Step 1: 添加 Expr 枚举和 TokenStream（私有，放在 mod 内 impl 外）**

```rust
// src/model/content.rs — 在 ContentMatch struct 定义之前插入：

/// An expression node produced by the content expression parser.
#[derive(Debug, Clone)]
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

/// Tokenizer for content expressions.
///
/// Splits expressions like `"paragraph horizontal_rule paragraph"` or
/// `"(heading | paragraph)+"` into tokens.
struct TokenStream<'a> {
    string: &'a str,
    node_types: &'a indexmap::IndexMap<String, bool>, // name → is_inline
    tokens: Vec<&'a str>,
    pos: usize,
}

impl<'a> TokenStream<'a> {
    fn new(
        string: &'a str,
        node_types: &'a indexmap::IndexMap<String, bool>,
    ) -> Result<Self, SyntaxError> {
        let tokens: Vec<&'a str> = {
            let mut out = vec![];
            let mut start = 0;
            let bytes = string.as_bytes();
            let mut i = 0;
            while i <= bytes.len() {
                // boundary: whitespace, word boundary, non-word char, or end
                let at_end = i == bytes.len();
                let is_sep = at_end || {
                    let b = bytes[i] as char;
                    b.is_whitespace() || (!b.is_alphanumeric() && b != '_')
                };
                if is_sep && start < i {
                    out.push(&string[start..i]);
                    start = i;
                }
                if !at_end {
                    let b = bytes[i] as char;
                    if b.is_whitespace() {
                        start = i + 1;
                    } else if !b.is_alphanumeric() && b != '_' {
                        if start < i { /* already pushed */ }
                        out.push(&string[i..i + 1]);
                        start = i + 1;
                    }
                }
                i += 1;
            }
            out.into_iter().filter(|s| !s.trim().is_empty()).collect()
        };
        Ok(TokenStream { string, node_types, tokens, pos: 0 })
    }

    fn next(&self) -> Option<&str> {
        self.tokens.get(self.pos).copied()
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
```

- [ ] **Step 2: 验证编译（暂不测试，后续 parse() 测试会覆盖）**

```bash
cargo check
```

Expected: 无报错（`indexmap` 需要在 content.rs 顶部加 use）

添加 import：在文件顶部已有的 `use` 块中加入：
```rust
use indexmap::IndexMap;
```

- [ ] **Step 3: Commit**

```bash
git add src/model/content.rs
git commit -m "feat(model): add Expr enum and TokenStream for content expression parsing"
```

---

## Task 5: 表达式解析器（parse_expr 系列函数）

**Files:**
- Modify: `src/model/content.rs`

TS 原文：`parseExpr`, `parseExprSeq`, `parseExprSubscript`, `parseExprAtom`, `parseNum`, `parseExprRange`, `resolveName`

- [ ] **Step 1: 添加解析函数（私有，放在 impl 外）**

```rust
// src/model/content.rs — 添加到 TokenStream impl 之后，nfa() 之前：

fn parse_expr(stream: &mut TokenStream) -> Result<Expr, SyntaxError> {
    let mut exprs = vec![parse_expr_seq(stream)?];
    while stream.eat("|") {
        exprs.push(parse_expr_seq(stream)?);
    }
    Ok(if exprs.len() == 1 { exprs.remove(0) } else { Expr::Choice(exprs) })
}

fn parse_expr_seq(stream: &mut TokenStream) -> Result<Expr, SyntaxError> {
    let mut exprs = vec![parse_expr_subscript(stream)?];
    while stream.next().map_or(false, |t| t != ")" && t != "|") {
        exprs.push(parse_expr_subscript(stream)?);
    }
    Ok(if exprs.len() == 1 { exprs.remove(0) } else { Expr::Seq(exprs) })
}

fn parse_expr_subscript(stream: &mut TokenStream) -> Result<Expr, SyntaxError> {
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

fn parse_num(stream: &mut TokenStream) -> Result<usize, SyntaxError> {
    let tok = stream
        .next()
        .ok_or_else(|| stream.err("Expected number, got end of expression"))?;
    tok.parse::<usize>()
        .map_err(|_| stream.err(&format!("Expected number, got '{}'", tok)))
        .map(|n| { stream.pos += 1; n })
}

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

/// Resolves a name token: returns a list of (type_name, is_inline) pairs.
/// The name may be a direct type name or a group name.
fn resolve_name<'a>(
    stream: &TokenStream<'a>,
    name: &str,
    groups: &indexmap::IndexMap<String, Vec<String>>, // group → [type_names]
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

fn parse_expr_atom(
    stream: &mut TokenStream,
    groups: &indexmap::IndexMap<String, Vec<String>>,
) -> Result<Expr, SyntaxError> {
    if stream.eat("(") {
        let expr = parse_expr(stream)?;
        if !stream.eat(")") {
            return Err(stream.err("Missing closing paren"));
        }
        return Ok(expr);
    }
    let tok = stream
        .next()
        .ok_or_else(|| stream.err("Unexpected end of expression"))?;
    // word token: must be alphanumeric or _
    if tok.chars().all(|c| c.is_alphanumeric() || c == '_') {
        let resolved = resolve_name(stream, tok, groups)?;
        stream.pos += 1;
        // Check mixing inline/block (TS does this, we mirror it)
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
        Ok(if exprs.len() == 1 { exprs.into_iter().next().unwrap() } else { Expr::Choice(exprs) })
    } else {
        Err(stream.err(&format!("Unexpected token '{}'", tok)))
    }
}
```

**注意：** `parse_expr_seq` 和 `parse_expr_subscript` 调用的 atom 函数需要 `groups` 参数，需要传递。重构上面的 `parse_expr` / `parse_expr_seq` / `parse_expr_subscript` 也带上 groups 参数：

```rust
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

fn parse_expr_seq(
    stream: &mut TokenStream,
    groups: &IndexMap<String, Vec<String>>,
) -> Result<Expr, SyntaxError> {
    let mut exprs = vec![parse_expr_subscript(stream, groups)?];
    while stream.next().map_or(false, |t| t != ")" && t != "|") {
        exprs.push(parse_expr_subscript(stream, groups)?);
    }
    Ok(if exprs.len() == 1 { exprs.remove(0) } else { Expr::Seq(exprs) })
}

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
```

- [ ] **Step 2: 验证编译**

```bash
cargo check
```

Expected: 无报错（如有未使用变量 warning，可先忽略，步骤 3 的 NFA 会用到所有函数）

- [ ] **Step 3: Commit**

```bash
git add src/model/content.rs
git commit -m "feat(model): add content expression parser (parse_expr family)"
```

---

## Task 6: NFA 构建

**Files:**
- Modify: `src/model/content.rs`

TS 原文：`nfa()`, `nullFrom()` 函数

NFA 表示：`Vec<Vec<NfaEdge>>`，第 0 个 state 是入口，最后一个是成功 state。

```
NfaEdge: { term: Option<String>, to: usize }
```

- [ ] **Step 1: 添加 NFA 数据结构和 nfa() 函数**

```rust
// src/model/content.rs — 添加在解析函数之后：

#[derive(Debug, Clone)]
struct NfaEdge {
    /// None = ε-edge (null transition)
    term: Option<String>,
    to: usize,
}

/// Build an NFA from a parsed Expr.
///
/// The NFA is represented as a vector of states (each a vector of edges).
/// State 0 is the entry; the last state is the success/accept state.
fn build_nfa(expr: &Expr) -> Vec<Vec<NfaEdge>> {
    let mut nfa: Vec<Vec<NfaEdge>> = vec![vec![]]; // state 0

    fn new_state(nfa: &mut Vec<Vec<NfaEdge>>) -> usize {
        nfa.push(vec![]);
        nfa.len() - 1
    }

    fn add_edge(nfa: &mut Vec<Vec<NfaEdge>>, from: usize, to: usize, term: Option<String>) {
        nfa[from].push(NfaEdge { term, to });
    }

    fn connect(nfa: &mut Vec<Vec<NfaEdge>>, edges: &[(usize, usize)], to: usize) {
        // edges: (from_state, edge_index) pairs whose `to` field is unset (points to usize::MAX)
        for &(state, idx) in edges {
            nfa[state][idx].to = to;
        }
    }

    /// Compile expr from `from` state.
    /// Returns list of (state, edge_index) for "dangling" edges (their `to` is usize::MAX).
    fn compile(
        expr: &Expr,
        from: usize,
        nfa: &mut Vec<Vec<NfaEdge>>,
    ) -> Vec<(usize, usize)> {
        match expr {
            Expr::Choice(exprs) => {
                exprs
                    .iter()
                    .flat_map(|e| compile(e, from, nfa))
                    .collect()
            }
            Expr::Seq(exprs) => {
                let mut cur = from;
                let last = exprs.len() - 1;
                for (i, e) in exprs.iter().enumerate() {
                    let dangling = compile(e, cur, nfa);
                    if i == last {
                        return dangling;
                    }
                    let next = new_state(nfa);
                    connect(nfa, &dangling, next);
                    cur = next;
                }
                unreachable!()
            }
            Expr::Star(inner) => {
                let loop_state = new_state(nfa);
                // ε from `from` to loop_state
                let idx = nfa[from].len();
                nfa[from].push(NfaEdge { term: None, to: loop_state });
                let dangling_inner = compile(inner, loop_state, nfa);
                connect(nfa, &dangling_inner, loop_state);
                // One dangling ε-edge from loop_state (the exit)
                let exit_idx = nfa[loop_state].len();
                nfa[loop_state].push(NfaEdge { term: None, to: usize::MAX });
                // The ε edge from `from` to loop_state is NOT dangling
                let _ = idx;
                vec![(loop_state, exit_idx)]
            }
            Expr::Plus(inner) => {
                let loop_state = new_state(nfa);
                let first = compile(inner, from, nfa);
                connect(nfa, &first, loop_state);
                let second = compile(inner, loop_state, nfa);
                connect(nfa, &second, loop_state);
                let exit_idx = nfa[loop_state].len();
                nfa[loop_state].push(NfaEdge { term: None, to: usize::MAX });
                vec![(loop_state, exit_idx)]
            }
            Expr::Opt(inner) => {
                // ε-edge from `from` (the skip path) + the compiled inner
                let skip_idx = nfa[from].len();
                nfa[from].push(NfaEdge { term: None, to: usize::MAX });
                let mut dangling = vec![(from, skip_idx)];
                dangling.extend(compile(inner, from, nfa));
                dangling
            }
            Expr::Range { min, max, expr: inner } => {
                let mut cur = from;
                // Mandatory min repetitions
                for _ in 0..*min {
                    let next = new_state(nfa);
                    let d = compile(inner, cur, nfa);
                    connect(nfa, &d, next);
                    cur = next;
                }
                match max {
                    None => {
                        // Open range {n,}: after min, loop back
                        let d = compile(inner, cur, nfa);
                        connect(nfa, &d, cur);
                        let exit_idx = nfa[cur].len();
                        nfa[cur].push(NfaEdge { term: None, to: usize::MAX });
                        vec![(cur, exit_idx)]
                    }
                    Some(m) => {
                        let mut dangling: Vec<(usize, usize)> = vec![];
                        // Optional (max - min) repetitions
                        for _ in *min..*m {
                            let next = new_state(nfa);
                            let skip_idx = nfa[cur].len();
                            nfa[cur].push(NfaEdge { term: None, to: usize::MAX });
                            dangling.push((cur, skip_idx));
                            let d = compile(inner, cur, nfa);
                            connect(nfa, &d, next);
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

    let success = new_state(&mut nfa);
    let dangling = compile(expr, 0, &mut nfa);
    connect(&mut nfa, &dangling, success);
    nfa
}

/// Returns the set of NFA states reachable from `state` via ε-edges (sorted descending).
///
/// Omits states that have only a single null-out-edge (they lead to needless duplicates).
fn null_from(nfa: &[Vec<NfaEdge>], state: usize) -> Vec<usize> {
    let mut result: Vec<usize> = vec![];

    fn scan(nfa: &[Vec<NfaEdge>], node: usize, result: &mut Vec<usize>) {
        let edges = &nfa[node];
        // Skip through single-ε chains
        if edges.len() == 1 && edges[0].term.is_none() {
            scan(nfa, edges[0].to, result);
            return;
        }
        if result.contains(&node) {
            return;
        }
        result.push(node);
        for e in edges {
            if e.term.is_none() && !result.contains(&e.to) {
                scan(nfa, e.to, result);
            }
        }
    }

    scan(nfa, state, &mut result);
    result.sort_by(|a, b| b.cmp(a)); // descending
    result
}
```

- [ ] **Step 2: 验证编译**

```bash
cargo check
```

Expected: 无报错

- [ ] **Step 3: Commit**

```bash
git add src/model/content.rs
git commit -m "feat(model): add NFA builder for content expressions"
```

---

## Task 7: DFA 构建 + ContentMatch::parse()

**Files:**
- Modify: `src/model/content.rs`

TS 原文：`dfa()`, `checkForDeadEnds()`, `ContentMatch.parse()`

- [ ] **Step 1: 添加 DFA 构建和 parse()**

```rust
// src/model/content.rs — 添加在 null_from 之后：

/// Compiles an NFA into a DFA represented as ContentMatch states.
fn build_dfa(
    nfa: &[Vec<NfaEdge>],
    node_types: &IndexMap<String, bool>, // name → is_inline
) -> Arc<ContentMatch> {
    use std::collections::HashMap;
    let accept_state = nfa.len() - 1;

    // labeled: state-set key → Arc<ContentMatch>
    let mut labeled: HashMap<String, Arc<ContentMatch>> = HashMap::new();

    fn key(states: &[usize]) -> String {
        states
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(",")
    }

    fn explore(
        states: &[usize],
        nfa: &[Vec<NfaEdge>],
        accept_state: usize,
        node_types: &IndexMap<String, bool>,
        labeled: &mut HashMap<String, Arc<ContentMatch>>,
    ) -> Arc<ContentMatch> {
        // Collect outgoing edges grouped by term
        let mut out: Vec<(String, Vec<usize>)> = vec![];
        for &node in states {
            for edge in &nfa[node] {
                if let Some(term) = &edge.term {
                    // Find or create the target set entry
                    let set = if let Some(pos) = out.iter().position(|(t, _)| t == term) {
                        &mut out[pos].1
                    } else {
                        out.push((term.clone(), vec![]));
                        let last = out.len() - 1;
                        &mut out[last].1
                    };
                    // Add null-reachable states from edge.to
                    for ns in null_from(nfa, edge.to) {
                        if !set.contains(&ns) {
                            set.push(ns);
                        }
                    }
                }
            }
        }

        let valid_end = states.contains(&accept_state);
        let inline_content = out
            .first()
            .and_then(|(t, _)| node_types.get(t).copied())
            .unwrap_or(false);

        // Register this state before recursing (prevent infinite loops)
        let k = key(states);
        let cm = Arc::new(ContentMatch {
            valid_end,
            inline_content,
            edges: vec![],
            wrap_cache: std::cell::RefCell::new(vec![]),
        });
        labeled.insert(k, cm.clone());

        // Build edges (sort each target set descending for determinism)
        let mut edges: Vec<(String, Arc<ContentMatch>)> = vec![];
        for (term, mut target_states) in out {
            target_states.sort_by(|a, b| b.cmp(a));
            let child_key = key(&target_states);
            let next = if let Some(existing) = labeled.get(&child_key) {
                existing.clone()
            } else {
                explore(&target_states, nfa, accept_state, node_types, labeled)
            };
            edges.push((term, next));
        }

        // Patch edges into the Arc we already inserted
        // Safety: refcount is 1 here (only `cm` and `labeled` hold it, and we just inserted)
        if let Some(m) = Arc::get_mut(&mut labeled.get_mut(&key(states)).unwrap()) {
            m.edges = edges;
        }

        labeled[&key(states)].clone()
    }

    let start = null_from(nfa, 0);
    explore(&start, nfa, accept_state, node_types, &mut labeled)
}

/// Check that every non-valid-end state has at least one generatable outgoing type.
///
/// A "generatable" type is one that is not text and has no required attrs.
fn check_for_dead_ends(
    match_state: &Arc<ContentMatch>,
    expr_str: &str,
    // name → has_required_attrs
    has_required: &IndexMap<String, bool>,
) -> Result<(), SyntaxError> {
    let mut work: Vec<Arc<ContentMatch>> = vec![match_state.clone()];
    let mut i = 0;
    while i < work.len() {
        let state = work[i].clone();
        i += 1;
        let dead = !state.valid_end;
        let mut node_names: Vec<String> = vec![];
        let mut all_non_generatable = dead;
        for (name, next) in &state.edges {
            node_names.push(name.clone());
            let is_text = name == "text";
            let required = has_required.get(name).copied().unwrap_or(false);
            if !(is_text || required) {
                all_non_generatable = false;
            }
            if !work.iter().any(|w| Arc::ptr_eq(w, next)) {
                work.push(next.clone());
            }
        }
        if dead && all_non_generatable && !node_names.is_empty() {
            return Err(SyntaxError(format!(
                "Only non-generatable nodes ({}) in a required position (in content expression '{}')",
                node_names.join(", "),
                expr_str
            )));
        }
    }
    Ok(())
}
```

Now add `ContentMatch::parse()` to the impl block:

```rust
impl ContentMatch {
    // ... existing methods ...

    /// Parse a content expression string into a DFA.
    ///
    /// `node_types`: map from type name to `is_inline` flag.
    /// `groups`: map from group name to list of type names in that group.
    /// `has_required_attrs`: map from type name to whether it has required attrs.
    pub fn parse(
        expr: &str,
        node_types: &IndexMap<String, bool>,
        groups: &IndexMap<String, Vec<String>>,
        has_required_attrs: &IndexMap<String, bool>,
    ) -> Result<Arc<ContentMatch>, SyntaxError> {
        if expr.trim().is_empty() {
            return Ok(ContentMatch::empty());
        }
        let mut stream = TokenStream::new(expr, node_types)?;
        let ast = parse_expr(&mut stream, groups)?;
        if stream.next().is_some() {
            return Err(stream.err("Unexpected trailing text"));
        }
        let nfa = build_nfa(&ast);
        let dfa = build_dfa(&nfa, node_types);
        check_for_dead_ends(&dfa, expr, has_required_attrs)?;
        Ok(dfa)
    }
}
```

- [ ] **Step 2: 验证编译**

```bash
cargo check
```

Expected: 无报错

- [ ] **Step 3: Commit**

```bash
git add src/model/content.rs
git commit -m "feat(model): add DFA builder and ContentMatch::parse"
```

---

## Task 8: parse() 测试

**Files:**
- Modify: `src/model/content.rs`（在测试模块中新增测试）

这些测试不需要 Schema，直接用 `node_types` map 构造。

- [ ] **Step 1: 在 tests 模块中添加 parse 测试**

```rust
// src/model/content.rs tests 模块中追加：

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
        // All false (no required attrs) for simplicity in tests
        names.iter().map(|n| (n.to_string(), false)).collect()
    }

    fn node_types_basic() -> IndexMap<String, bool> {
        // is_inline: paragraph=false, heading=false, image=true, text=true
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
        let mut cur: Option<&Arc<ContentMatch>> = Some(m);
        for &t in types {
            if let Some(state) = cur {
                cur = state.match_type(t);
            } else {
                return false;
            }
        }
        cur.map_or(false, |s| s.valid_end)
    }

    #[test]
    fn parse_empty_expr() {
        let m = parse_ok("");
        assert!(m.valid_end);
        assert_eq!(m.edge_count(), 0);
        // Can't match any type
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
        // (heading paragraph+)+
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
    fn inline_content_flag() {
        let m = parse_ok("image*");
        assert!(m.inline_content, "image is inline, so inline_content should be true");
        let m2 = parse_ok("paragraph*");
        assert!(!m2.inline_content, "paragraph is block, inline_content should be false");
    }

    #[test]
    fn default_type_returns_first_generatable() {
        let m = parse_ok("paragraph+");
        assert_eq!(m.default_type(), Some("paragraph"));
        // Empty has no default
        assert_eq!(ContentMatch::empty().default_type(), None);
    }
```

- [ ] **Step 2: 添加 `default_type()` 方法（返回 Option<&str>）**

```rust
impl ContentMatch {
    // ... 已有方法 ...

    /// Get the name of the first matching node type at this match position that can be generated.
    ///
    /// A type can be generated if it is not "text" and has no required attributes
    /// (those are tracked externally; here we use the has-required flag baked in during parse).
    /// Returns the type name, or None if no generatable type exists.
    pub fn default_type(&self) -> Option<&str> {
        self.edges
            .iter()
            .find(|(name, _)| name != "text")
            .map(|(name, _)| name.as_str())
    }
}
```

**注意：** default_type 的 "无 required attrs" 检查在 TS 里通过 `type.hasRequiredAttrs()` 完成。因为 Phase 2.1 没有 NodeType，此处简化为"不是 text 就返回"。Phase 2.2/2.3 中 Schema 构建后，如果需要精确行为，可通过集成测试覆盖。

- [ ] **Step 3: 运行所有测试**

```bash
cargo test -p md-editor model::content::tests
```

Expected:
```
running 21 tests
test model::content::tests::match_type_hit ... ok
test model::content::tests::match_type_miss ... ok
test model::content::tests::empty_valid_end ... ok
test model::content::tests::edge_count_and_edge ... ok
test model::content::tests::edge_out_of_range ... ok
test model::content::tests::compatible_same_type ... ok
test model::content::tests::compatible_different_types ... ok
test model::content::tests::parse_empty_expr ... ok
test model::content::tests::parse_star ... ok
test model::content::tests::parse_plus ... ok
test model::content::tests::parse_opt ... ok
test model::content::tests::parse_seq ... ok
test model::content::tests::parse_choice ... ok
test model::content::tests::parse_group ... ok
test model::content::tests::parse_count_exact ... ok
test model::content::tests::parse_count_range ... ok
test model::content::tests::parse_open_range ... ok
test model::content::tests::parse_nested_plus ... ok
test model::content::tests::parse_unknown_type_error ... ok
test model::content::tests::inline_content_flag ... ok
test model::content::tests::default_type_returns_first_generatable ... ok
test result: ok. 21 passed; 0 failed
```

- [ ] **Step 4: Commit**

```bash
git add src/model/content.rs
git commit -m "feat(model): add ContentMatch::parse tests (NFA→DFA, match_type, inline_content)"
```

---

## Task 9: match_fragment / fill_before / find_wrapping stubs

**Files:**
- Modify: `src/model/content.rs`

这三个方法依赖 Fragment（Phase 3），此处只加签名和 todo!()，保证 API 完整。

- [ ] **Step 1: 在文件顶部添加注释说明 stubs**

在 `content.rs` 的文件顶部注释中加入（已有的注释块，追加说明）：

```rust
// src/model/content.rs
//
// ContentMatch：节点内容表达式的 DFA 匹配状态机。
// 对应 TypeScript 的 content.ts。
//
// 注意：以下方法为占位实现，依赖 Fragment，将在 Phase 3 补全：
//   - match_fragment(&self, frag: &Fragment, start: usize, end: usize) -> Option<Arc<ContentMatch>>
//   - fill_before(&self, after: &Fragment, to_end: bool, start_index: usize) -> Option<Fragment>
//   - find_wrapping(&self, target_type_name: &str) -> Option<Vec<String>>
```

- [ ] **Step 2: 在 impl ContentMatch 中添加 stub 方法**

```rust
impl ContentMatch {
    // ... 已有方法 ...

    // ---- Phase 3 stubs (依赖 Fragment，届时补全) ----

    /// Try to match a fragment. Returns the resulting match state when successful.
    ///
    /// NOTE: stub — requires Fragment (Phase 3). Will be implemented in Phase 3.
    #[allow(unused_variables)]
    pub fn match_fragment_stub(&self) -> ! {
        todo!("match_fragment requires Fragment — implement in Phase 3")
    }

    /// Fill in missing content to make `after` fit after the current match state.
    ///
    /// NOTE: stub — requires Fragment (Phase 3). Will be implemented in Phase 3.
    #[allow(unused_variables)]
    pub fn fill_before_stub(&self) -> ! {
        todo!("fill_before requires Fragment — implement in Phase 3")
    }

    /// Find a set of wrapping node type names that would allow a node of the given type
    /// to appear at this position.
    ///
    /// NOTE: stub — requires NodeType.create_and_fill (Phase 3). Will be implemented in Phase 3.
    #[allow(unused_variables)]
    pub fn find_wrapping_stub(&self) -> ! {
        todo!("find_wrapping requires NodeType.create_and_fill — implement in Phase 3")
    }
}
```

**注意：** 方法名加 `_stub` 后缀，Phase 3 中实现时替换为正式方法名。这样可以避免 Phase 3 前被调用。

- [ ] **Step 3: 运行全量测试**

```bash
cargo test -p md-editor
```

Expected: 全部通过（包括 Phase 1 的 24 个测试和 Phase 2.1 的 21 个测试，共 45 个）

- [ ] **Step 4: Clippy 检查**

```bash
cargo clippy -p md-editor -- -D warnings
```

Expected: 无 errors，允许 `dead_code` 类 warning（stub 方法未被调用）

- [ ] **Step 5: 更新阶段计划状态**

将 `docs/superpowers/plans/model-phases.md` 中 Phase 2.1 状态从 `⬜ 待开始` 改为 `✅ 完成`。

- [ ] **Step 6: Commit**

```bash
git add src/model/content.rs docs/superpowers/plans/model-phases.md
git commit -m "feat(model): complete Phase 2.1 - ContentMatch DFA core with stubs for Fragment-dependent methods"
```
