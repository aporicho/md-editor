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

TS 的 `DOMParser`、`DOMSerializer`、`DOMOutputSpec`、`ParseRule` 等类型是浏览器专用 API，本项目使用 Iced + cosmic-text 原生渲染，不经过 HTML DOM。

**处理方式：**
- `dom_parser.rs` / `dom_serializer.rs`：创建文件，公开 API 签名完整，方法体用 `todo!("需要 cosmic-text 集成后实现")`
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
/// 渲染输出占位类型，待 cosmic-text 接口确定后替换。
pub struct RenderOutput(pub serde_json::Value);

// dom_parser.rs
// 注意：以下所有方法均为占位实现，待 cosmic-text 渲染层完成后实现。
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
    pub fn from_schema(schema: &Arc<Schema>) -> Self { todo!("需要 cosmic-text 集成后实现") }
    pub fn parse(&self, input: &RenderOutput, options: Option<ParseOptions>) -> Node { todo!("需要 cosmic-text 集成后实现") }
    pub fn parse_slice(&self, input: &RenderOutput, options: Option<ParseOptions>) -> Slice { todo!("需要 cosmic-text 集成后实现") }
}

// dom_serializer.rs
// 注意：以下所有方法均为占位实现，待 cosmic-text 渲染层完成后实现。
pub struct DOMSerializer {
    pub nodes: HashMap<String, Box<dyn Fn(&Node) -> RenderOutput + Send + Sync>>,
    pub marks: HashMap<String, Box<dyn Fn(&Mark, bool) -> RenderOutput + Send + Sync>>,
}
impl DOMSerializer {
    pub fn from_schema(schema: &Arc<Schema>) -> Self { todo!("需要 cosmic-text 集成后实现") }
    pub fn serialize_node(&self, node: &Node) -> RenderOutput { todo!("需要 cosmic-text 集成后实现") }
    pub fn serialize_fragment(&self, fragment: &Fragment) -> RenderOutput { todo!("需要 cosmic-text 集成后实现") }
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

## Phase 3/4 设计决策（Fragment / Node / ResolvedPos）

### 决策 9：ResolvedPos::path 用 struct 替代混合数组

TS 的 `path: any[]` 将节点、序号、起始位置混合存入一个数组，靠 `path[depth * 3]`、`path[depth * 3 + 1]` 等手算偏移访问。这是 JS 为避免对象分配的性能 hack。

Rust 里用有名字的 struct，栈上分配，零额外开销：

```rust
struct PathStep {
    node: Arc<Node>,
    index: usize,
    start: usize,
}
// ResolvedPos.path: Vec<PathStep>
```

### 决策 10：Fragment 实现 Iterator，不用 forEach 回调

TS 的 `forEach(f: (node, offset, index) => void)` 是回调风格。Rust 里实现 `iter()` 返回迭代器，调用方可以直接用 `map`、`filter`、`fold` 等标准迭代器方法，不需要学新 API。

### 决策 11：Fragment::from() 用 From trait 替代运行时类型判断

TS 的 `Fragment.from()` 在运行时用 `instanceof` 判断输入类型。Rust 里实现：

```rust
impl From<Node> for Fragment { ... }
impl From<Vec<Node>> for Fragment { ... }
```

编译期确定类型，比运行时判断更安全。

### 决策 12：findIndex 返回元组，不复用全局对象

TS 里 `findIndex` 复用一个全局 `{index, offset}` 对象来避免堆分配。Rust 里直接返回 `(usize, usize)`，放在栈上，零开销，不需要这个 hack。

### 决策 13：resolve_cached 有意不实现

TS 用 `WeakMap<Node, ResolveCache>` 缓存 resolve 结果，依赖 GC 自动清理。Rust 没有对应机制，备选方案均存在较大代价：

- **Node 内嵌缓存**：每个 Node 都带缓存，文档树有大量叶子节点永远不会被 resolve，内存浪费。
- **引入独立 Doc 类型**：改动面覆盖全部 7 个模块（model/transform/state/history/commands/keymap/markdown），为性能优化引入大范围架构改动不合算。

**决定：** Phase 4 只实现无缓存的 `resolve()`，等性能测试发现实际瓶颈后再针对性设计缓存方案。Rust 的编译优化比 JS 强，无缓存的实际性能损耗预计远小于 TS。

---

## Phase 2.2/2.3 设计决策（NodeType / MarkType / Schema）

### 决策 14：删除 `Schema.cached`

TS 的 `cached: {[key: string]: any}` 是通用扩展槽，实际只用于存 `wrappings` 缓存。该缓存已移入 `ContentMatch.wrap_cache`（决策2），Schema 不再需要此字段。Rust 里 `HashMap<String, Box<dyn Any + Send + Sync>>` 既丑又难用，直接删除。

### 决策 15：保留 `MarkType.instance` 优化

TS 里当 mark 所有属性都有默认值时，提前缓存一个默认 Mark 实例，`create()` 时直接返回，不新建对象。`em`、`strong` 等无属性 mark 在文档里出现频率极高，此优化有实际价值。

Rust 里对应：

```rust
pub struct MarkType {
    instance: Option<Arc<Mark>>,  // 所有属性有默认值时预建
    // ...
}
```

### 决策 16：`is_leaf` 检查边数，不用 ptr_eq

TS 靠 `contentMatch == ContentMatch.empty` 对象引用相等判断叶子节点。Rust 里用 `Arc::ptr_eq` 可以模拟，但依赖单例比较，语义不直接。改为：

```rust
pub fn is_leaf(&self) -> bool {
    self.content_match.edge_count() == 0 && self.content_match.valid_end
}
```

### 决策 17：不存 `nodeFromJSON`/`markFromJSON` 闭包

TS 将这两个方法存为绑定闭包是因为 JS 方法调用需要绑定 `this`。Rust 方法自动持有 `&self`，直接写成普通方法，无需存闭包。

---

## Phase 3 设计决策（Mark / Diff / Replace）

### 决策 18：mark set 用 `Arc<[Mark]>` 而不是 `Vec<Mark>`

文档树里大量节点共享相同的 mark set（尤其是空 mark set）。`Arc<[Mark]>` 允许零拷贝共享，`Vec<Mark>` 每次都要克隆。与 TS 的 `readonly Mark[]` 语义一致，但内存效率更高。

### 决策 19：`Mark::none` 为全局单例

TS 的 `static none: readonly Mark[] = []` 是运行时静态字段。Rust 里用 `OnceLock<Arc<[Mark]>>` 实现全局单例，所有无 mark 节点共享同一个 Arc。该值为只读数据，无副作用，线程安全。

### 决策 20：Mark 实现 `PartialEq` + `Hash`

TS 用 `mark.eq(other)` 自定义方法比较。Rust 里实现 `PartialEq` 支持 `==` 运算符，同时必须实现 `Hash`（Rust 语义约定：实现 `PartialEq` 的类型若可能用作集合键，必须同时实现 `Hash`）。两者必须一起加。

### 决策 21：replace 全链路返回 `Result`

TS 里 `replace()` 及内部辅助函数通过 `throw new ReplaceError(...)` 传递错误，调用方无法在编译期感知。Rust 里整条调用链返回 `Result<Node, ReplaceError>`，调用方在编译期必须处理错误，不可能漏掉。这不是可选优化，是 Rust 的正确做法。

### 决策 22：`findDiffEnd` 返回具名 struct

TS 返回 `{a: number, b: number} | null`。Rust 里用具名 struct 替代匿名对象：

```rust
pub struct DiffEnd {
    pub a: usize,
    pub b: usize,
}
// 返回 Option<DiffEnd>
```

语义比元组 `(usize, usize)` 更清晰，调用方不需要记住位置含义。

---

## Phase 3 基础设计决策（影响所有文件）

### 决策 26：Fragment 存 `Vec<Arc<Node>>`，不用 `Arc<[Node]>`

两个方向：

- **`Arc<[Node]>`**：节点紧凑存储，一次分配。但 PathStep 只能持有 `&Node`，需要生命周期参数 `'a`，ResolvedPos、transform、state 所有模块全部感染，代价极大。
- **`Vec<Arc<Node>>`**：每个节点独立包 Arc，PathStep 直接存 `Arc<Node>`，ResolvedPos 无需生命周期，API 简洁。

**决定：** 使用 `Vec<Arc<Node>>`。多一次堆分配换来整个系统无生命周期负担，值得。

Fragment 定义更新为：

```rust
pub struct Fragment {
    content: Vec<Arc<Node>>,
    pub size: usize,
}
```

同时 PathStep 定义：

```rust
struct PathStep {
    node: Arc<Node>,
    index: usize,
    start: usize,
}
```

### 决策 27：保持 4 个独立错误类型，不统一成枚举

当前有 `SchemaError`、`ReplaceError`、`SyntaxError`、`RangeError` 四个独立 struct。

统一成单一枚举的好处是 `?` 可以统一传播，但代价是调用方每次 match 都要处理不可能出现的变体（比如调用 `replace` 的地方不可能遇到 `SchemaError`）。

**决定：** 保持现状，4 个独立 struct 各自对应自己的使用场景，调用方只处理真正可能出现的错误类型。

---

## 跨模块通用设计决策

### 决策 23：JSON 序列化手写方法，不用 serde DeserializeSeed

所有类型的 `fromJSON` 需要 `Schema` 参数（根据名字查找 NodeType/MarkType）。serde 的 `Deserialize` trait 不支持传额外参数，绕过限制需要用 `DeserializeSeed`，调用方写法复杂且无法融入 serde 生态（axum、sqlx 等期待标准 `Deserialize`）。

**决定：** 全部手写：

```rust
impl Node {
    pub fn to_json(&self) -> Value { ... }
    pub fn from_json(schema: &Schema, json: &Value) -> Result<Node, RangeError> { ... }
}
```

调用方写法和 TS 几乎一致，简单直接。`toJSON` 方向（序列化）可酌情用 serde，因为不需要传额外参数。

### 决策 24：所有类型实现 `Display` trait 替代 `toString()`

TS 里 `Fragment`、`Node`、`Slice`、`ResolvedPos` 等类型都有 `toString()` 用于调试输出。Rust 里实现 `std::fmt::Display`，可直接用 `format!("{}", node)`、`println!`、`dbg!`，与所有 Rust 内置类型用法一致，零额外代价。

### 决策 25：`Schema::new` 内用局部 HashMap 缓存 ContentMatch

TS 构建 Schema 时用局部 `contentExprCache` 避免相同内容表达式重复编译 DFA。Rust 里对应：

```rust
let mut expr_cache: HashMap<String, Arc<ContentMatch>> = HashMap::new();
// 构建完自动释放，不影响外部
```

大型 schema 里多个节点共享同一内容规则（如 `"inline*"`）时，避免重复编译，是正确实现的一部分，不是可选优化。

---

## Phase 3.4 设计决策

### 决策 28：fill_before / find_wrapping 增加 `schema: &Schema` 参数

TS 的 `ContentMatch.fillBefore(after, toEnd)` 和 `findWrapping(target)` 在运行时直接持有 `NodeType` 对象，可以调用 `type.isText`、`type.hasRequiredAttrs()`、`type.createAndFill()` 等。

Rust 的 ContentMatch::edges 存储 `(String, Arc<ContentMatch>)`（Decision 6），不持有 NodeType 引用，避免循环强引用（Schema→NodeType→ContentMatch→NodeType）。

**处理方式：** `fill_before` 和 `find_wrapping` 签名增加 `schema: &Schema` 参数，在调用时按类型名查询 NodeType：

```rust
pub fn fill_before(
    &self,
    schema: &Schema,
    after: &Fragment,
    to_end: bool,
    start_index: usize,
) -> Option<Fragment>

pub fn find_wrapping(
    &self,
    schema: &Schema,
    target: &str,
) -> Option<Vec<String>>
```

API 从 TS 略有不同，但与 Decision 6（出边存类型名，按需查 Schema）保持一致。`match_fragment` 同样增加 `schema: &Schema` 参数（需要通过 node_type 名查 NodeType 做属性判断）。

### 决策 29：`Fragment::empty()` 用 OnceLock 懒加载全局单例

TS 的 `Fragment.empty` 是静态字段，所有叶节点共享同一空 Fragment 实例。Rust 里 `Fragment::empty()` 用 `OnceLock<Fragment>` 实现全局单例，调用时 clone（只增 Arc refcount，廉价）。与 Mark::NONE（`const &'static [Self]`）语义一致。

---

## 外部依赖

```toml
[dependencies]
indexmap = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```
