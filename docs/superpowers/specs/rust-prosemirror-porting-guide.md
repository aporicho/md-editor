# Rust ProseMirror 移植指南

**每个模块开始前必须阅读本文档。**

---

## 目标

将 ProseMirror 编辑器生态（TypeScript）完整重写为 Rust，支撑基于 Iced + cosmic-text 渲染的 Markdown 编辑器。功能范围对齐 ProseMirror 全部库，包含协同编辑。

---

## 原则

### 1. Rust 惯用，而非 TS 直译

不追求与 TypeScript API 一字不差地对应。每个设计决策以"在 Rust 里最自然、最安全"为准。宁可调用方式稍有不同，也不引入 unsafe 或结构性缺陷。

### 2. 无 unsafe

整个代码库不使用 `unsafe`。如果某个设计需要 unsafe 才能实现，说明设计本身有问题，应当重新设计数据结构。

### 3. 无循环引用

类型之间不允许 `Arc` 循环引用。如果发现循环，通过存名字（`String`）或使用 `Weak` 来打破，而不是用 `unsafe` 绕过。

### 4. 不可变优先

文档数据（`Node`、`Fragment`、`Slice` 等）构建后不可变，修改通过创建新值实现，与 ProseMirror 的不可变文档模型一致。

### 5. 功能完整，测试覆盖

每个 TS 测试文件都要有对应的 Rust 测试。移植不算完成，除非对应的测试全部通过。

### 6. API 一对一还原，不多不少

ProseMirror 每个公开类的每个公开方法和属性，都必须在 Rust 中有对应实现，一个不多，一个不少。调用方式可以符合 Rust 习惯（如 `snake_case`、`Option` 替代 `null`），但功能和语义必须完全对齐。不允许以"Rust 不方便实现"为由跳过某个方法，也不允许擅自增加 ProseMirror 没有的方法。

公开 API 的参照来源：`reference/prosemirror-*/src/index.ts` 列出的所有导出类型。

### 7. 写就写完，不留技术债

一旦开始写一个文件，必须写完——所有方法、所有测试、全部通过后再提交。不允许写一半留 `TODO`、`unimplemented!()` 或空方法体后转去写别的。如果某个文件的实现依赖另一个尚未实现的文件，先把依赖文件实现完再回来。

**例外：外部依赖尚未存在时。** 当某个 API 的实现依赖一个尚未设计/实现的外部层（如 cosmic-text 渲染层），允许用 `todo!("原因说明")` 作为方法体占位，但必须满足：
1. 文件顶部有注释说明哪些方法是占位及原因
2. 公开 API 签名必须完整正确（不能因为占位而省略参数或返回类型）
3. 该文件在对应的外部层实现后必须立即补全

### 8. TS 注释全部移植

原始 TypeScript 源码中的所有注释（包括方法说明、参数说明、行为说明）必须完整移植到对应的 Rust 文件中，作为 `///` 文档注释。不允许省略、缩写或替换成中文。

### 9. 模块顺序不能跳

`model` → `transform` → `state` → `history/commands/keymap/markdown`，严格按依赖顺序，不超前实现。

---

## 工作流程

每个模块按以下流程推进，**不允许跳步**：

### 步骤 1：Brainstorming（设计决策）
用 `/brainstorming` 技能对本模块做设计决策。产出：
- 模块设计文档 `docs/superpowers/specs/YYYY-MM-DD-{module}-design.md`

### 步骤 2：阶段计划（Phase Plan）
创建模块阶段计划文档 `docs/superpowers/plans/{module}-phases.md`，将模块拆分为 3-6 个阶段，每个阶段：
- 范围：涉及哪些文件
- 依赖：需要哪些前置阶段完成
- 产出：可独立编译并通过测试的代码

格式见 `docs/superpowers/plans/model-phases.md`（示例）。

### 步骤 3：逐阶段实现
对每个阶段，用 `/writing-plans` 技能生成详细实现计划，然后用 `subagent-driven-development` 或 `executing-plans` 执行。

每个阶段的详细计划保存到：
`docs/superpowers/plans/{module}-phase{N}-plan.md`

### 原则
- 阶段计划（Phase Plan）：高层，列出阶段和范围，不写代码
- 详细实现计划（writing-plans 产出）：低层，每个步骤有完整代码
- 每个阶段完成后才能开始下一个阶段

### 阶段粒度规则

**每个阶段的工作量上限：一个文件的完整实现。**

以下情况必须拆成子阶段（Phase N.1 / N.2 / N.3）：
1. **跨文件依赖**：一个文件的某些方法依赖另一个尚未实现的文件（如 ContentMatch.fill_before 依赖 Fragment），必须把依赖方法单独拆到另一个子阶段。
2. **单文件过大**：如果一个文件包含多个独立的职责（如 schema.rs 同时包含 NodeType、MarkType、Schema::new），必须按职责拆为独立子阶段。
3. **测试依赖前置数据**：如果某个文件的测试需要其他模块（如 tests_content.ts 需要 Schema 构建文档树），不能把测试强塞进同一阶段，要等依赖就绪后再单独安排测试子阶段。

**判断方法：** 写阶段计划时，把每个文件的方法列出来，逐一检查：
- 方法的参数类型或返回类型是否引用了尚未实现的类型？→ 是 → 拆子阶段或加 stub
- 测试能否只用当前阶段已有的类型构造输入？→ 否 → 拆子阶段

**stub 规则：**
- stub 方法名加 `_stub` 后缀，防止意外调用
- 文件顶部注释列出所有 stub 方法及原因
- stub 对应的子阶段必须在阶段计划中明确登记

---

## 项目背景

将 ProseMirror 编辑器生态（TypeScript）重写为 Rust，服务于 Iced + cosmic-text 渲染的 Markdown 编辑器。包含全部功能，含协同编辑。

**参考源码位置：** `reference/prosemirror-*/src/`

**三个阶段目标：**

- **第一阶段（最小可用编辑器）：** model + transform + state + view，完成后编辑器可运行
- **第二阶段（完整功能）：** history + commands + keymap + markdown，完成后具备完整编辑体验
- **第三阶段（协同编辑）：** collab，完成后支持多人实时协作

**模块实现顺序：**
1. `src/model/` — 文档数据模型（当前）
2. `src/transform/` — 编辑操作
3. `src/state/` — 编辑器状态与插件
4. `src/view/` — 渲染与用户交互（基于 Iced + cosmic-text，替代 prosemirror-view）
5. `src/history/` — 撤销/重做
6. `src/commands/` — 编辑命令
7. `src/keymap/` — 快捷键
8. `src/markdown/` — Markdown 解析/序列化
9. `src/collab/` — 协同编辑（基于 prosemirror-collab）

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
| GUI 框架 | `iced` |
| 文字渲染 | `cosmic-text` |
| 有序 HashMap | `indexmap` |
| JSON 序列化 | `serde` + `serde_json` |
| Markdown 解析 | `pulldown-cmark` 或 `markdown-it` 的 Rust 实现 |
| 协同编辑 OT | 待调研 |
