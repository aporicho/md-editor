# Rust ProseMirror 架构参考

实现单个模块时的快速参考。包含全局 API、模块依赖关系、数据流。

> **注意：本文档仅供快速定位和整体理解，内容可能不完整或过时。实现时必须以 `reference/prosemirror-*/src/` 中的 TypeScript 源码为准，所有细节、边界情况、参数类型请直接查阅原始代码。**

> **工作流程：** 每个模块先做 Brainstorming（设计决策）→ 创建阶段计划（`docs/superpowers/plans/{module}-phases.md`）→ 逐阶段用 writing-plans 生成详细计划并执行。详见移植指南的"工作流程"章节。

---

## 模块依赖图

```
markdown ──┐
commands ──┤
 history ──┼──→ state ──→ transform ──→ model
  keymap ──┘
```

- `model`：无依赖，基础层
- `transform`：只依赖 `model`
- `state`：依赖 `model` + `transform`
- `history` / `commands` / `keymap` / `markdown`：依赖 `state`（及其上游）

---

## 一次编辑的完整数据流

```
用户按键
  → keymap 匹配快捷键
  → commands 执行命令(state, dispatch)
  → dispatch(tr: Transaction)       ← Transaction 继承 Transform
  → state.apply(tr) → EditorState   ← 不可变，返回新 state
  → plugins.apply(tr)               ← 每个 Plugin.state.apply 更新自身状态
  → history 记录 Step[]
  → Iced 重新渲染 (comit-text)
```

---

## model 模块

**文件：** `reference/prosemirror-model/src/`

### 对外暴露的核心类型

```rust
// 文档节点（不可变）
pub struct Node {
    pub node_type: Arc<NodeType>,
    pub attrs: Arc<Attrs>,
    pub content: Fragment,
    pub marks: Vec<Mark>,
    pub text: Option<String>,  // 仅文本节点
}

// 节点数组（不可变）
pub struct Fragment {
    content: Arc<[Node]>,
    pub size: usize,
}

// 行内样式
pub struct Mark {
    pub mark_type: Arc<MarkType>,
    pub attrs: Arc<Attrs>,
}

// 类型系统
pub struct NodeType {
    pub name: String,
    pub is_block: bool,
    pub is_text: bool,
    pub is_atom: bool,
    pub inline_content: bool,
    pub groups: Vec<String>,
    pub content_match: Option<Arc<ContentMatch>>,
    pub mark_set: Option<Vec<Arc<MarkType>>>,
    pub attrs: IndexMap<String, AttrSpec>,
    pub whitespace: Whitespace,
}

pub struct MarkType {
    pub name: String,
    pub rank: usize,
    pub excluded: ExcludeSpec,   // 见移植指南决策2
    pub inclusive: bool,
    pub attrs: IndexMap<String, AttrSpec>,
}

// 内容 DFA（出边存类型名，见移植指南决策1）
pub struct ContentMatch {
    pub valid_end: bool,
    edges: Vec<(String, Arc<ContentMatch>)>,
}

// 中央类型注册表
pub struct Schema {
    pub nodes: IndexMap<String, Arc<NodeType>>,
    pub marks: IndexMap<String, Arc<MarkType>>,
    pub top_node_type: Arc<NodeType>,
}

// 编辑切片
pub struct Slice {
    pub content: Fragment,
    pub open_start: usize,
    pub open_end: usize,
}

// 解析后的文档位置
pub struct ResolvedPos { ... }
pub struct NodeRange { ... }
```

### model 提供给上层模块的关键方法

```rust
// Node
node.resolve(pos) -> ResolvedPos
node.slice(from, to) -> Slice
node.replace(from, to, slice) -> Result<Node, ReplaceError>
node.node_at(pos) -> Option<&Node>
node.content_match_at(index) -> Option<ContentMatch>
node.can_replace(from, to, replacement, start, end) -> bool

// Fragment
Fragment::from(nodes) -> Fragment
frag.append(other) -> Fragment
frag.cut(from, to) -> Fragment
frag.find_diff_start(other) -> Option<usize>
frag.find_diff_end(other) -> Option<(usize, usize)>

// Schema
Schema::new(spec) -> Result<Arc<Schema>, SchemaError>
schema.node(type, attrs, content, marks) -> Node
schema.text(text, marks) -> Node
schema.mark(type, attrs) -> Mark
```

---

## transform 模块

**文件：** `reference/prosemirror-transform/src/`

### 核心类型

```rust
// 文档变换（步骤序列）
pub struct Transform {
    pub doc: Node,
    pub steps: Vec<Box<dyn Step>>,
    pub docs: Vec<Node>,     // 每步之前的文档快照
    pub mapping: Mapping,
}

// 单个原子操作（trait）
pub trait Step: Any + Send + Sync {
    fn apply(&self, doc: &Node) -> StepResult;
    fn get_map(&self) -> StepMap;
    fn invert(&self, doc: &Node) -> Box<dyn Step>;
    fn map(&self, mapping: &dyn Mappable) -> Option<Box<dyn Step>>;
    fn merge(&self, other: &dyn Step) -> Option<Box<dyn Step>>;
    fn to_json(&self) -> serde_json::Value;
}

// 位置映射
pub struct StepMap {
    ranges: Vec<usize>,   // [oldStart, oldSize, newSize, ...]
}
pub struct Mapping {
    pub maps: Vec<StepMap>,
    pub from: usize,
    pub to: usize,
}
pub struct MapResult {
    pub pos: usize,
    pub deleted: bool,
}

// Step 的内置实现
pub struct ReplaceStep { ... }
pub struct ReplaceAroundStep { ... }
pub struct AddMarkStep { ... }
pub struct RemoveMarkStep { ... }
pub struct AddNodeMarkStep { ... }
pub struct RemoveNodeMarkStep { ... }
pub struct AttrStep { ... }
pub struct DocAttrStep { ... }
```

### transform 提供给上层模块的关键方法

```rust
// Transform（state::Transaction 继承此结构）
tr.replace(from, to, slice) -> &mut Self
tr.insert(pos, content) -> &mut Self
tr.delete(from, to) -> &mut Self
tr.add_mark(from, to, mark) -> &mut Self
tr.remove_mark(from, to, mark) -> &mut Self
tr.split(pos, depth, types_after) -> &mut Self
tr.join(pos, depth) -> &mut Self
tr.lift(range, target) -> &mut Self
tr.wrap(range, wrappers) -> &mut Self
tr.set_block_type(from, to, type, attrs) -> &mut Self
tr.set_node_attribute(pos, attr, value) -> &mut Self
tr.step(step) -> Result<&mut Self, String>

// Mappable trait（Mapping 和 StepMap 都实现）
mapping.map(pos, assoc) -> usize
mapping.map_result(pos, assoc) -> MapResult
```

---

## state 模块

**文件：** `reference/prosemirror-state/src/`

### 核心类型

```rust
// 编辑器完整状态（不可变）
pub struct EditorState {
    pub doc: Node,
    pub selection: Box<dyn Selection>,
    pub stored_marks: Option<Vec<Mark>>,
    // 内部：插件状态列表
}

// 事务（继承 Transform，可变）
pub struct Transaction {
    // 包含 Transform 的所有字段和方法
    pub time: u64,
    pub stored_marks: Option<Vec<Mark>>,
    // 内部：selection、meta、scrolled_into_view
}

// 选择（trait）
pub trait Selection: Any + Send + Sync {
    fn anchor(&self) -> usize;
    fn head(&self) -> usize;
    fn from(&self) -> usize;
    fn to(&self) -> usize;
    fn empty(&self) -> bool;
    fn eq(&self, other: &dyn Selection) -> bool;
    fn map(&self, doc: &Node, mapping: &dyn Mappable) -> Box<dyn Selection>;
    fn content(&self) -> Slice;
    fn replace(&self, tr: &mut Transaction, content: Slice);
    fn get_bookmark(&self) -> Box<dyn SelectionBookmark>;
    fn to_json(&self) -> serde_json::Value;
}

// Selection 的内置实现
pub struct TextSelection { ... }
pub struct NodeSelection { ... }
pub struct AllSelection { ... }

// 插件
pub struct Plugin {
    pub spec: PluginSpec,
}
pub struct PluginKey { ... }

pub struct PluginSpec {
    pub state: Option<Box<dyn StateFieldDyn>>,
    pub filter_transaction: Option<Box<dyn Fn(&Transaction, &EditorState) -> bool>>,
    pub append_transaction: Option<Box<dyn Fn(&[Transaction], &EditorState, &EditorState) -> Option<Transaction>>>,
}
```

### state 提供给上层模块的关键方法

```rust
// EditorState
EditorState::create(config) -> EditorState
state.apply(tr: Transaction) -> EditorState
state.tr() -> Transaction       // 创建新事务

// Transaction（继承 Transform 的所有方法，额外：）
tr.set_selection(selection) -> &mut Self
tr.set_stored_marks(marks) -> &mut Self
tr.add_stored_mark(mark) -> &mut Self
tr.remove_stored_mark(mark) -> &mut Self
tr.replace_selection(slice) -> &mut Self
tr.replace_selection_with(node) -> &mut Self
tr.delete_selection() -> &mut Self
tr.insert_text(text, from, to) -> &mut Self
tr.set_meta(key, value) -> &mut Self
tr.get_meta(key) -> Option<Value>
tr.scroll_into_view() -> &mut Self

// Plugin
plugin.get_state(state) -> Option<&dyn Any>
```

---

## history 模块

**文件：** `reference/prosemirror-history/src/history.ts`

### 对外暴露

```rust
pub fn history(config: HistoryConfig) -> Plugin   // 创建历史插件
pub fn undo(state: &EditorState, dispatch: Option<DispatchFn>) -> bool
pub fn redo(state: &EditorState, dispatch: Option<DispatchFn>) -> bool
pub fn undo_no_scroll(state: &EditorState, dispatch: Option<DispatchFn>) -> bool
pub fn redo_no_scroll(state: &EditorState, dispatch: Option<DispatchFn>) -> bool
pub fn close_history(tr: &mut Transaction) -> &mut Transaction
pub fn undo_depth(state: &EditorState) -> usize
pub fn redo_depth(state: &EditorState) -> usize
pub fn is_history_transaction(tr: &Transaction) -> bool
```

---

## commands 模块

**文件：** `reference/prosemirror-commands/src/commands.ts`

### Command 类型

```rust
// 所有命令的统一签名
pub type Command = fn(state: &EditorState, dispatch: Option<DispatchFn>) -> bool;
pub type DispatchFn = dyn Fn(Transaction);
```

### 对外暴露的命令

```rust
pub fn delete_selection(state, dispatch) -> bool
pub fn join_backward(state, dispatch) -> bool
pub fn join_forward(state, dispatch) -> bool
pub fn join_up(state, dispatch) -> bool
pub fn join_down(state, dispatch) -> bool
pub fn join_textblock_backward(state, dispatch) -> bool
pub fn join_textblock_forward(state, dispatch) -> bool
pub fn lift(state, dispatch) -> bool
pub fn lift_empty_block(state, dispatch) -> bool
pub fn select_node_backward(state, dispatch) -> bool
pub fn select_node_forward(state, dispatch) -> bool
pub fn select_parent_node(state, dispatch) -> bool
pub fn select_all(state, dispatch) -> bool
pub fn select_textblock_start(state, dispatch) -> bool
pub fn select_textblock_end(state, dispatch) -> bool
pub fn newline_in_code(state, dispatch) -> bool
pub fn exit_code(state, dispatch) -> bool
pub fn create_paragraph_near(state, dispatch) -> bool
pub fn split_block(state, dispatch) -> bool
pub fn split_block_keep_marks(state, dispatch) -> bool
pub fn split_block_as(split_node: ...) -> Command
pub fn wrap_in(node_type, attrs) -> Command
pub fn set_block_type(node_type, attrs) -> Command
pub fn toggle_mark(mark_type, attrs, options) -> Command
pub fn auto_join(command, is_joinable) -> Command
pub fn chain_commands(commands: &[Command]) -> Command
pub static PC_BASE_KEYMAP: &[(key, Command)]
pub static MAC_BASE_KEYMAP: &[(key, Command)]
pub static BASE_KEYMAP: &[(key, Command)]
```

---

## keymap 模块

**文件：** `reference/prosemirror-keymap/src/keymap.ts`

### 对外暴露

```rust
pub fn keymap(bindings: HashMap<String, Command>) -> Plugin
pub fn keydown_handler(bindings: HashMap<String, Command>) -> impl Fn(KeyEvent) -> bool
```

---

## markdown 模块

**文件：** `reference/prosemirror-markdown/src/`

### 对外暴露

```rust
pub struct MarkdownParser {
    pub schema: Arc<Schema>,
    pub tokens: HashMap<String, ParseSpec>,
}
impl MarkdownParser {
    pub fn parse(&self, text: &str) -> Node
}

pub struct MarkdownSerializer {
    pub nodes: HashMap<String, NodeSerializerFn>,
    pub marks: HashMap<String, MarkSerializerSpec>,
}
impl MarkdownSerializer {
    pub fn serialize(&self, content: &Node) -> String
}

pub static DEFAULT_MARKDOWN_PARSER: Lazy<MarkdownParser>
pub static DEFAULT_MARKDOWN_SERIALIZER: Lazy<MarkdownSerializer>
pub static MARKDOWN_SCHEMA: Lazy<Arc<Schema>>  // markdown 专用 schema
```

---

## 跨模块共享的关键类型

| 类型 | 定义在 | 被哪些模块使用 |
|------|--------|---------------|
| `Node` | model | 所有模块 |
| `Fragment` | model | 所有模块 |
| `Mark` | model | 所有模块 |
| `Slice` | model | transform, state |
| `Schema` | model | state, markdown |
| `NodeType` / `MarkType` | model | transform, state, commands |
| `ResolvedPos` | model | state, commands |
| `NodeRange` | model | transform, commands |
| `Transform` | transform | state（Transaction 继承） |
| `Step` / `StepMap` / `Mapping` | transform | state, history |
| `StepResult` | transform | state |
| `EditorState` | state | history, commands, keymap, markdown |
| `Transaction` | state | history, commands, keymap |
| `Selection` | state | commands |
| `Plugin` / `PluginKey` | state | history, keymap, inputrules |
| `Command` / `DispatchFn` | commands | keymap |

---

## 插件系统工作原理

```
EditorState
  └── plugins: Vec<Plugin>
        └── Plugin
              ├── spec.state: StateField<T>    // 插件私有状态
              │     ├── init(config, state) -> T
              │     └── apply(tr, old_val, old_state, new_state) -> T
              ├── spec.filter_transaction(tr, state) -> bool
              └── spec.append_transaction(trs, old_state, new_state) -> Option<Transaction>

// 每次 state.apply(tr)：
1. 对每个 plugin 调用 filter_transaction → 有一个返回 false 则丢弃 tr
2. 对每个 plugin 调用 state_field.apply → 更新插件状态
3. 对每个 plugin 调用 append_transaction → 可追加额外 tr（如 history 记录）
```

---

## 协同编辑（collab）数据流

```
本地操作：
  user action → Transaction → steps[] → 发送到服务器

收到远端步骤：
  remote steps → rebaseSteps(steps, remoteSteps, mapping)
               → 新 Transaction → state.apply()

关键函数（待实现）：
  collab(config) -> Plugin          // 创建协同插件
  send_able_steps(state) -> {version, steps, clientID}  // 获取待发送步骤
  receive_transaction(state, steps, clientIDs) -> Transaction  // 接收远端步骤
```
