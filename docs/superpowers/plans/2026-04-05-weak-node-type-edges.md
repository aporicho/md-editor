# ContentMatch Edges: String → Weak<NodeType> Refactor Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 `ContentMatch::edges` 中的类型名字符串替换为 `Weak<NodeType>` 弱引用，消除 `fill_before`/`find_wrapping`/`create_and_fill` 的 `schema` 参数，使 API 与 TypeScript 原版对称。

**Architecture:** `NodeType → Arc<ContentMatch>` 存在强引用；`ContentMatch::edges` 改用 `Weak<NodeType>` 持有反向引用，打破强引用循环而不会内存泄漏。所有需要 NodeType 属性的方法（`is_leaf`、`has_required_attrs`、`create_and_fill`）直接通过 `upgrade()` 获取，无需外部传入 schema。

**Tech Stack:** Rust `std::sync::{Arc, Weak}`, `indexmap::IndexMap`

---

## 文件列表

| 操作 | 文件 | 说明 |
|------|------|------|
| Modify | `src/model/content.rs` | 核心：修改全部 DFA 数据结构和相关方法 |
| Modify | `src/model/schema.rs` | 更新 `Schema::new` 调用侧 + `create_and_fill` 去掉 schema 参数 |
| Modify | `src/model/tests_content.rs` | fill/fill3 辅助函数去掉 schema |
| Modify | `docs/superpowers/plans/2026-04-05-phase-3-4c-find-wrapping.md` | 更新 find_wrapping 不再需要 schema |

---

## 关键设计

**修改前（字符串，需要外部 schema）：**
```rust
edges: OnceLock<Vec<(String, Arc<ContentMatch>)>>
// 调用方必须传 schema 才能查 NodeType
pub fn fill_before(self: &Arc<Self>, schema: &Arc<Schema>, ...) -> Option<Fragment>
```

**修改后（弱引用，直接访问 NodeType）：**
```rust
edges: OnceLock<Vec<(Weak<NodeType>, Arc<ContentMatch>)>>
// 直接 upgrade() 获取 NodeType，无需 schema
pub fn fill_before(self: &Arc<Self>, ...) -> Option<Fragment>
```

**`Weak::upgrade()` 返回 None 的时机：** 仅当 Schema 已 drop 后。使用 ContentMatch 时 Schema 必然存活，等价于当前字符串查找时 Schema 不存在的行为。

---

## Task 1: content.rs — 数据结构 + DFA builder

**Files:** `src/model/content.rs`

本 Task 修改 content.rs 中的数据结构定义和 DFA 构建逻辑。不改变方法签名（下一步做）。

- [ ] **Step 1: 修改 `Expr::Name` 变体（约第 23-32 行）**

将：
```rust
/// A resolved node type name (plus inline flag for the DFA builder).
Name { type_name: String, is_inline: bool },
```
替换为：
```rust
/// A resolved node type weak reference (for the DFA builder).
Name { node_type: std::sync::Weak<crate::model::schema::NodeType> },
```

- [ ] **Step 2: 修改 `TokenStream` 字段类型（约第 37-42 行）**

将：
```rust
struct TokenStream<'a> {
    string: &'a str,
    /// Map from node type name to is_inline flag.
    node_types: &'a IndexMap<String, bool>,
    tokens: Vec<String>,
    pos: usize,
}
```
替换为：
```rust
struct TokenStream<'a> {
    string: &'a str,
    node_types: &'a IndexMap<String, Arc<crate::model::schema::NodeType>>,
    tokens: Vec<String>,
    pos: usize,
}
```

- [ ] **Step 3: 修改 `NfaEdge.term` 字段类型（约第 234-240 行）**

将：
```rust
struct NfaEdge {
    /// None = ε-edge (epsilon/null transition)
    term: Option<String>,
    /// Target state index. usize::MAX means "dangling" (not yet connected).
    to: usize,
}
```
替换为：
```rust
struct NfaEdge {
    /// None = ε-edge (epsilon/null transition)
    term: Option<std::sync::Weak<crate::model::schema::NodeType>>,
    /// Target state index. usize::MAX means "dangling" (not yet connected).
    to: usize,
}
```

- [ ] **Step 4: 修改 `DfaState.edges` 字段类型（约第 408-414 行）**

将：
```rust
struct DfaState {
    valid_end: bool,
    inline_content: bool,
    /// Edges: (type_name, target_state_index).
    edges: Vec<(String, usize)>,
}
```
替换为：
```rust
struct DfaState {
    valid_end: bool,
    inline_content: bool,
    /// Edges: (weak node type ref, target_state_index).
    edges: Vec<(std::sync::Weak<crate::model::schema::NodeType>, usize)>,
}
```

- [ ] **Step 5: 修改 `resolve_name` 函数（约第 168-191 行）**

将整个 `resolve_name` 函数替换为：

```rust
/// Resolves a name token to a list of Weak<NodeType>.
/// The name may be a direct type name or a group name.
#[allow(dead_code)]
fn resolve_name(
    stream: &TokenStream,
    name: &str,
    groups: &IndexMap<String, Vec<String>>,
) -> Result<Vec<std::sync::Weak<crate::model::schema::NodeType>>, SyntaxError> {
    // Direct type name
    if let Some(nt) = stream.node_types.get(name) {
        return Ok(vec![Arc::downgrade(nt)]);
    }
    // Group name
    if let Some(members) = groups.get(name) {
        if members.is_empty() {
            return Err(stream.err(&format!("No node type or group '{}' found", name)));
        }
        let result: Result<Vec<_>, _> = members
            .iter()
            .map(|n| {
                stream.node_types
                    .get(n)
                    .map(|nt| Arc::downgrade(nt))
                    .ok_or_else(|| stream.err(&format!("Unknown node type '{}' in group '{}'", n, name)))
            })
            .collect();
        return result;
    }
    Err(stream.err(&format!("No node type or group '{}' found", name)))
}
```

- [ ] **Step 6: 修改 `parse_expr_atom` 中的 inline 检查和 Expr::Name 构建（约第 193-231 行）**

找到 `parse_expr_atom` 中 `let resolved = resolve_name(...)` 之后的部分，将：

```rust
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
```

替换为：

```rust
stream.pos += 1;
// Check mixing inline/block
let first_inline = resolved[0].upgrade().map_or(false, |nt| nt.is_inline());
for weak in &resolved {
    let is_inline = weak.upgrade().map_or(false, |nt| nt.is_inline());
    if is_inline != first_inline {
        return Err(stream.err("Mixing inline and block content"));
    }
}
let exprs: Vec<Expr> = resolved
    .into_iter()
    .map(|weak_nt| Expr::Name { node_type: weak_nt })
    .collect();
```

- [ ] **Step 7: 修改 `nfa_compile` 中 `Expr::Name` 分支（约第 347-354 行）**

将：
```rust
Expr::Name { type_name, .. } => {
    let edge_idx = nfa[from].len();
    nfa[from].push(NfaEdge {
        term: Some(type_name.clone()),
        to: usize::MAX,
    });
    vec![(from, edge_idx)]
}
```
替换为：
```rust
Expr::Name { node_type } => {
    let edge_idx = nfa[from].len();
    nfa[from].push(NfaEdge {
        term: Some(node_type.clone()),
        to: usize::MAX,
    });
    vec![(from, edge_idx)]
}
```

- [ ] **Step 8: 重写 `build_dfa_indexed` 函数（约第 417-495 行）**

将整个 `build_dfa_indexed` 函数替换为（移除 `node_types` 参数，按 Weak 指针而非字符串分组）：

```rust
/// Build a DFA from an NFA using subset construction, producing an indexed state table.
///
/// Returns (states, start_index) where `states[start_index]` is the initial state.
fn build_dfa_indexed(
    nfa: &[Vec<NfaEdge>],
) -> Vec<DfaState> {
    use std::collections::HashMap;
    let accept_state = nfa.len() - 1;

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
        labeled: &mut HashMap<String, usize>,
        states: &mut Vec<DfaState>,
    ) -> usize {
        let k = key(&state_set);
        if let Some(&idx) = labeled.get(&k) {
            return idx;
        }

        // Collect outgoing edges grouped by Weak<NodeType> pointer (ptr_eq)
        let mut out: Vec<(std::sync::Weak<crate::model::schema::NodeType>, Vec<usize>)> = vec![];
        for &node in &state_set {
            for edge in &nfa[node] {
                if let Some(weak_nt) = &edge.term {
                    let set_idx = if let Some(pos) = out.iter().position(|(w, _)| {
                        std::sync::Weak::ptr_eq(w, weak_nt)
                    }) {
                        pos
                    } else {
                        out.push((weak_nt.clone(), vec![]));
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
        // inline_content: upgrade first edge's NodeType
        let inline_content = out
            .first()
            .and_then(|(w, _)| w.upgrade())
            .map_or(false, |nt| nt.is_inline());

        let idx = states.len();
        labeled.insert(k, idx);
        states.push(DfaState { valid_end, inline_content, edges: vec![] });

        let mut edges: Vec<(std::sync::Weak<crate::model::schema::NodeType>, usize)> = vec![];
        for (weak_nt, mut target) in out {
            target.sort_by(|a, b| b.cmp(a));
            let child_idx = explore(target, nfa, accept_state, labeled, states);
            edges.push((weak_nt, child_idx));
        }
        states[idx].edges = edges;

        idx
    }

    let start = null_from(nfa, 0);
    explore(start, nfa, accept_state, &mut labeled, &mut states);
    states
}
```

- [ ] **Step 9: 修改 `build_dfa` 函数签名和实现（约第 501-532 行）**

将整个 `build_dfa` 函数替换为（移除 `node_types` 参数）：

```rust
/// Convert an indexed DFA state table to Arc<ContentMatch> using two-phase construction.
fn build_dfa(
    nfa: &[Vec<NfaEdge>],
) -> Arc<ContentMatch> {
    let states = build_dfa_indexed(nfa);

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

    // Phase 2: fill edges via OnceLock::set
    for (i, state) in states.iter().enumerate() {
        let edges: Vec<(std::sync::Weak<crate::model::schema::NodeType>, Arc<ContentMatch>)> = state
            .edges
            .iter()
            .map(|(weak_nt, idx)| (weak_nt.clone(), arcs[*idx].clone()))
            .collect();
        arcs[i].edges.set(edges).ok();
    }

    arcs.into_iter().next().unwrap()
}
```

- [ ] **Step 10: 修改 `check_for_dead_ends` 函数（约第 534-570 行）**

将整个 `check_for_dead_ends` 函数替换为（移除 `has_required` 参数，从 Weak 直接获取）：

```rust
/// Check that every non-valid-end state has at least one generatable outgoing type.
fn check_for_dead_ends(
    start: &Arc<ContentMatch>,
    expr_str: &str,
) -> Result<(), SyntaxError> {
    let mut work: Vec<Arc<ContentMatch>> = vec![start.clone()];
    let mut i = 0;
    while i < work.len() {
        let state = work[i].clone();
        i += 1;
        let mut dead = !state.valid_end;
        let mut node_names: Vec<String> = vec![];
        let state_edges = state.edges.get().map(|v| v.as_slice()).unwrap_or(&[]);
        for (weak_nt, next) in state_edges {
            if let Some(nt) = weak_nt.upgrade() {
                node_names.push(nt.name.clone());
                if !(nt.is_text || nt.has_required_attrs()) {
                    dead = false;
                }
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
```

- [ ] **Step 11: 修改 `ContentMatch` struct 的 `edges` 字段类型（约第 578-588 行）**

将：
```rust
    /// Outgoing edges: (type_name, next_state).
    /// OnceLock enables safe two-phase construction: set once, then read-only.
    edges: std::sync::OnceLock<Vec<(String, Arc<ContentMatch>)>>,
```
替换为：
```rust
    /// Outgoing edges: (weak ref to NodeType, next_state).
    /// OnceLock enables safe two-phase construction: set once, then read-only.
    /// Weak<NodeType> breaks the Arc cycle: NodeType → Arc<ContentMatch> → Weak<NodeType>.
    edges: std::sync::OnceLock<Vec<(std::sync::Weak<crate::model::schema::NodeType>, Arc<ContentMatch>)>>,
```

- [ ] **Step 12: 修改 `ContentMatch::parse` 函数签名（约第 652-670 行）**

将：
```rust
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
```
替换为：
```rust
pub fn parse(
    expr: &str,
    node_types: &IndexMap<String, Arc<crate::model::schema::NodeType>>,
    groups: &IndexMap<String, Vec<String>>,
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
    let dfa = build_dfa(&nfa);
    check_for_dead_ends(&dfa, expr)?;
    Ok(dfa)
}
```

- [ ] **Step 13: 修改 `ContentMatch::match_type` 方法（约第 606-612 行）**

将：
```rust
pub fn match_type(&self, type_name: &str) -> Option<Arc<ContentMatch>> {
    self.edges
        .get()
        .and_then(|edges| edges.iter().find(|(name, _)| name == type_name))
        .map(|(_, next)| next.clone())
}
```
替换为：
```rust
pub fn match_type(&self, type_name: &str) -> Option<Arc<ContentMatch>> {
    self.edges.get()?.iter().find_map(|(weak_nt, next)| {
        let nt = weak_nt.upgrade()?;
        if nt.name == type_name { Some(next.clone()) } else { None }
    })
}
```

- [ ] **Step 14: 修改 `ContentMatch::edge` 方法（约第 623-630 行）**

将：
```rust
pub fn edge(&self, n: usize) -> Result<(String, Arc<ContentMatch>), RangeError> {
    self.edges
        .get()
        .and_then(|edges| edges.get(n))
        .map(|(name, next)| (name.clone(), next.clone()))
        .ok_or_else(|| RangeError(format!("There's no {}th edge in this content match", n)))
}
```
替换为：
```rust
pub fn edge(&self, n: usize) -> Result<(String, Arc<ContentMatch>), RangeError> {
    self.edges
        .get()
        .and_then(|edges| edges.get(n))
        .map(|(weak_nt, next)| {
            let name = weak_nt.upgrade()
                .map_or_else(|| "<dropped>".to_string(), |nt| nt.name.clone());
            (name, next.clone())
        })
        .ok_or_else(|| RangeError(format!("There's no {}th edge in this content match", n)))
}
```

- [ ] **Step 15: 修改 `ContentMatch::compatible` 方法（约第 634-645 行）**

将：
```rust
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
```
替换为（使用 `Weak::ptr_eq`，比字符串比较更准确，与 TS 对象引用相等语义一致）：
```rust
pub fn compatible(&self, other: &ContentMatch) -> bool {
    let my_edges = self.edges.get().map(|v| v.as_slice()).unwrap_or(&[]);
    let other_edges = other.edges.get().map(|v| v.as_slice()).unwrap_or(&[]);
    for (weak_a, _) in my_edges {
        for (weak_b, _) in other_edges {
            if std::sync::Weak::ptr_eq(weak_a, weak_b) {
                return true;
            }
        }
    }
    false
}
```

- [ ] **Step 16: 修改 `ContentMatch::default_type` 方法（约第 677-683 行）**

将：
```rust
pub fn default_type(&self) -> Option<String> {
    self.edges
        .get()
        .and_then(|edges| edges.iter().find(|(name, _)| name != "text"))
        .map(|(name, _)| name.clone())
}
```
替换为：
```rust
pub fn default_type(&self) -> Option<String> {
    self.edges.get()?.iter().find_map(|(weak_nt, _)| {
        let nt = weak_nt.upgrade()?;
        if !nt.is_text && !nt.has_required_attrs() { Some(nt.name.clone()) } else { None }
    })
}
```

- [ ] **Step 17: 修改 `fill_before_search` 函数（约第 572-622 行）**

将整个 `fill_before_search` 函数替换为（移除 schema 参数，types 改用 `Weak<NodeType>`）：

```rust
/// DFS 辅助函数，搜索需要在 `after` 前插入的节点类型列表。
fn fill_before_search(
    match_state: &Arc<ContentMatch>,
    types: &[std::sync::Weak<crate::model::schema::NodeType>],
    seen: &mut Vec<Arc<ContentMatch>>,
    after: &crate::model::fragment::Fragment,
    to_end: bool,
    start_index: usize,
) -> Option<crate::model::fragment::Fragment> {
    if let Some(finished) = match_state.match_fragment(after, start_index, after.child_count()) {
        if !to_end || finished.valid_end {
            let nodes: Option<Vec<crate::model::node::Node>> = types
                .iter()
                .map(|weak_nt| {
                    let nt = weak_nt.upgrade()?;
                    nt.create_and_fill(None, vec![])
                })
                .collect();
            if let Some(nodes) = nodes {
                return Some(crate::model::fragment::Fragment::from_array(&nodes));
            }
        }
    }

    let edges = match_state.edges.get().map(|v| v.as_slice()).unwrap_or(&[]);
    for (weak_nt, next_state) in edges {
        if let Some(nt) = weak_nt.upgrade() {
            if !nt.is_text
                && !nt.has_required_attrs()
                && !seen.iter().any(|s| Arc::ptr_eq(s, next_state))
            {
                seen.push(next_state.clone());
                let mut new_types = types.to_vec();
                new_types.push(weak_nt.clone());
                if let Some(found) = fill_before_search(
                    next_state, &new_types, seen, after, to_end, start_index,
                ) {
                    return Some(found);
                }
            }
        }
    }
    None
}
```

- [ ] **Step 18: 修改 `ContentMatch::fill_before` 方法签名（约第 710-717 行）**

将：
```rust
pub fn fill_before(
    self: &Arc<Self>,
    schema: &Arc<crate::model::schema::Schema>,
    after: &crate::model::fragment::Fragment,
    to_end: bool,
    start_index: usize,
) -> Option<crate::model::fragment::Fragment> {
    let mut seen: Vec<Arc<ContentMatch>> = vec![self.clone()];
    fill_before_search(self, &[], &mut seen, schema, after, to_end, start_index)
}
```
替换为：
```rust
pub fn fill_before(
    self: &Arc<Self>,
    after: &crate::model::fragment::Fragment,
    to_end: bool,
    start_index: usize,
) -> Option<crate::model::fragment::Fragment> {
    let mut seen: Vec<Arc<ContentMatch>> = vec![self.clone()];
    fill_before_search(self, &[], &mut seen, after, to_end, start_index)
}
```

- [ ] **Step 19: 验证编译**

运行：`cargo build 2>&1 | grep "^error" | head -20`

预期：出现 schema.rs 的错误（Schema::new 调用旧签名），以及 tests_content.rs 的错误（fill_before 调用传了 schema）。这是预期的——后续 Task 修复。

- [ ] **Step 20: Commit**

```bash
cd /Users/aporicho/Desktop/md-editor
git add src/model/content.rs
git commit -m "refactor(model): change ContentMatch edges from String to Weak<NodeType>"
```

---

## Task 2: content.rs — 更新内部测试

**Files:** `src/model/content.rs`（`mod tests` 块，约第 722-1114 行）

content.rs 的内部测试使用 `make_state` 和 `parse_ok`，两者需要适配新的 Weak<NodeType> 类型。

- [ ] **Step 1: 替换 `make_state`、`two_state_dfa` 和相关测试辅助（约第 724-812 行）**

找到 `mod tests` 块中的 `make_state` 和辅助函数，将下列代码：

```rust
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
```

替换为：

```rust
/// 创建一个最小化的 NodeType Arc，用于单元测试（不需要完整 Schema）。
fn make_minimal_node_type(name: &str, is_inline: bool) -> Arc<crate::model::schema::NodeType> {
    use crate::model::schema::{NodeType, NodeSpec, Whitespace};
    Arc::new(NodeType {
        name: name.to_string(),
        is_block: !is_inline,
        is_text: name == "text",
        inline_content: false,
        groups: vec![],
        content_match: ContentMatch::empty(),
        mark_set: None,
        whitespace: Whitespace::Normal,
        spec: NodeSpec::default(),
        default_attrs: None,
    })
}

fn make_state(
    valid_end: bool,
    edges_input: Vec<(Arc<crate::model::schema::NodeType>, Arc<ContentMatch>)>,
) -> Arc<ContentMatch> {
    let cm = Arc::new(ContentMatch {
        valid_end,
        inline_content: false,
        edges: std::sync::OnceLock::new(),
        wrap_cache: std::sync::Mutex::new(std::collections::HashMap::new()),
    });
    let _ = cm.edges.set(
        edges_input.into_iter().map(|(nt, s)| (Arc::downgrade(&nt), s)).collect()
    );
    cm
}

// state_a --p--> state_b (valid_end=true)
fn two_state_dfa() -> Arc<ContentMatch> {
    let p_type = make_minimal_node_type("p", false);
    let state_b = make_state(true, vec![]);
    make_state(false, vec![(p_type, state_b)])
}
```

- [ ] **Step 2: 更新 `edge_count_and_edge` 测试（约第 766-778 行）**

将：
```rust
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
```
替换为：
```rust
#[test]
fn edge_count_and_edge() {
    let p_type = make_minimal_node_type("p", false);
    let h1_type = make_minimal_node_type("h1", false);
    let state_b = make_state(true, vec![]);
    let state_c = make_state(false, vec![]);
    let m = make_state(false, vec![(p_type, state_b), (h1_type, state_c)]);
    assert_eq!(m.edge_count(), 2);
    let (name, _) = m.edge(0).unwrap();
    assert_eq!(name, "p");
    let (name, _) = m.edge(1).unwrap();
    assert_eq!(name, "h1");
}
```

- [ ] **Step 3: 更新 `compatible_same_type` 和 `compatible_different_types` 测试（约第 785-800 行）**

将：
```rust
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
```
替换为：
```rust
#[test]
fn compatible_same_type() {
    let p_type = make_minimal_node_type("p", false);
    let state_b = make_state(true, vec![]);
    // 同一个 Arc<NodeType>，ptr_eq 返回 true
    let m1 = make_state(false, vec![(p_type.clone(), state_b.clone())]);
    let m2 = make_state(false, vec![(p_type.clone(), state_b.clone())]);
    assert!(m1.compatible(&m2));
}

#[test]
fn compatible_different_types() {
    let p_type = make_minimal_node_type("p", false);
    let h1_type = make_minimal_node_type("h1", false);
    let sb = make_state(true, vec![]);
    let sc = make_state(true, vec![]);
    let m1 = make_state(false, vec![(p_type, sb)]);
    let m2 = make_state(false, vec![(h1_type, sc)]);
    assert!(!m1.compatible(&m2));
}
```

- [ ] **Step 4: 替换 parse 测试的辅助函数（约第 802-844 行）**

删除以下所有辅助函数（不再需要）：
- `make_node_types`
- `make_groups`
- `make_required`
- `node_types_basic`
- `groups_basic`
- `required_basic`
- `parse_ok`（旧版）
- `matches`（如果引用了旧函数）

替换为以下新辅助函数：

```rust
// ---- helpers for parse tests ----

/// 为 parse 测试构建最小 Schema（含 paragraph/heading/hr/image/text/hard_break/code_block）。
fn parse_schema() -> &'static Arc<crate::model::schema::Schema> {
    use std::sync::OnceLock;
    use indexmap::indexmap;
    use crate::model::schema::{Schema, SchemaSpec, NodeSpec};
    static SCHEMA: OnceLock<Arc<crate::model::schema::Schema>> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        Schema::new(SchemaSpec {
            nodes: indexmap! {
                "paragraph".to_string() => NodeSpec {
                    group: Some("block".to_string()),
                    ..NodeSpec::default()
                },
                "heading".to_string() => NodeSpec {
                    group: Some("block".to_string()),
                    ..NodeSpec::default()
                },
                "horizontal_rule".to_string() => NodeSpec {
                    group: Some("block".to_string()),
                    ..NodeSpec::default()
                },
                "image".to_string() => NodeSpec {
                    inline: true,
                    group: Some("inline".to_string()),
                    ..NodeSpec::default()
                },
                "text".to_string() => NodeSpec {
                    inline: true,
                    group: Some("inline".to_string()),
                    ..NodeSpec::default()
                },
                "hard_break".to_string() => NodeSpec {
                    inline: true,
                    group: Some("inline".to_string()),
                    ..NodeSpec::default()
                },
                "code_block".to_string() => NodeSpec {
                    group: Some("block".to_string()),
                    ..NodeSpec::default()
                },
            },
            marks: indexmap! {},
            top_node: None,
        }).expect("parse_schema construction failed")
    })
}

fn parse_ok(expr: &str) -> Arc<ContentMatch> {
    let s = parse_schema();
    let groups: IndexMap<String, Vec<String>> = {
        let mut m = IndexMap::new();
        for (name, nt) in &s.nodes {
            for g in &nt.groups {
                m.entry(g.clone()).or_default().push(name.clone());
            }
        }
        m
    };
    ContentMatch::parse(expr, &s.nodes, &groups)
        .expect(&format!("parse should succeed for '{}'", expr))
}

fn matches(m: &Arc<ContentMatch>, types: &[&str]) -> bool {
    let mut cur: Option<Arc<ContentMatch>> = Some(m.clone());
    for &t in types {
        cur = cur.and_then(|state| state.match_type(t));
    }
    cur.map_or(false, |s| s.valid_end)
}
```

- [ ] **Step 5: 验证 content.rs 内部测试编译并通过**

运行：`cargo test --lib model::content::tests 2>&1 | tail -10`

预期：所有 content.rs 内部测试通过（parse_* 系列、match_type_* 系列等）

- [ ] **Step 6: Commit**

```bash
cd /Users/aporicho/Desktop/md-editor
git add src/model/content.rs
git commit -m "refactor(model): update content.rs internal tests for Weak<NodeType> edges"
```

---

## Task 3: schema.rs — 更新 Schema::new + create_and_fill

**Files:** `src/model/schema.rs`

- [ ] **Step 1: 删除 Schema::new 中的两个辅助 map（约第 857-874 行）**

找到以下三行并删除（只删 `is_inline_map` 和 `has_required_map`，保留 `groups_map`）：

```rust
// 删除这一行：
let is_inline_map: IndexMap<String, bool> = node_types
    .iter()
    .map(|(n, nt)| (n.clone(), nt.is_inline()))
    .collect();

// 保留 groups_map 不变

// 删除这一行：
let has_required_map: IndexMap<String, bool> = node_types
    .iter()
    .map(|(n, nt)| (n.clone(), nt.has_required_attrs()))
    .collect();
```

- [ ] **Step 2: 更新 Schema::new 中的 ContentMatch::parse 调用（约第 890 行）**

将：
```rust
let cm =
    ContentMatch::parse(expr, &is_inline_map, &groups_map, &has_required_map)
        .map_err(|e| SchemaError(e.0))?;
```
替换为：
```rust
let cm =
    ContentMatch::parse(expr, &node_types, &groups_map)
        .map_err(|e| SchemaError(e.0))?;
```

- [ ] **Step 3: 修改 `NodeType::create_and_fill` 签名和实现（约第 492-519 行）**

将：
```rust
pub fn create_and_fill(
    self: &Arc<Self>,
    schema: &Arc<Schema>,
    attrs: Option<&std::collections::HashMap<String, serde_json::Value>>,
    marks: Vec<crate::model::mark::Mark>,
) -> Option<crate::model::node::Node> {
    if self.is_text {
        return None;
    }
    let attrs = self.compute_attrs_for(attrs).ok()?;
    let content = crate::model::fragment::Fragment::empty();
    let matched = self.content_match.match_fragment(&content, 0, 0)?;
    let after = matched.fill_before(
        schema,
        &crate::model::fragment::Fragment::empty(),
        true,
        0,
    )?;
    Some(crate::model::node::Node::new(
        self.clone(),
        attrs,
        content.append(&after),
        marks,
    ))
}
```
替换为：
```rust
/// Like `create`, but fill in missing content automatically.
///
/// If no fitting content can be constructed, returns `None`.
pub fn create_and_fill(
    self: &Arc<Self>,
    attrs: Option<&std::collections::HashMap<String, serde_json::Value>>,
    marks: Vec<crate::model::mark::Mark>,
) -> Option<crate::model::node::Node> {
    if self.is_text {
        return None;
    }
    let attrs = self.compute_attrs_for(attrs).ok()?;
    let content = crate::model::fragment::Fragment::empty();
    // match_fragment(empty, 0, 0) 迭代 0 次，返回初始 match state 本身（content_match 起始状态）
    let matched = self.content_match.match_fragment(&content, 0, 0)?;
    let after = matched.fill_before(
        &crate::model::fragment::Fragment::empty(),
        true,
        0,
    )?;
    Some(crate::model::node::Node::new(
        self.clone(),
        attrs,
        content.append(&after),
        marks,
    ))
}
```

- [ ] **Step 4: 验证编译**

运行：`cargo build 2>&1 | grep "^error" | head -20`

预期：只剩 `tests_content.rs` 的错误（fill 辅助函数仍传 schema）。schema.rs 和 content.rs 应无报错。

- [ ] **Step 5: Commit**

```bash
cd /Users/aporicho/Desktop/md-editor
git add src/model/schema.rs
git commit -m "refactor(model): update Schema::new and remove schema param from create_and_fill"
```

---

## Task 4: tests_content.rs — 移除 fill/fill3 中的 schema 参数

**Files:** `src/model/tests_content.rs`

- [ ] **Step 1: 修改 `fill` 辅助函数**

找到 `fn fill(...)` 函数，将函数体内的 `let schema = test_schema();` 和 `fill_before(schema, ...)` 调用更新。

将 `fill` 函数整体替换为：

```rust
fn fill(expr: &str, before_content: Fragment, after_content: Fragment, expected: Option<Fragment>) {
    let cm = get(expr);
    let matched = cm
        .match_fragment(&before_content, 0, before_content.child_count())
        .unwrap_or_else(|| panic!("fill: matchFragment failed for before_content in '{}'", expr));
    let filled = matched.fill_before(&after_content, true, 0);
    match expected {
        Some(exp) => {
            let filled = filled
                .unwrap_or_else(|| panic!("fill: expected Some but got None for '{}'", expr));
            assert!(
                filled.eq(&exp),
                "fill '{}': result mismatch — got size {}, expected size {}",
                expr,
                filled.size,
                exp.size
            );
        }
        None => assert!(
            filled.is_none(),
            "fill '{}': expected None but got Some",
            expr
        ),
    }
}
```

- [ ] **Step 2: 修改 `fill3` 辅助函数**

将 `fill3` 函数整体替换为：

```rust
fn fill3(
    expr: &str,
    before: Fragment,
    mid: Fragment,
    after: Fragment,
    expected_left: Option<Fragment>,
    expected_right: Option<Fragment>,
) {
    let cm = get(expr);
    let a = cm
        .match_fragment(&before, 0, before.child_count())
        .unwrap_or_else(|| panic!("fill3: matchFragment failed for `before` in '{}'", expr))
        .fill_before(&mid, false, 0);
    let b = a.as_ref().and_then(|fill_a| {
        let combined = before.append(fill_a).append(&mid);
        cm.match_fragment(&combined, 0, combined.child_count())
            .unwrap_or_else(|| panic!("fill3: matchFragment failed for combined in '{}'", expr))
            .fill_before(&after, true, 0)
    });
    match expected_left {
        Some(left) => {
            let a = a.unwrap_or_else(|| panic!("fill3 '{}': a should be Some", expr));
            assert!(
                a.eq(&left),
                "fill3 '{}': a mismatch — got size {}, expected size {}",
                expr,
                a.size,
                left.size
            );
            let right =
                expected_right.unwrap_or_else(|| panic!("fill3 '{}': expected_right is None when expected_left is Some", expr));
            let b = b.unwrap_or_else(|| panic!("fill3 '{}': b should be Some", expr));
            assert!(
                b.eq(&right),
                "fill3 '{}': b mismatch — got size {}, expected size {}",
                expr,
                b.size,
                right.size
            );
        }
        None => assert!(b.is_none(), "fill3 '{}': b should be None", expr),
    }
}
```

- [ ] **Step 3: 运行 fill 测试**

运行：`cargo test tests_content 2>&1 | tail -10`

预期：24 个 fill/fill3 测试全部 PASS

- [ ] **Step 4: 运行全量测试**

运行：`cargo test 2>&1 | tail -5`

预期：249 passed，0 failed

- [ ] **Step 5: Commit**

```bash
cd /Users/aporicho/Desktop/md-editor
git add src/model/tests_content.rs
git commit -m "refactor(model): remove schema param from fill/fill3 test helpers"
```

---

## Task 5: 更新 3.4-c 计划文档

**Files:** `docs/superpowers/plans/2026-04-05-phase-3-4c-find-wrapping.md`

3.4-c 计划中的 `find_wrapping` 和 `compute_wrapping` 仍然引用 `schema: &Arc<Schema>` 参数。现在这些方法也不再需要 schema，因为直接通过 `Weak::upgrade()` 获取 NodeType。

- [ ] **Step 1: 更新 Task 2 中 find_wrapping 的 stub 签名**

找到：
```rust
pub fn find_wrapping(
    self: &Arc<Self>,
    schema: &Arc<crate::model::schema::Schema>,
    target: &str,
) -> Option<Vec<String>> {
    todo!("find_wrapping: implement in Phase 3.4-c")
}
```
替换为：
```rust
pub fn find_wrapping(
    self: &Arc<Self>,
    target: &str,
) -> Option<Vec<String>> {
    todo!("find_wrapping: implement in Phase 3.4-c")
}
```

- [ ] **Step 2: 更新 Task 5 中 compute_wrapping 函数**

找到 `fn compute_wrapping(` 的函数签名：
```rust
fn compute_wrapping(
    match_state: &Arc<ContentMatch>,
    schema: &Arc<crate::model::schema::Schema>,
    target: &str,
) -> Option<Vec<String>> {
```
替换为：
```rust
fn compute_wrapping(
    match_state: &Arc<ContentMatch>,
    target: &str,
) -> Option<Vec<String>> {
```

在函数体中，找到所有 `schema.nodes.get(type_name)` 调用，替换为 `weak_nt.upgrade()`：

将：
```rust
        let edges = current_match.edges.get().map(|v| v.as_slice()).unwrap_or(&[]);
        for (type_name, next_state) in edges {
            if let Some(nt) = schema.nodes.get(type_name) {
                if !nt.is_leaf()
                    && !nt.has_required_attrs()
                    && !seen.contains(type_name.as_str())
                    && (!has_type || next_state.valid_end)
                {
                    seen.insert(type_name.clone());
                    let child_match = nt.content_match.clone();
```
替换为：
```rust
        let edges = current_match.edges.get().map(|v| v.as_slice()).unwrap_or(&[]);
        for (weak_nt, next_state) in edges {
            if let Some(nt) = weak_nt.upgrade() {
                if !nt.is_leaf()
                    && !nt.has_required_attrs()
                    && !seen.contains(nt.name.as_str())
                    && (!has_type || next_state.valid_end)
                {
                    seen.insert(nt.name.clone());
                    let child_match = nt.content_match.clone();
```

- [ ] **Step 3: 更新 Task 5 中 find_wrapping 的实现**

找到：
```rust
pub fn find_wrapping(
    self: &Arc<Self>,
    schema: &Arc<crate::model::schema::Schema>,
    target: &str,
) -> Option<Vec<String>> {
    {
        let cache = self.wrap_cache.lock().unwrap();
        if let Some(cached) = cache.get(target) {
            return cached.clone();
        }
    }
    let result = compute_wrapping(self, schema, target);
    {
        let mut cache = self.wrap_cache.lock().unwrap();
        cache.insert(target.to_string(), result.clone());
    }
    result
}
```
替换为：
```rust
pub fn find_wrapping(
    self: &Arc<Self>,
    target: &str,
) -> Option<Vec<String>> {
    {
        let cache = self.wrap_cache.lock().unwrap();
        if let Some(cached) = cache.get(target) {
            return cached.clone();
        }
    }
    let result = compute_wrapping(self, target);
    {
        let mut cache = self.wrap_cache.lock().unwrap();
        cache.insert(target.to_string(), result.clone());
    }
    result
}
```

- [ ] **Step 4: 更新 tests_content.rs 中的 find_wrapping 测试（Task 3）**

找到三个 find_wrapping 测试（`find_wrapping_direct_fit`、`find_wrapping_one_level`、`find_wrapping_impossible`），将调用方式从：
```rust
let result = doc_cm.find_wrapping(schema, "paragraph");
```
更新为：
```rust
let result = doc_cm.find_wrapping("paragraph");
```

同时删除每个测试中的 `let schema = test_schema();` 行。

- [ ] **Step 5: 更新 3.4-c 计划的"关键已有 API"部分**

在计划顶部的 API 参考中，找到：
```
pub fn fill_before(self: &Arc<Self>, schema: &Arc<Schema>, after: &Fragment, to_end: bool, start_index: usize) -> Option<Fragment>
```
替换为：
```
pub fn fill_before(self: &Arc<Self>, after: &Fragment, to_end: bool, start_index: usize) -> Option<Fragment>
```

- [ ] **Step 6: 更新 3.4-c 计划的"设计决策说明"部分**

将最后的"设计决策说明"替换为：

```
### 设计决策说明

`check`/`can_replace`/`can_replace_with`/`can_append` 均**不需要** `schema` 参数——Node 直接持有 `Arc<NodeType>`，方法通过 `content_match_at` 和 `allows_marks` 工作，均不需要 schema 查找。

`find_wrapping` 同样**不需要** `schema` 参数——`ContentMatch::edges` 已改为 `Weak<NodeType>`（Decision 28 重构），直接 `upgrade()` 获取 NodeType，无需外部传入 schema。

这与 TypeScript 原实现完全对称。
```

- [ ] **Step 7: Commit**

```bash
cd /Users/aporicho/Desktop/md-editor
git add docs/superpowers/plans/2026-04-05-phase-3-4c-find-wrapping.md
git commit -m "docs: update Phase 3.4-c plan — remove schema param from find_wrapping after Weak<NodeType> refactor"
```

---

## Task 6: 最终验证

- [ ] **Step 1: 全量测试**

运行：`cargo test 2>&1 | tail -5`

预期：`test result: ok. 249 passed; 0 failed; ...`

- [ ] **Step 2: 确认无 todo!() 遗留在 refactor 范围内**

运行：`grep -n "Decision 28\|is_inline_map\|has_required_map\|schema.*fill_before\|fill_before.*schema" src/model/content.rs src/model/schema.rs src/model/tests_content.rs`

预期：无输出（所有相关代码已清理）

---

## 自查

**Spec 覆盖：**
- ✅ `edges` 改为 `Weak<NodeType>`（Task 1）
- ✅ DFA builder 不再依赖外部 node_types 参数（Task 1）
- ✅ `fill_before` 去掉 schema 参数（Task 1）
- ✅ `create_and_fill` 去掉 schema 参数（Task 3）
- ✅ `Schema::new` 更新调用侧（Task 3）
- ✅ `tests_content.rs` fill 辅助函数更新（Task 4）
- ✅ 3.4-c 计划更新（Task 5）

**类型一致性：**
- `fill_before_search` 的 `types` 参数：`&[Weak<NodeType>]`（Task 1）
- `fill_before` 签名：`(self: &Arc<Self>, after: &Fragment, to_end: bool, start_index: usize)`（Task 1, Task 4）
- `create_and_fill` 签名：`(self: &Arc<Self>, attrs: Option<...>, marks: Vec<Mark>)`（Task 3）
- `find_wrapping` 签名（在 3.4-c 计划中）：`(self: &Arc<Self>, target: &str) -> Option<Vec<String>>`（Task 5）
