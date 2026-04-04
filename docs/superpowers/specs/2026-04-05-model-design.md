# model 模块设计文档

**日期：** 2026-04-05  
**状态：** 已确认，待实现  
**参考：** `reference/prosemirror-model/src/`、`docs/superpowers/specs/rust-prosemirror-porting-guide.md`

---

## 概述

将 `prosemirror-model` TypeScript 库以 Rust 重写，作为编辑器数据模型的基础层。重写目标是功能和语义与原始库完全对齐，但 API 设计以"在 Rust 里最自然、最安全"为准，而非逐字对应 TS 代码。model 不依赖任何其他模块，所有上层模块（transform、state 等）都依赖它。

---

## 设计决策

### 决策 1：DOM 相关 API 改为渲染层 stub

TS 的 `DOMParser`、`DOMSerializer`、`DOMOutputSpec`、`ParseRule` 等类型是浏览器专用 API，本项目使用 Iced + comit-text 原生渲染，不经过 HTML DOM。

**处理方式：**
- `dom_parser.rs` / `dom_serializer.rs`：创建文件，公开 API 签名完整，方法体用 `todo!("需要 comit-text 集成后实现")`
- `NodeSpec.to_render` / `MarkSpec.to_render`：字段保留，使用 `RenderOutput` 占位类型
- `test-dom.ts`：跳过，不创建对应测试文件
- 依据移植指南原则 7 例外条款（外部依赖尚未存在）

### 决策 2：AttrValue 使用 serde_json::Value

TS 的 `Attrs` 类型为 `{[attr: string]: any}`。Rust 中：

```rust
pub type Attrs = Arc<HashMap<String, serde_json::Value>>;
```

使用 `serde_json::Value` 的理由：ProseMirror 的 attrs 本身就是 JSON 可序列化的，`toJSON()`/`fromJSON()` 是核心 API，无需额外转换。

### 决策 3：Node 使用单一 struct，不用枚举

TS 中 `TextNode extends Node`，Rust 中统一为一个 struct：

```rust
pub struct Node {
    pub text: Option<String>,  // Some = 文本节点，None = 元素节点
    // ...
}
```

枚举方案（`enum Node { Text(TextNode), Element(ElementNode) }`）虽然更类型安全，但 `node_type`、`attrs`、`marks` 三个字段重复，几十个共用方法需要重复 match，代价大于收益。

### 决策 4：文件结构镜像 TS 源码

一个 TS 源文件对应一个 Rust 文件，方便移植时对照查阅。（见下方文件结构）

### 决策 5：去掉 NodeType/MarkType 的 schema 反向引用

TS 中 `NodeType.schema` 和 `MarkType.schema` 反向持有 Schema，Rust 中移除，避免循环引用。需要 schema 的方法改为接收 `&Schema` 参数。

### 决策 6：ContentMatch 出边存类型名

见移植指南决策 1。

### 决策 7：ExcludeSpec 枚举替代 Vec<Arc<MarkType>>

见移植指南决策 2。

### 决策 8：Schema 两遍构建

见移植指南决策 3。

---

## 文件结构

```
src/model/
  mod.rs              ← 公开导出所有类型
  compare_deep.rs     ← compare_deep 工具函数
  diff.rs             ← find_diff_start / find_diff_end
  content.rs          ← ContentMatch（DFA 状态机）
  schema.rs           ← Schema, NodeType, MarkType, NodeSpec, MarkSpec, AttrSpec, SchemaSpec
  mark.rs             ← Mark
  fragment.rs         ← Fragment
  node.rs             ← Node
  resolved_pos.rs     ← ResolvedPos, NodeRange
  replace.rs          ← Slice, ReplaceError, replace 算法
  render_output.rs    ← RenderOutput 占位类型
  dom_parser.rs       ← DOMParser stub（todo!）
  dom_serializer.rs   ← DOMSerializer stub（todo!）
  errors.rs           ← SchemaError, SyntaxError, RangeError
```

---

## 核心类型定义

### Node

```rust
pub struct Node {
    pub node_type: Arc<NodeType>,
    pub attrs: Attrs,                          // Arc<HashMap<String, serde_json::Value>>
    pub content: Fragment,
    pub marks: Vec<Mark>,
    pub text: Option<String>,                  // Some = 文本节点
}
```

主要方法：`resolve`、`slice`、`replace`、`node_at`、`child_after`、`child_before`、`nodes_between`、`descendants`、`text_between`、`can_replace`、`can_replace_with`、`can_append`、`check`、`copy`、`mark`、`cut`、`eq`、`same_markup`、`has_markup`、`to_json`、`from_json`，以及所有 `is_*` 属性方法。

### Fragment

```rust
pub struct Fragment {
    content: Arc<[Node]>,
    pub size: usize,
}
```

主要方法：`from`、`from_array`、`from_json`、`append`、`cut`、`replace_child`、`add_to_start`、`add_to_end`、`eq`、`child`、`maybe_child`、`for_each`、`nodes_between`、`text_between`、`find_diff_start`、`find_diff_end`、`to_json`，以及 `first_child`、`last_child`、`child_count` 属性。

### Mark

```rust
pub struct Mark {
    pub mark_type: Arc<MarkType>,
    pub attrs: Attrs,
}
```

主要方法：`add_to_set`、`remove_from_set`、`is_in_set`、`eq`、`to_json`、`from_json`、`same_set`、`set_from`，以及常量 `NONE`。

### Slice

```rust
pub struct Slice {
    pub content: Fragment,
    pub open_start: usize,
    pub open_end: usize,
}
```

主要方法：`size`、`eq`、`to_json`、`from_json`、`max_open`，以及常量 `EMPTY`。

### ResolvedPos / NodeRange

```rust
pub struct ResolvedPos {
    pub pos: usize,
    // 内部路径：每三个元素一组 [node, index, start_pos]，对应 TS 的 path: any[]
    path: Vec<PathEntry>,
    pub parent_offset: usize,
}

// 路径中每一层的信息
enum PathEntry {
    NodeRef(Arc<Node>),
    Index(usize),
    StartPos(usize),
}

pub struct NodeRange {
    pub from: ResolvedPos,   // 对应 TS 的 $from
    pub to: ResolvedPos,     // 对应 TS 的 $to
    pub depth: usize,
}
```

ResolvedPos 主要方法：`resolve`、`depth`、`parent`、`doc`、`node`、`index`、`index_after`、`start`、`end`、`before`、`after`、`text_offset`、`node_after`、`node_before`、`pos_at_index`、`marks`、`marks_across`、`shared_depth`、`block_range`、`same_parent`、`max`、`min`。

### ContentMatch

```rust
pub struct ContentMatch {
    pub valid_end: bool,
    edges: Vec<(String, Arc<ContentMatch>)>,
}
```

主要方法：`parse`、`match_type`、`match_fragment`、`inline_content`、`default_type`、`fill_before`、`find_wrapping`、`edge_count`、`edge`，以及常量 `EMPTY`。

### NodeType

```rust
pub struct NodeType {
    pub name: String,
    pub is_block: bool,
    pub is_text: bool,
    pub is_inline: bool,
    pub is_leaf: bool,
    pub is_atom: bool,
    pub inline_content: bool,
    pub groups: Vec<String>,
    pub content_match: Arc<ContentMatch>,
    pub mark_set: Option<Vec<Arc<MarkType>>>,
    pub attrs: IndexMap<String, AttrSpec>,
    pub whitespace: Whitespace,
    pub spec: NodeSpec,
}
```

主要方法：`create`、`create_checked`、`create_and_fill`、`valid_content`、`allows_mark_type`、`allows_marks`、`allowed_marks`、`compatible_content`、`has_required_attrs`、`is_in_group`、`is_textblock`。

### MarkType

```rust
pub struct MarkType {
    pub name: String,
    pub rank: usize,
    pub attrs: IndexMap<String, AttrSpec>,
    pub inclusive: bool,
    pub excluded: ExcludeSpec,
    pub spec: MarkSpec,
}

pub enum ExcludeSpec {
    SelfOnly,
    All,
    Named(Vec<String>),
}
```

主要方法：`create`、`excludes`、`remove_from_set`、`is_in_set`。

### Schema

```rust
pub struct Schema {
    pub nodes: IndexMap<String, Arc<NodeType>>,
    pub marks: IndexMap<String, Arc<MarkType>>,
    pub top_node_type: Arc<NodeType>,
    pub linebreak_replacement: Option<Arc<NodeType>>,
    pub spec: SchemaSpec,
}
```

主要方法：`new`、`node`、`text`、`mark`、`node_from_json`、`mark_from_json`。

---

## Spec 类型

```rust
pub struct NodeSpec {
    pub content: Option<String>,
    pub marks: Option<String>,
    pub group: Option<String>,
    pub inline: bool,
    pub atom: bool,
    pub attrs: IndexMap<String, AttrSpec>,
    pub selectable: bool,
    pub draggable: bool,
    pub code: bool,
    pub whitespace: Option<Whitespace>,
    pub defining_as_context: bool,
    pub defining_for_content: bool,
    pub defining: bool,
    pub isolating: bool,
    pub linebreak_replacement: bool,
    pub to_render: Option<Box<dyn Fn(&Node) -> RenderOutput + Send + Sync>>,
    pub leaf_text: Option<Box<dyn Fn(&Node) -> String + Send + Sync>>,
    pub to_debug_string: Option<Box<dyn Fn(&Node) -> String + Send + Sync>>,
}

pub struct MarkSpec {
    pub attrs: IndexMap<String, AttrSpec>,
    pub inclusive: bool,
    pub excludes: Option<String>,
    pub group: Option<String>,
    pub spanning: bool,
    pub code: bool,
    pub to_render: Option<Box<dyn Fn(&Mark, bool) -> RenderOutput + Send + Sync>>,
}

pub struct AttrSpec {
    pub default: Option<serde_json::Value>,
    pub validate: Option<AttrValidator>,
}

pub enum AttrValidator {
    Types(Vec<String>),
    Func(Box<dyn Fn(&serde_json::Value) -> Result<(), String> + Send + Sync>),
}

pub struct SchemaSpec {
    pub nodes: IndexMap<String, NodeSpec>,
    pub marks: IndexMap<String, MarkSpec>,
    pub top_node: Option<String>,
}

pub enum Whitespace { Normal, Pre }
```

---

## 错误类型

```rust
pub struct SchemaError(pub String);    // Schema 构建、节点校验失败
pub struct ReplaceError(pub String);   // Node::replace 操作失败
pub struct SyntaxError(pub String);    // ContentMatch::parse 解析失败
pub struct RangeError(pub String);     // 位置越界等范围错误
```

全部实现 `std::error::Error` + `fmt::Display`。

---

## 渲染层 Stub

```rust
// render_output.rs
/// 渲染输出占位类型，待 comit-text 接口确定后替换。
pub struct RenderOutput(pub serde_json::Value);

// dom_parser.rs
// 注意：以下所有方法均为占位实现，待 comit-text 渲染层完成后实现。
pub struct ParseOptions {
    pub preserve_whitespace: Option<bool>,
    pub find_positions: Option<Vec<usize>>,
    pub from: Option<usize>,
    pub to: Option<usize>,
    pub top_node: Option<Node>,
    pub top_match: Option<Arc<ContentMatch>>,
    pub context: Option<ResolvedPos>,
}

pub struct DOMParser { schema: Arc<Schema> }
impl DOMParser {
    pub fn from_schema(schema: &Arc<Schema>) -> Self { todo!("需要 comit-text 集成后实现") }
    pub fn parse(&self, input: &RenderOutput, options: Option<ParseOptions>) -> Node { todo!("需要 comit-text 集成后实现") }
    pub fn parse_slice(&self, input: &RenderOutput, options: Option<ParseOptions>) -> Slice { todo!("需要 comit-text 集成后实现") }
}

// dom_serializer.rs
// 注意：以下所有方法均为占位实现，待 comit-text 渲染层完成后实现。
pub struct DOMSerializer {
    pub nodes: HashMap<String, Box<dyn Fn(&Node) -> RenderOutput + Send + Sync>>,
    pub marks: HashMap<String, Box<dyn Fn(&Mark, bool) -> RenderOutput + Send + Sync>>,
}
impl DOMSerializer {
    pub fn from_schema(schema: &Arc<Schema>) -> Self { todo!("需要 comit-text 集成后实现") }
    pub fn serialize_node(&self, node: &Node) -> RenderOutput { todo!("需要 comit-text 集成后实现") }
    pub fn serialize_fragment(&self, fragment: &Fragment) -> RenderOutput { todo!("需要 comit-text 集成后实现") }
}
```

---

## mod.rs 公开导出

```rust
pub use node::Node;
pub use fragment::Fragment;
pub use mark::Mark;
pub use replace::{Slice, ReplaceError, replace};
pub use resolved_pos::{ResolvedPos, NodeRange};
pub use schema::{Schema, NodeType, MarkType, NodeSpec, MarkSpec, AttrSpec, SchemaSpec, Attrs, Whitespace, ExcludeSpec, AttrValidator};
pub use content::ContentMatch;
pub use render_output::RenderOutput;
pub use dom_parser::{DOMParser, ParseOptions};
pub use dom_serializer::DOMSerializer;
pub use errors::{SchemaError, SyntaxError, RangeError};
```

---

## 测试文件对应关系

| TS 测试文件 | Rust 测试文件 |
|---|---|
| `test-content.ts` | `src/model/tests_content.rs` |
| `test-diff.ts` | `src/model/tests_diff.rs` |
| `test-mark.ts` | `src/model/tests_mark.rs` |
| `test-node.ts` | `src/model/tests_node.rs` |
| `test-replace.ts` | `src/model/tests_replace.rs` |
| `test-resolve.ts` | `src/model/tests_resolve.rs` |
| `test-slice.ts` | `src/model/tests_slice.rs` |
| `test-dom.ts` | 跳过（无 DOM 层） |

---

## 外部依赖

```toml
[dependencies]
indexmap = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```
