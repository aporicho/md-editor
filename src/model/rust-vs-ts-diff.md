# Model 模块：Rust 实现 vs TypeScript 原版差异对照

> 基于 ProseMirror/prosemirror-model 最新 master 与本项目 src/model/ 的逐项对比。

---

## 一、架构层面的根本性变更

| 差异 | TypeScript | Rust | 原因 |
|------|-----------|------|------|
| **Schema 反向引用** | `NodeType.schema` / `MarkType.schema` 持有 Schema 引用 | 移除；需要 Schema 的方法改用 `&Schema` 参数 | 避免 `Arc` 循环引用 |
| **ContentMatch 边** | 边存储 `NodeType` 引用 | 边存储类型名 `String`，运行时查 Schema | 避免 Arc 循环 |
| **Mark 排除** | `excluded: MarkType[]` | `ExcludeSpec` 枚举 (`SelfOnly` / `All` / `Named(Vec<String>)`) | 避免 MarkType 自引用 |
| **Schema 构建** | 单趟构建 | 两阶段：先建 Arc，再 `Arc::get_mut` 填充 | refcount=1 时安全修改 |
| **ContentMatch DFA** | Arc 递归构建 | Arena + 索引构建，最后转 `Arc<ContentMatch>` | 避免 `Arc::into_inner` panic |
| **ResolvedPos 缓存** | `WeakMap<Node, ResolveCache>` 自动 GC | **未实现缓存** | Rust 无 WeakMap 等价物，等性能测试 |

---

## 二、类型系统适配

| 差异 | TypeScript | Rust |
|------|-----------|------|
| **Node 继承** | `TextNode extends Node` 子类 | 单一 `struct Node`，`text: Option<String>` 区分 |
| **Attrs** | `{[attr: string]: any}` | `Arc<HashMap<String, serde_json::Value>>` |
| **marks 字段** | `readonly Mark[]` | `Arc<[Mark]>`（零拷贝共享） |
| **Mark.none** | `static none = []` | `OnceLock<Arc<[Mark]>>`（线程安全懒初始化） |
| **Fragment.content** | `readonly Node[]` | `Vec<Arc<Node>>`（避免生命周期传播） |
| **Fragment.empty** | 静态字段 | `OnceLock<Fragment>` |
| **ResolvedPos.path** | `any[]` 混合类型数组（手算偏移 `path[d*3]`） | `Vec<PathFrame>` 命名字段结构体 |
| **findDiffEnd 返回** | `{a: number, b: number} \| null` | `Option<DiffEnd>` 命名结构体 |
| **错误处理** | `throw new ReplaceError(...)` 异常 | `Result<T, ReplaceError>` 四种错误类型 |

---

## 三、API 差异（方法级）

### 3.1 缺失的 API

| TS 方法 | 状态 | 说明 |
|---------|------|------|
| `Node.children` getter | **缺失** | TS 返回 `this.content.content` |
| `Fragment.from()` 多态构造器 | **改为 From trait** | `From<Node>` / `From<Vec<Node>>` 替代 |
| `ResolvedPos.resolveCached()` | **缺失** | 无缓存实现 |
| `Schema.cached` | **缺失** | TS 的 schema 级缓存对象 |
| `Schema.nodeFromJSON` / `markFromJSON` 绑定闭包 | **改为普通方法** | Rust 不需要 bind this |
| `ContentMatch.empty` 单例 | **改为函数** | `ContentMatch::empty()` |
| `DOMParser` 全部方法 | **stub** | `todo!()` 等 cosmic-text |
| `DOMSerializer` 全部方法 | **stub** | `todo!()` 等 cosmic-text |
| `DOMOutputSpec` 类型 | **不适用** | 非 DOM 环境 |
| `ParseRule` / `TagParseRule` / `StyleParseRule` | **stub** | 非 DOM 环境 |

### 3.2 重命名的 API

| TS | Rust | 说明 |
|----|------|------|
| `Node.mark(marks)` | `Node::with_marks(marks)` | 避免与 `Mark` 类型混淆 |
| `Node.toString()` | `Display` trait + `to_string_repr()` | Rust 惯例 |
| `Fragment.forEach()` | 保留，但同时支持 `iter()` | Rust 迭代器惯例 |
| `Fragment.findIndex()` 返回对象 | 返回 `(usize, usize)` 元组 | TS 用全局对象复用避免分配 |
| `Mark.eq()` | `PartialEq` trait（`==` 操作符） | Rust 惯例 |
| `NodeSpec.toDOM` / `parseDOM` | `to_render` / `RenderOutput` | DOM → Canvas 渲染 |

### 3.3 新增的 API

| Rust 方法 | 说明 |
|-----------|------|
| `Node::ptr_eq()` | 指针相等（Arc 地址比较） |
| `Node::text_node()` 构造器 | 显式文本节点构建 |
| `NodeType::inline_content` 字段 | TS 隐式，Rust 显式字段 |
| `Whitespace` 枚举 | 显式 `Normal` / `Pre` |
| `AttrValidator::Func` | 支持自定义闭包验证器 |
| `Node::create_and_fill()` 递归填充 | TS 只填一层，Rust 递归填充嵌套 |

---

## 四、语义差异

| 项目 | TS 行为 | Rust 行为 |
|------|---------|-----------|
| **文本索引** | UTF-16 | UTF-8 char 索引（`char_indices()`） |
| **is_leaf 判断** | 指针等式 `contentMatch == ContentMatch.empty` | `edge_count() == 0 && valid_end` |
| **JSON 序列化** | serde Deserialize | 手写 `to_json()` / `from_json(&Schema)` |
| **Fragment::cut_by_index** | 可能合并相邻文本节点 | 不合并，直接切片 |
| **Mark inclusive 默认** | 隐式 | `Option<bool>`，`None` = 默认 `true` |

---

## 五、已知 Bug 修复（与 TS 行为不同）

| Bug | TS 行为 | Rust 修正 |
|-----|---------|-----------|
| `ResolvedPos::after()` | `path[d-1].node.node_size()` | `self.path[d-1].start + self.path[d].node.node_size()` |
| `block_range()` 起始深度 | 检查 `parent().content.size == 0` | 改用 `parent().node_type.inline_content` |
| `node()` 负深度 | unsigned wrap 后 panic | 显式边界检查 + 有意义的错误信息 |
| `find_wrapping()` 路径重建 | 递归 `via` 字段（丢失中间节点） | 索引回溯（`via: Option<usize>`），保留完整路径 |

---

## 六、Stub 清单（待实现）

| 文件 | 内容 | 阻塞因素 |
|------|------|----------|
| `render_output.rs` | `enum RenderOutput { Pending }` | 等 cosmic-text 渲染层 |
| `dom_parser.rs` | `DOMParser::parse()` / `parse_slice()` 返回 `todo!()` | 等 cosmic-text 渲染层 |
| `dom_serializer.rs` | `DOMSerializer::serialize_node()` / `serialize_fragment()` 返回 `todo!()` | 等 cosmic-text 渲染层 |

---

## 七、dead_code 注解现状

所有核心文件顶部带有 `#![allow(dead_code)]`，标注原因为后续模块（transform/state/view）尚未调用：

- `node.rs` — `// TODO: Phase 3.2`
- `fragment.rs` — `// TODO: Phase 3.2`
- `mark.rs` — `// TODO: Phase 3.2+`
- `schema.rs` — `// TODO: Phase 2`
- `content.rs` — `// TODO: Phase 3`
- `resolved_pos.rs` — `// TODO: Phase 4.4`
