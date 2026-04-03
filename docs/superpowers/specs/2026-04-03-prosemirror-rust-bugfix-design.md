# Design Spec: ProseMirror Rust 移植 Bug 修复与测试套件

**日期**: 2026-04-03
**项目**: md-editor — ProseMirror `prosemirror-model` Rust 移植
**范围**: `src/model/` 下 7 个文件的逻辑错误修复 + 完整单元测试套件
**方案**: 方案 B — 三阶段流水线（紧急修复 → Schema 扩展 + B 类 bug → 测试套件）

---

## 背景

已完成的 Rust 移植文件（`mark.rs`、`content.rs`、`fragment.rs`、`node.rs`、`diff.rs`、`resolvedpos.rs`、`replace.rs`）通过代码审查发现 16 处问题，分为：

- **严重**（必然 panic 或完全错误结果）：4 处
- **重要**（功能缺失，影响核心用例）：8 处（其中 5 处依赖 schema 接口）
- **次要**（行为差异）：4 处

---

## Phase 1 — 无依赖紧急修复

> **注意**：fix 1.3（block_range）依赖 Phase 2 中 `NodeType.inline_content` 字段，需在 Phase 2 的 schema.rs 改动先行提交后再合并 1.3 的修复。Phase 1 其余三处（1.1、1.2、1.4）完全无外部依赖，可立即实施。

### 1.1 `content.rs` — DFA 构建 Arc::into_inner panic

**问题**（行 600–604）：`explore` 函数先将 `Arc::clone(&state)` 插入 `labeled` map（引用计数=2），随即调用 `Arc::into_inner(state).unwrap()`（要求引用计数=1），必然返回 `None` 导致 panic。整个 `ContentMatch::parse` 完全不可用。

**修复**：采用 **arena + index 方案**，彻底回避 Arc 循环引用导致的内存泄漏和 into_inner 问题：

1. **收集阶段**：`explore` 函数操作一个 `Vec<BuildNode>` arena，每个节点用 `usize` 下标表示，出边目标也是 `usize`。递归前先注册下标（空 edges）防止无限循环。
2. **构造阶段**：arena 构建完毕后，按拓扑顺序（或反向遍历）将每个 `BuildNode` 转为 `ContentMatch`，出边指向对应下标的 `Arc<ContentMatch>`，通过临时 `Vec<Arc<ContentMatch>>` 按下标索引完成连接——无循环 Arc，无 Mutex。

```rust
struct BuildNode {
    valid_end: bool,
    // 出边：(NodeType, 目标 arena 下标)
    edges: Vec<(Arc<NodeType>, usize)>,
}

// 返回起始节点在 arena 中的下标
fn explore_into_arena(
    nfa: &[Vec<NfaEdge>],
    states: &[usize],
    accept: usize,
    key_to_idx: &mut HashMap<String, usize>,
    arena: &mut Vec<BuildNode>,
) -> usize {
    let key = states_key(states);
    if let Some(&idx) = key_to_idx.get(&key) { return idx; }
    let idx = arena.len();
    arena.push(BuildNode { valid_end: states.contains(&accept), edges: vec![] });
    key_to_idx.insert(key, idx);
    // 收集此状态集的所有出边（同原 explore 逻辑）...
    for (node_type, target_states) in out {
        let target_idx = explore_into_arena(nfa, &target_states, accept, key_to_idx, arena);
        arena[idx].edges.push((node_type, target_idx));
    }
    idx
}

fn assemble(arena: Vec<BuildNode>, root_idx: usize) -> Arc<ContentMatch> {
    // 步骤 1：为每个 BuildNode 创建对应的 Arc<ContentMatch>（此时 next 为空）
    // ContentMatch.next 字段改为 OnceLock<Vec<MatchEdge>>，支持一次性写入。
    let nodes: Vec<Arc<ContentMatch>> = arena.iter()
        .map(|b| Arc::new(ContentMatch { valid_end: b.valid_end, next: OnceLock::new() }))
        .collect();
    // 步骤 2：填充出边。此时每个 Arc 引用计数=1（仅 nodes 持有），
    // 用 Arc::get_mut 安全获取可变引用（无需 unsafe），写入后引用计数不变。
    for (i, build_node) in arena.iter().enumerate() {
        let edges: Vec<MatchEdge> = build_node.edges.iter()
            .map(|(nt, target_idx)| MatchEdge {
                node_type: Arc::clone(nt),
                next: Arc::clone(&nodes[*target_idx]),
            })
            .collect();
        // OnceLock::set 只能成功一次，此处每个节点只填充一次，安全。
        nodes[i].next.set(edges).expect("assemble: duplicate edge initialization");
    }
    Arc::clone(&nodes[root_idx])
}
```

**关键约束**：`ContentMatch` 结构体的 `next` 字段需从 `Vec<MatchEdge>` 改为 `OnceLock<Vec<MatchEdge>>`，所有读取 `next` 的代码（`match_type`、`inline_content`、`edge` 等）改为 `self.next.get().map(...)` 或 `self.next.get().unwrap_or(&[])`（DFA 构建完成后 `next` 必然已初始化）。

### 1.2 `resolvedpos.rs:148` — `after()` 节点层级错误

**问题**：`after(depth=d)` 使用 `self.path[d-1].node.node_size()`（第 d-1 层即父节点的大小），应使用第 d 层节点的大小。

**修复**：
```rust
// 修复前
let entry = &self.path[d - 1];
Ok(entry.start + entry.node.node_size())

// 修复后
Ok(self.path[d - 1].start + self.path[d].node.node_size())
```

路径映射关系验证：
- `self.path[d-1].start` ↔ TS `path[d*3 - 1]`（d 层节点在文档中的绝对起始位置）✓
- `self.path[d].node` ↔ TS `path[d*3]`（d 层节点本体）✓

### 1.3 `resolvedpos.rs:270` — `block_range()` 起始深度条件错误

**问题**：TS 条件为 `this.parent.inlineContent`（父节点是 textblock），Rust 错误使用 `self.parent().content.size == 0`（父节点内容为空）。当光标在非空 textblock 内时，TS 从 `depth-1` 开始搜索，Rust 从 `depth` 开始，返回错误的块级范围。

**修复**（依赖 Phase 2 的 `NodeType.inline_content` 字段）：
```rust
let start_depth = if self.parent().node_type.inline_content || self.pos == other.pos {
    self.depth().saturating_sub(1)
} else {
    self.depth()
};
```

### 1.4 `resolvedpos.rs:74` — 负数深度转 usize wrapping panic

**问题**：`(self.depth() as isize + d) as usize` 当结果为负时 wrapping 为极大 usize，导致后续数组越界 panic。

**修复**：
```rust
Some(d) if d < 0 => {
    let result = self.depth() as isize + d;
    assert!(result >= 0, "Depth {} out of range (actual depth: {})", d, self.depth());
    result as usize
}
```

---

## Phase 2 — Schema 扩展 + B 类 Bug 修复

### 2.1 `schema.rs` — NodeType 扩展

新增字段和方法：

```rust
pub struct NodeType {
    pub name: String,
    pub groups: Vec<String>,
    pub is_block: bool,
    pub is_text: bool,
    pub inline_content: bool,                        // 新增：是否为 textblock
    pub mark_set: Option<Vec<Arc<MarkType>>>,        // 新增：None = 全部允许
    pub content_match: Option<Arc<ContentMatch>>,
}

impl NodeType {
    /// 是否包含行内内容（textblock）。
    pub fn is_textblock(&self) -> bool {
        self.is_block && self.inline_content
    }

    /// content expression 兼容性检查（用于 check_join）。
    /// 对应 TS: compatibleContent(other)
    pub fn compatible_content(&self, other: &NodeType) -> bool {
        std::ptr::eq(self, other)
            || match (&self.content_match, &other.content_match) {
                (Some(a), Some(b)) => a.compatible(b),
                _ => false,
            }
    }

    /// 检查 fragment 是否为合法内容。
    /// 对应 TS: validContent(content)
    pub fn valid_content(&self, content: &Fragment) -> bool {
        let cm = match &self.content_match {
            Some(cm) => cm,
            None => return content.size == 0,
        };
        let result = cm.match_fragment(content, 0, content.child_count());
        match result {
            Some(state) if state.valid_end => {}
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

    /// 是否允许特定 MarkType。
    /// 注意：比较用 name 字段，不用 Arc::ptr_eq（ptr_eq 对新建 Arc 永远为 false）。
    pub fn allows_mark_type(&self, mt: &Arc<MarkType>) -> bool {
        match &self.mark_set {
            None => true,
            Some(set) => set.iter().any(|m| m.name == mt.name),
        }
    }

    /// 是否允许整个标记集合中的所有标记。
    /// `Mark.mark_type` 字段类型为 `Arc<MarkType>`。
    pub fn allows_marks(&self, marks: &[Mark]) -> bool {
        self.mark_set.is_none()
            || marks.iter().all(|m| self.allows_mark_type(&m.mark_type))
    }
}
```

### 2.2 `schema.rs` — MarkType 新增 `inclusive`

```rust
pub struct MarkType {
    pub name: String,
    pub rank: usize,
    pub excluded: Vec<Arc<MarkType>>,
    pub inclusive: Option<bool>,  // None 表示默认 true
}
```

### 2.3 `node.rs` — 新增 `content_match_at`、`can_replace`、`inline_content`

`Node.node_type` 字段类型为 `Arc<NodeType>`，`match_type` 接收 `&Arc<NodeType>`，调用时直接传 `&child.node_type`。

```rust
impl Node {
    /// 在子节点索引 index 处的 ContentMatch 状态。
    pub fn content_match_at(&self, index: usize) -> Option<Arc<ContentMatch>> {
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
        let two = one.and_then(|s| s.match_fragment(&self.content, to, self.content.child_count()));
        match two {
            Some(s) if s.valid_end => {}
            _ => return false,
        }
        for i in start..end {
            if !self.node_type.allows_marks(&replacement.child(i).marks) {
                return false;
            }
        }
        true
    }

    /// 是否包含行内内容（委托到 NodeType）。
    pub fn inline_content(&self) -> bool {
        self.node_type.inline_content
    }
}
```

### 2.4 `content.rs` — 补全 `match_fragment` 和 `fill_before`

**`match_fragment`**（依赖 `Fragment::child` 和 `match_type`）：
```rust
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
```

**`fill_before`**（显式队列 BFS，与 TS 原版语义一致，保证返回最短填充序列）：

`seen` 使用 `HashSet<usize>`（存 `Arc::as_ptr(x) as usize`）避免裸指针悬空风险；显式队列保证 BFS 语义（最少节点优先）。`Node.node_type` 字段类型为 `Arc<NodeType>`。

```rust
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
                queue.push_back(Entry { state: Arc::clone(&edge.next), types: new_types });
            }
        }
    }
    None
}
```

### 2.5 `content.rs` — 修复 `find_wrapping` 完整 via 链

**问题**：BFS 中 `via` 字段用 `Some(Box::new(Active { ..., via: None }))` 只保留一层，多层包裹时中间节点丢失。

**修复**：改用索引回溯（`via: Option<usize>` 指向 `active` 数组中前驱的下标），完整链路保存在数组中：

```rust
struct Active {
    content_match: Arc<ContentMatch>,
    node_type: Option<Arc<NodeType>>,
    via: Option<usize>,  // 指向 active Vec 中前驱的下标
}

let mut active: Vec<Active> = vec![Active { ..., via: None }];
let mut head = 0;
while head < active.len() {
    let match_ref = Arc::clone(&active[head].content_match);
    if match_ref.match_type(target).is_some() {
        // 从 active[head] 沿 via 链回溯，收集 node_type 并 reverse
        let mut result = vec![];
        let mut idx = head;
        while let Some(nt) = &active[idx].node_type {
            result.push(Arc::clone(nt));
            idx = active[idx].via.unwrap();
        }
        result.reverse();
        return Some(result);
    }
    // 提前读取 is_root，避免在 for 循环内不可变借用 active 时再 push（可变借用冲突）
    let is_root = active[head].node_type.is_none();
    // match_ref 是 Arc::clone，不持有 active 的借用，for 循环内 push 安全
    for edge in match_ref.next.get().map(|v| v.as_slice()).unwrap_or(&[]) {
        let nt = &edge.node_type;
        if !nt.is_leaf() && !nt.has_required_attrs()
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
```

### 2.6 `resolvedpos.rs` — 完成 `marks()` 和新增 `marks_across()`

**`marks()` inclusive 过滤**：
```rust
// 在返回前过滤
let mut result = main.marks.clone();
let other_marks = other.map(|o| &o.marks);
let mut i = result.len();
while i > 0 {
    i -= 1;
    if result[i].mark_type.inclusive == Some(false) {
        if other_marks.map_or(true, |om| !result[i].is_in_set(om)) {
            result = result[i].remove_from_set(&result);
        }
    }
}
result
```

**新增 `marks_across`**：
```rust
pub fn marks_across(&self, end: &ResolvedPos) -> Option<Vec<Mark>> {
    let after = self.parent().content.maybe_child(self.index(None))?;
    if !after.is_inline() {
        return None;
    }
    let mut marks = after.marks.clone();
    let next = end.parent().content.maybe_child(end.index(None));
    let mut i = marks.len();
    while i > 0 {
        i -= 1;
        if marks[i].mark_type.inclusive == Some(false) {
            if next.map_or(true, |n| !marks[i].is_in_set(&n.marks)) {
                marks = marks[i].remove_from_set(&marks);
            }
        }
    }
    Some(marks)
}
```

### 2.7 `replace.rs` — 修复 B 类三处 + 错误处理

- **`check_join`**：改为 `NodeType::compatible_content`
- **`close()`**：调用 `node.node_type.check_content(&content)?`
- **`insert_into()`**：当 parent 存在时调用 `parent.can_replace(index, index, insert, 0, insert.child_count())`
- **`remove_range()` 非平坦范围**：改为返回 `Err(ReplaceError("Removing non-flat range".into()))`，函数签名改为 `Result<Fragment, ReplaceError>`

### 2.8 `fragment.rs` — `cut_by_index` 不合并文本节点

```rust
pub fn cut_by_index(&self, from: usize, to: usize) -> Fragment {
    if from == to { return Fragment::empty(); }
    if from == 0 && to == self.content.len() { return self.clone(); }
    let content = self.content[from..to].to_vec();
    let size = content.iter().map(|n| n.node_size()).sum();
    Fragment::new(content, size)
}
```

---

## Phase 3 — 完整测试套件

### 测试辅助工具（`src/model/test_helpers.rs` 或各文件内 `mod tests`）

```rust
fn make_node_type(name: &str, is_block: bool, inline_content: bool) -> Arc<NodeType>
fn make_mark_type(name: &str, rank: usize, inclusive: Option<bool>) -> Arc<MarkType>
fn make_text_node(text: &str, marks: Vec<Mark>) -> Node
fn make_block_node(node_type: Arc<NodeType>, children: Vec<Node>) -> Node
fn make_fragment(nodes: Vec<Node>) -> Fragment
fn make_schema_with_content(expr: &str) -> Arc<ContentMatch>  // 快速构造 DFA
```

### 各文件测试覆盖

**`mark.rs` 测试**：
- `add_to_set`：幂等（已存在的 mark 返回原集合）、按 rank 排序插入、排斥关系（this 排斥 other → 跳过 other、other 排斥 this → 返回原集合）
- `remove_from_set`：存在时移除、不存在时返回原集合
- `same_set`：长度不同、内容不同、完全相同
- `set_from`：空、单个、多个（验证排序）

**`content.rs` 测试**：
- `parse`：空字符串 → empty、`"paragraph"` 单节点、`"paragraph+"` plus、`"paragraph*"` star、`"heading{1,3}"` range、`"block | inline"` choice、`"(a b)+"` 括号组合
- DFA 构建不 panic（覆盖 1.1 修复）
- `match_type`：正确推进状态、未知类型返回 None
- `match_fragment`：全匹配、部分匹配、不匹配
- `fill_before`：空 after、需要填充一个节点、to_end=true 时验证 validEnd
- `find_wrapping`：直接可匹配（返回空数组）、需要一层包裹、需要两层包裹（验证路径完整性）

**`fragment.rs` 测试**：
- `from_array`：空数组、相邻同标记文本合并、不同标记不合并
- `cut`：全范围、文本节点截取、块节点截取、边界值
- `cut_by_index`：不合并相邻文本（覆盖 2.8 修复）
- `append`：空+非空、末首文本合并
- `find_index`：pos=0、pos=size、中间位置

**`diff.rs` 测试**：
- `find_diff_start`：完全相同返回 None、第一个节点不同、文本节点内部差异
- `find_diff_end`：完全相同返回 None、最后节点不同、文本节点末尾差异

**`resolvedpos.rs` 测试**：
- `resolve`：doc 根节点（depth=0）、段落内（depth=1）、嵌套块（depth=2）
- `after()`：各深度正确值（覆盖 1.2 修复）
- `before()`：各深度正确值
- `start()` / `end()`：验证与 before/after 的一致性
- `text_offset()`：文本中间位置 vs 节点边界
- `resolve_depth()` 负数边界：`node(Some(-1))` 在 depth=1 时等价于 `node(Some(0))`；`node(Some(-2))` 在 depth=1 时触发 assert/panic；`node(Some(-1))` 在 depth=0 时（result=-1）也应触发 assert/panic（覆盖 1.4 修复）
- `block_range()`：同一 textblock 内（应从 depth-1 开始）、跨 block（覆盖 1.3 修复）
- `marks()`：空父节点、文本内部、位置边界 inclusive=false 过滤（覆盖 2.5 修复）
- `marks_across()`：行内节点、非行内返回 None

**`replace.rs` 测试**：
- `Slice::empty`、`Slice::max_open`
- `replace`：空 slice（纯删除）、平坦插入、open_start/end 不一致报错
- `check_join`：compatible_content 兼容通过、不兼容报错（覆盖 2.7 修复）
- `remove_range`：平坦范围、非平坦范围返回 Err（覆盖 2.7 修复）
- `insert_into`：canReplace 通过插入、不通过返回 None

**`schema.rs` 测试**：
- `compatible_content`：同类型、兼容异类型、不兼容
- `valid_content` / `check_content`：合法 fragment、非法 fragment
- `allows_marks`：mark_set=None 全通、mark_set 部分允许

---

## 文件修改清单

| 文件 | Phase | 变更类型 |
|------|-------|---------|
| `src/model/content.rs` | 1, 2 | 重构 DFA explore、新增 match_fragment/fill_before、修复 find_wrapping |
| `src/model/resolvedpos.rs` | 1, 2 | 修复 after/block_range/depth、完成 marks/marks_across |
| `src/model/schema.rs` | 2 | 新增 inline_content/mark_set/compatible_content 等 |
| `src/model/node.rs` | 2 | 新增 content_match_at/can_replace/inline_content |
| `src/model/replace.rs` | 2 | 修复 check_join/close/insert_into/remove_range |
| `src/model/fragment.rs` | 2 | 修复 cut_by_index |
| `src/model/diff.rs` | — | 仅加测试，无逻辑变更 |
| `src/model/mark.rs` | — | 仅加测试，无逻辑变更 |

---

## 成功标准

1. `cargo build` 零 warning 零 error
2. `cargo test` 全部通过
3. 无任何 `unwrap()` 在非测试代码的非预期路径上（DFA 构建路径全部改为 Result/安全构造）
4. `resolvedpos::after()` 返回值与 TS 参考实现一致（可对照手动验证）
5. `ContentMatch::parse` 对合法 content 表达式不 panic
