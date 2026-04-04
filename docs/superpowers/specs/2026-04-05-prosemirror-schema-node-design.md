# Design Spec: ProseMirror Rust 移植 Phase 2 — Schema + Node 方法完善

**日期**: 2026-04-05
**项目**: md-editor — ProseMirror `prosemirror-model` Rust 移植
**前置**: `docs/superpowers/specs/2026-04-03-prosemirror-rust-bugfix-design.md`（Bug 修复已完成）
**目标**: 完整 API 兼容

---

## 背景与动机

Bug 修复（Phase 1 & 2 of 上一个 spec）已实现。剩余三类问题：

1. **设计缺陷**
   - `content_match: None` 同时表示"叶节点"和"无约束"，语义歧义
   - 无 `Schema` 结构体，类型注册靠散落的 `HashMap` 临时传递
   - 比较语义退步：`Mark::eq`、`MarkType::excludes` 等用名称字符串比较，而非 TS 的指针相等

2. **功能缺失**
   - `Node::replace()`、`Node::slice()` 未实现，对应 TS 测试无法运行
   - `Node::node_at()`、`text_content()`、`nodes_between()`、`check()` 缺失
   - `create_and_fill()` 只填一层，无法递归填充嵌套必需内容

3. **测试覆盖缺口**
   - `test-replace.ts` 和 `test-slice.ts` 无对应 Rust 测试

---

## 方案选择

**采用方案 A：Schema 优先**

- Schema 是根基。先建好它，ptr_eq 语义自动恢复，后续所有工作站在正确语义上
- 方案 B（replace/slice 优先，Schema 推后）会引入临时技术债，Schema 重构时需二次修改
- 方案 C（跳过 Schema）无法实现真正的 API 兼容

---

## Phase 1 — Schema 结构体

### 目标

建立中央类型注册表，保证同一 Schema 内同名类型是唯一 Arc 实例，恢复 ptr_eq 比较语义。

### 新增数据类型

```rust
// src/model/schema.rs

/// 节点类型的构建描述
pub struct NodeSpec {
    pub content: Option<String>,   // content 表达式，如 "paragraph+"
    pub marks: Option<String>,     // 允许的 mark："_"=全部，""=无，空格分隔列表
    pub group: Option<String>,     // 所属分组，空格分隔，如 "block"
    pub inline: bool,              // 是否行内节点
    pub is_text: bool,             // 是否文本节点
}

/// 标记类型的构建描述
pub struct MarkSpec {
    pub rank: usize,
    pub excludes: Option<String>,  // 排斥的 mark 名："_"=排斥所有，""=仅排斥自身
    pub inclusive: Option<bool>,
}

/// Schema 构建规格
pub struct SchemaSpec {
    pub nodes: Vec<(String, NodeSpec)>,  // 有序；第一个节点为 topNode
    pub marks: Vec<(String, MarkSpec)>,
}

/// 中央类型注册表
pub struct Schema {
    pub nodes: HashMap<String, Arc<NodeType>>,
    pub marks: HashMap<String, Arc<MarkType>>,
    pub top_node_type: Arc<NodeType>,
}

/// Schema 构建错误
pub enum SchemaError {
    EmptyNodes,
    ContentParseError(String),
    UnknownMarkRef(String),
    UnknownNodeRef(String),
}
```

### Schema::new 两遍遍历

**第一遍**：注册所有 NodeType 和 MarkType（`content_match = None`，`mark_set = None`）。

**第二遍**：利用完整的 node_types 映射，填充每个 NodeType 的 `content_match`、`inline_content`、`mark_set`。`Arc::get_mut` 在此安全（第一遍结束后每个 Arc 只被本 HashMap 持有，引用计数 = 1）。

**构建完成后**：所有类型固化为不可变，Schema 以 `Arc<Schema>` 对外暴露。

### 比较语义恢复

Schema 建好后，同一 Schema 内同名类型是同一 Arc 实例：

| 方法 | 修复前 | 修复后 |
|------|--------|--------|
| `MarkType::excludes` | `e.name == other.name` | `Arc::ptr_eq(e, other)` |
| `Mark::eq`（type 部分）| `mark_type.name ==` | `Arc::ptr_eq(&mark_type, &other.mark_type)` |
| `ContentMatch::match_type` | `edge.node_type.name ==` | `Arc::ptr_eq(&edge.node_type, node_type)` |
| `ContentMatch::compatible` | `a.node_type.name ==` | `Arc::ptr_eq(&a.node_type, &b.node_type)` |

### 对现有代码的影响

- `ContentMatch::parse` 签名不变，仍接受 `&HashMap<String, Arc<NodeType>>`
- 现有测试辅助函数（`make_nt`、`test_schema` 等）可保留，Schema 接口作为新增路径
- 不需要在 NodeType/MarkType 上存 back-reference（`nodeType.schema` 在当前使用中不需要）

---

## Phase 2 — Node::slice() + Node::replace()

### 目标

将已有的 `replace.rs` 算法挂载到 `Node` 上，移植 test-replace.ts 和 test-slice.ts。

### Node::slice

```rust
impl Node {
    /// 从 from 到 to 位置剪出一个 Slice。
    /// 白话：就像复制粘贴选中的内容——哪怕选到段落中间也能剪出来。
    pub fn slice(&self, from: usize, to: usize) -> Result<Slice, String> {
        // 1. 边界检查
        // 2. ResolvedPos::resolve(self, from) 和 resolve(self, to)
        // 3. 若 from == to 返回 Slice::empty()
        // 4. 找两端公共祖先深度 shared = from_pos.shared_depth(to)
        // 5. 收集 from_pos..to_pos 范围内容，计算 open_start/open_end
        // 6. 返回 Slice::new(content, open_start, open_end)
    }
}
```

### Node::replace

```rust
impl Node {
    /// 用 slice 替换 from..to，返回新文档（原节点不变）。
    /// 白话：编辑器的"替换选中内容"——选中一段、粘贴进去、得到新文档。
    pub fn replace(&self, from: usize, to: usize, slice: &Slice) 
        -> Result<Node, ReplaceError> 
    {
        // 1. ResolvedPos::resolve(self, from) → from_pos
        // 2. ResolvedPos::resolve(self, to)   → to_pos
        // 3. 调用已有的 replace::replace(&from_pos, &to_pos, slice)
    }
}
```

### 测试

在 `tests_ts.rs` 新增 test-replace.ts 和 test-slice.ts 对应用例，与现有 content/diff/mark/resolve 测试并列。

核心测试场景：
- 空 slice 替换 = 删除一段内容
- 跨段落替换（open_start/open_end > 0）
- 替换后内容非法时返回 `ReplaceError`
- slice 边界与文档边界重合的边界值

---

## Phase 3 — 剩余 Node 方法 + create_and_fill 改进

### Node 方法

| 方法 | 说明 |
|------|------|
| `node_at(pos) -> Option<&Node>` | 给绝对位置，返回该位置的叶节点 |
| `text_content() -> String` | 把整棵子树的文本拼成字符串（忽略结构）|
| `nodes_between(from, to, f)` | 遍历 from..to 范围内所有节点，逐个调用 `f(node, pos, parent, index)` |
| `check() -> Result<(), String>` | 递归验证整棵树内容合法；不合法返回 Err |

### create_and_fill 递归改进

现状：只填一层——若需要 `blockquote > paragraph` 才合法，当前返回 `None`。

改进：递归调用，每层填完后，若子节点本身也有必需内容，继续往下填，直到叶节点或 `has_required_attrs()` 为止。

```
blockquote 需要 paragraph+ → create paragraph
  paragraph 需要 inline*  → 允许为空 → OK
```

---

## 文件修改清单

| 文件 | Phase | 变更 |
|------|-------|------|
| `src/model/schema.rs` | 1 | 新增 SchemaSpec / NodeSpec / MarkSpec / Schema；恢复 ptr_eq 比较 |
| `src/model/content.rs` | 1 | match_type / compatible 改用 ptr_eq |
| `src/model/mark.rs` | 1 | Mark::eq type 部分改用 ptr_eq |
| `src/model/node.rs` | 2, 3 | 新增 slice / replace / node_at / text_content / nodes_between / check |
| `src/model/replace.rs` | 2 | 无逻辑变更，仅确保 replace fn 签名对外可用 |
| `src/model/tests_ts.rs` | 2 | 新增 test-replace.ts 和 test-slice.ts 移植测试 |

---

## 成功标准

1. `cargo build` 零 warning 零 error
2. `cargo test` 全部通过，包含 test-replace 和 test-slice 用例
3. `Schema::new` 对合法 spec 不 panic，对非法 spec 返回 `Err`
4. 同一 Schema 内同名 NodeType/MarkType 通过 `Arc::ptr_eq` 返回 true
5. `Node::replace` 和 `Node::slice` 与 TS 参考实现行为一致
