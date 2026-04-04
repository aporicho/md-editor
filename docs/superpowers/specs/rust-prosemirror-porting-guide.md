# Rust ProseMirror 移植指南

本文档是所有模块重写的参考基准。每个模块开始前必须阅读。

---

## 项目背景

将 ProseMirror 编辑器生态（TypeScript）重写为 Rust，服务于 Iced + comit-text 渲染的 Markdown 编辑器。包含全部功能，含协同编辑。

**参考源码位置：** `reference/prosemirror-*/src/`

**模块实现顺序：**
1. `src/model/` — 文档数据模型（当前）
2. `src/transform/` — 编辑操作
3. `src/state/` — 编辑器状态与插件
4. `src/history/` — 撤销/重做
5. `src/commands/` — 编辑命令
6. `src/keymap/` — 快捷键
7. `src/markdown/` — Markdown 解析/序列化

---

## 核心设计决策

### 决策1：ContentMatch 出边存类型名，不存 Arc

**问题：** TS 的 ContentMatch 存 `Arc<NodeType>`，导致 Schema 构建时 Arc 引用计数 >1，无法用 `Arc::get_mut` 原地修改 NodeType。

**方案：**
```rust
// TS 直译（错误）
struct MatchEdge {
    node_type: Arc<NodeType>,   // ❌ 导致循环引用，Arc 重建
    next: Arc<ContentMatch>,
}

// Rust 重设计（正确）
pub struct ContentMatch {
    pub valid_end: bool,
    edges: Vec<(String, Arc<ContentMatch>)>,  // 存类型名，按名匹配
}

impl ContentMatch {
    pub fn match_type(&self, name: &str) -> Option<&Arc<ContentMatch>> {
        self.edges.iter().find(|(n, _)| n == name).map(|(_, next)| next)
    }
}
```

**效果：** Schema 构建时 NodeType Arc 引用计数始终为 1，`Arc::get_mut` 随时可用，无需重建 Arc，无需 unsafe。

---

### 决策2：MarkType 排斥关系用枚举，不用 Vec<Arc<MarkType>>

**问题：** TS 的 `excluded: MarkType[]` 在 Rust 中需要 `Vec<Arc<MarkType>>`，`link` 类型排斥自身导致循环引用，需要 unsafe 代码。

**方案：**
```rust
pub enum ExcludeSpec {
    SelfOnly,              // 默认：只排斥同名类型
    All,                   // "_"：排斥所有 mark
    Named(Vec<String>),    // 指定名字列表
}

pub struct MarkType {
    pub name: String,
    pub rank: usize,
    pub excluded: ExcludeSpec,
    pub inclusive: bool,
    // ...
}

impl MarkType {
    pub fn excludes(&self, other: &MarkType) -> bool {
        match &self.excluded {
            ExcludeSpec::SelfOnly => self.name == other.name,
            ExcludeSpec::All => true,
            ExcludeSpec::Named(names) => names.iter().any(|n| n == &other.name),
        }
    }
}
```

---

### 决策3：Schema 构建用真正的两遍遍历

得益于决策1和2，Schema 构建可以用干净的两遍遍历，无需重建 Arc：

```rust
impl Schema {
    pub fn new(spec: SchemaSpec) -> Result<Arc<Self>, SchemaError> {
        // 第一遍：创建所有 NodeType / MarkType Arc（refcount = 1）
        let mut node_types: IndexMap<String, Arc<NodeType>> = build_nodes(&spec);
        let mut mark_types: IndexMap<String, Arc<MarkType>> = build_marks(&spec);

        // 第二遍：Arc::get_mut 安全修改（refcount 仍为 1）
        for (name, ns) in &spec.nodes {
            let nt = Arc::get_mut(node_types.get_mut(name).unwrap()).unwrap();
            if let Some(ref expr) = ns.content {
                // ContentMatch::parse 只存 String，不 clone NodeType Arc → refcount 不增加
                nt.content_match = Some(ContentMatch::parse(expr, &node_types)?);
            }
            nt.mark_set = resolve_mark_set(&ns.marks, &mark_types)?;
        }
        // MarkType excluded 同理
        // ...
    }
}
```

---

### 决策4：NodeType 保留 content_match 字段

NodeType 直接持有 `content_match: Option<Arc<ContentMatch>>`，保持与 TS API 接近（`node_type.content_match`），不需要通过 Schema 中转。这由决策1保证安全。

```rust
pub struct NodeType {
    pub name: String,
    pub is_block: bool,
    pub is_text: bool,
    pub is_atom: bool,
    pub inline_content: bool,
    pub groups: Vec<String>,
    pub content_match: Option<Arc<ContentMatch>>,  // 安全，无循环
    pub mark_set: Option<Vec<Arc<MarkType>>>,
    pub attrs: IndexMap<String, AttrSpec>,
    pub whitespace: Whitespace,
}
```

---

## 类型对应关系

| TypeScript | Rust | 备注 |
|-----------|------|------|
| `Attrs` (`Record<string, any>`) | `Arc<HashMap<String, AttrValue>>` | Arc 共享避免克隆 |
| `Node` | `Node` (struct) | text 字段 `Option<String>` |
| `Fragment` | `Fragment` | `content: Arc<[Node]>` |
| `Mark` | `Mark` | |
| `NodeType` | `Arc<NodeType>` | Schema 管理生命周期 |
| `MarkType` | `Arc<MarkType>` | |
| `Schema` | `Arc<Schema>` | |
| `ContentMatch` | `Arc<ContentMatch>` | 出边存 String |
| `Slice` | `Slice` | |
| `ResolvedPos` | `ResolvedPos` | |

---

## 常见 TS → Rust 模式

### 链式方法（Builder 模式）
```typescript
// TS
tr.setSelection(sel).scrollIntoView()

// Rust（Transaction 可变，消耗 self 返回 self）
tr.set_selection(sel).scroll_into_view()
```

### 可选回调
```typescript
// TS
command(state, dispatch?, view?)

// Rust
fn command(state: &EditorState, dispatch: Option<&dyn Fn(Transaction)>) -> bool
```

### 抽象类
```typescript
// TS
abstract class Selection { abstract eq(other: Selection): boolean }

// Rust
pub trait Selection: Any + Send + Sync {
    fn eq(&self, other: &dyn Selection) -> bool;
    fn map(&self, doc: &Node, mapping: &dyn Mappable) -> Box<dyn Selection>;
}
```

### JSON 序列化
```typescript
// TS：toJSON() / fromJSON()

// Rust：实现 serde Serialize/Deserialize
#[derive(Serialize, Deserialize)]
pub struct Node { ... }
```

---

## 模块边界规则

- `model` 不依赖任何其他模块
- `transform` 只依赖 `model`
- `state` 依赖 `model` + `transform`
- `history`、`commands`、`keymap`、`markdown` 依赖 `state`（及其上游）
- 禁止循环依赖

---

## 测试策略

每个模块的 TS 测试文件在 `reference/prosemirror-*/test/` 中，逐一移植为 Rust 测试。测试文件命名：`src/{module}/tests_{ts_file_name}.rs`。

---

## 外部依赖

| 用途 | crate |
|------|-------|
| 有序 HashMap | `indexmap` |
| JSON 序列化 | `serde` + `serde_json` |
| Markdown 解析 | `pulldown-cmark` 或 `markdown-it` 的 Rust 实现 |
| 协同编辑 OT | 待调研 |
