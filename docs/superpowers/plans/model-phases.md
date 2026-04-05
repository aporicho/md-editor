# model 模块阶段计划

**设计文档：** `docs/superpowers/specs/2026-04-05-model-design.md`  
**参考源码：** `reference/prosemirror-model/src/`  
**实现目录：** `src/model/`

---

## 阶段总览

```
Phase 1 → Phase 2.1 → Phase 2.2 → Phase 2.3 → Phase 3.1 → Phase 3.2 → Phase 3.3-a → Phase 3.3-b → Phase 3.3-c → Phase 3.4-a → Phase 3.4-b → Phase 3.4-c → Phase 4.1 → Phase 4.2 → Phase 4.3 → Phase 4.4 → Phase 4.5 → Phase 4.6 → Phase 4.7 → Phase 5
```

| 阶段 | 名称 | 文件 | 前置依赖 | 状态 |
|---|---|---|---|---|
| Phase 1 | 基础类型与 Spec | errors, compare_deep, render_output, schema spec 类型, Cargo.toml | 无 | ✅ 完成 |
| Phase 2.1 | ContentMatch DFA 核心 | content.rs（不含 Fragment 依赖方法） | Phase 1 | ✅ 完成 |
| Phase 2.2 | NodeType + MarkType | schema.rs 续（NodeType、MarkType） | Phase 2.1 | ✅ 完成 |
| Phase 2.3 | Schema::new 两遍构建 | schema.rs 续（Schema::new）+ tests_schema | Phase 2.2 | ✅ 完成 |
| Phase 3.1 | Mark + MarkType stubs 补全 | mark.rs + tests_mark.rs + schema.rs（MarkType create/remove_from_set/is_in_set） | Phase 2.3 | ✅ 完成 |
| Phase 3.2 | Fragment + Node 全部方法 + diff | fragment.rs, node.rs（含所有方法）, diff.rs + tests_diff.rs, schema.rs stubs 补全 | Phase 3.1 | ✅ 完成 |
| Phase 3.3-a | 测试基础设施 test_builder | src/model/test_builder.rs — doc/p/h1/blockquote/em/strong/img/br 等快捷构建函数 | Phase 3.2 | ✅ 完成 |
| Phase 3.3-b | text_between leaf_text 回调扩展 | fragment.rs + node.rs — leaf_text 参数支持闭包形式 | Phase 3.2 | ✅ 完成 |
| Phase 3.3-c | tests_node.rs | tests_node.rs — 移植 test-node.ts 约 34 个测试（check 相关 4 个推迟到 Phase 3.4-c） | Phase 3.3-a + Phase 3.3-b | ✅ 完成 |
| Phase 3.4-a | match_fragment + valid_content | content.rs（match_fragment）+ schema.rs（NodeType::valid_content）+ 架构决策：fill_before/find_wrapping 增加 schema 参数 | Phase 3.3-c | ✅ 完成 |
| Phase 3.4-b | fill_before + create_and_fill | content.rs（fill_before）+ schema.rs（NodeType::create_and_fill）+ tests_content.rs fill 系列（28 个测试） | Phase 3.4-a | ⬜ 待开始 |
| Phase 3.4-c | find_wrapping + Node 方法补全 | content.rs（find_wrapping）+ node.rs（check/content_match_at/can_replace/can_replace_with/can_append）+ tests_content.rs matchType 系列 + 4 个 check 测试 | Phase 3.4-b | ⬜ 待开始 |
| Phase 4.1 | Slice | replace.rs（Slice struct + 所有方法）+ tests_slice.rs | Phase 3.4-c | ✅ 完成 |
| Phase 4.2 | ResolvedPos struct + resolve 算法 | resolved_pos.rs（struct + resolve + resolve_no_cache）+ 基础测试 | Phase 3.4 | ⬜ 待开始 |
| Phase 4.3 | ResolvedPos 简单访问器 | resolved_pos.rs 续（node/index/start/end/before/after/text_offset/node_after/node_before/pos_at_index） | Phase 4.2 | ⬜ 待开始 |
| Phase 4.4 | ResolvedPos 复杂方法 + NodeRange | resolved_pos.rs 续（marks/marks_across/block_range/shared_depth/same_parent/max/min）+ NodeRange + tests_resolve.rs | Phase 4.3 | ⬜ 待开始 |
| Phase 4.5 | replace 内部辅助函数 | replace.rs 续（remove_range/insert_into/add_node/add_range/close/check_join/prepare_slice_for_replace 等私有函数） | Phase 4.1 + Phase 4.4 | ⬜ 待开始 |
| Phase 4.6 | replace 算法积木块 | replace.rs 续（replace_two_way/replace_three_way） | Phase 4.5 | ⬜ 待开始 |
| Phase 4.7 | replace 主算法 + Node 方法补全 | replace.rs 续（replace_outer/replace）+ node.rs 补全（Node::replace/Node::slice）+ tests_replace.rs | Phase 4.6 | ⬜ 待开始 |
| Phase 5 | 渲染 Stub 与导出 | dom_parser.rs, dom_serializer.rs, mod.rs | Phase 4.7 | ⬜ 待开始 |

---

## Phase 1：基础类型与 Spec ✅

**文件：**
- `Cargo.toml` — 添加 indexmap, serde, serde_json 依赖
- `src/model/errors.rs` — SchemaError, ReplaceError, SyntaxError, RangeError
- `src/model/compare_deep.rs` — compare_deep 工具函数
- `src/model/render_output.rs` — RenderOutput 占位类型
- `src/model/schema.rs`（部分）— NodeSpec, MarkSpec, AttrSpec, SchemaSpec, Whitespace, ExcludeSpec, AttrValidator, Attrs 类型别名

**产出：** 可编译，errors 和 compare_deep 有完整测试（24 个测试全通过）。

**详细计划：** `docs/superpowers/plans/model-phase1-plan.md`

---

## Phase 2.1：ContentMatch DFA 核心

**文件：**
- `src/model/content.rs` — ContentMatch struct + NFA→DFA 引擎 + 不依赖 Fragment 的方法

**范围（content.rs）：**
- ContentMatch struct：`valid_end`, `inline_content`（bool，构建时计算）, `edges: Vec<(String, Arc<ContentMatch>)>`
- 常量：`ContentMatch::EMPTY`
- 内部：`TokenStream`, `Expr` 枚举, NFA 构建, DFA 构建
- 公开：`parse(expr, node_types)`, `match_type(name)`, `edge_count`, `edge(n)`, `inline_content` (字段), `compatible(&other)`
- `default_type()` 返回 `Option<String>`（类型名，由调用方查 Schema）
- **stub（todo!）**：`match_fragment`、`fill_before`、`find_wrapping`（依赖 Fragment，Phase 3 补全）

**产出：** ContentMatch 可从内容表达式字符串构建 DFA，match_type 可按名匹配。基本解析测试通过。

**依赖：** Phase 1 的所有类型。

**注意：** tests_content.rs 中 fill_before/matchFragment 测试依赖 Fragment，**推迟到 Phase 3**。

**详细计划：** `docs/superpowers/plans/model-phase2-1-plan.md`

---

## Phase 2.2：NodeType + MarkType

**文件：**
- `src/model/schema.rs`（续）— NodeType struct、MarkType struct 及所有不依赖 Node/Fragment/Mark 的方法

**范围（schema.rs 新增）：**
- NodeType struct：所有字段（含 `content_match: Arc<ContentMatch>`）
- NodeType 方法：`is_inline`, `is_block`, `is_text`, `is_leaf`, `is_atom`, `is_textblock`, `is_in_group`, `allows_mark_type`, `allows_marks`, `allowed_marks`, `valid_content`, `compatible_content`, `has_required_attrs`, `whitespace`
- NodeType **stub（todo!）**：`create`, `create_checked`, `create_and_fill`（依赖 Fragment/Node，Phase 3 补全）
- MarkType struct：所有字段（`excluded: ExcludeSpec`）
- MarkType 方法：`excludes`, `remove_from_set`, `is_in_set`
- MarkType **stub（todo!）**：`create`（依赖 Mark，Phase 3 补全）
- 内部辅助：`Attribute`, `compute_attrs`, `default_attrs`, `init_attrs`

**产出：** NodeType/MarkType 可实例化，is_* 谓词和 mark 权限检查可用。

**依赖：** Phase 2.1（NodeType 持有 `Arc<ContentMatch>`）。

**详细计划：** `docs/superpowers/plans/model-phase2-2-plan.md`

---

## Phase 2.3：Schema::new 两遍构建

**文件：**
- `src/model/schema.rs`（续）— Schema struct + `new()` + 测试

**范围（schema.rs 新增）：**
- Schema struct：`nodes`, `marks`, `top_node_type`, `linebreak_replacement`, `spec`
- `Schema::new(spec) -> Result<Arc<Self>, SchemaError>`
  - 第一遍：创建所有 NodeType / MarkType Arc（refcount = 1）
  - 第二遍：`Arc::get_mut` 填入 content_match 和 mark_set
- 公开方法：`node_type(name)`, `mark_type(name)`, `node(...)`, `text(...)`, `mark(...)`, `node_from_json`, `mark_from_json`（后两者 stub）
- 内部辅助：`gather_marks`, `resolve_mark_set`

**产出：** 可从 SchemaSpec 构建完整 Schema，NodeType/MarkType 的 content_match 和 mark_set 已填入。Schema 构建测试通过。

**依赖：** Phase 2.2。

**详细计划：** `docs/superpowers/plans/model-phase2-3-plan.md`

---

## Phase 3.3-a：测试基础设施 test_builder

**文件：**
- `src/model/test_builder.rs` — 仅 `#[cfg(test)]` 可见的测试工具模块

**范围：**
- 函数：`doc()`, `p()`, `h1()`, `h2()`, `blockquote()`, `pre()`, `ul()`, `li()`, `em()`, `strong()`, `code()`, `br()`, `img()`, `hr()` 等（参照 prosemirror-test-builder 的快捷 API）
- 内部使用简单 SchemaSpec（含 doc/paragraph/heading/blockquote/horizontal_rule/image/hard_break/text 节点，em/strong/link/code mark）
- 所有函数接收子节点列表，返回 Node

**产出：** 测试工具模块完整，Phase 3.3-c、3.4-b、3.4-c 可直接复用。

**依赖：** Phase 3.2（Node/Fragment/Schema）

---

## Phase 3.3-b：text_between leaf_text 回调扩展

**文件：**
- `src/model/fragment.rs` — `text_between` 的 `leaf_text` 参数
- `src/model/node.rs` — `text_between` / `text_content` 的 `leaf_text` 参数

**范围：**
- 当前 `leaf_text: Option<&str>` 只支持固定字符串，test-node.ts 中有一个测试传入闭包 `(node) => node.textContent`
- 新增 `LeafText<'_>` 枚举（`Str(&str)` / `Fn(Box<dyn Fn(&Node) -> String>)`），或改为泛型参数
- 同步更新 fragment.rs 的 `text_between`、node.rs 的 `text_between`/`text_content`
- 调用方（tests_mark.rs 等现有测试）不受影响（仍传 `None` 或 `Some(str)`）

**产出：** leaf_text 支持回调，所有现有测试仍通过。

**依赖：** Phase 3.2

---

## Phase 3.3-c：tests_node.rs

**文件：**
- `src/model/tests_node.rs` — 移植 test-node.ts，约 34 个测试
- `src/model/mod.rs` — 声明 `#[cfg(test)] mod tests_node`

**范围（覆盖 test-node.ts 中不依赖 check/valid_content 的所有测试）：**
- `toString`（3）：节点/marks 的调试字符串格式
- `cut`（6）：整块、文本、深层、左右、保留 marks
- `between`（3）：nodesBetween 迭代文本、多层、行内节点
- `textBetween`（6）：blockSeparator、leafText 字符串、leafText 回调、空段落、叶节点
- `textContent`（3）：整文档、文本节点、嵌套元素
- `from`（5）：Fragment::from_node/from_array/from_fragment/from_null/合并相邻文本
- `toJSON`（5）：序列化/反序列化 simple/marks/inline-leaf/block-leaf/nested
- `toString（Fragment）`（3）：toStringInner 格式、toDebugString 回调
- `leafText`（1）：自定义 textContent

4 个 `check` 测试推迟到 Phase 3.4-c 补写。

**产出：** tests_node.rs 34 个测试全部通过。

**依赖：** Phase 3.3-a（test_builder）、Phase 3.3-b（leaf_text 回调）

---

## Phase 3.4-a：match_fragment + valid_content + 架构决策

**文件：**
- `src/model/content.rs` — 实现 `match_fragment`
- `src/model/schema.rs` — 实现 `NodeType::valid_content`

**架构决策（记录为设计决策 28）：**
`fill_before` 和 `find_wrapping` 需要通过类型名查询 NodeType 属性（`is_leaf`, `has_required_attrs`, `content_match`, `create_and_fill`）。当前 ContentMatch::edges 存储 `(String, Arc<ContentMatch>)`，不持有 NodeType 引用（避免循环 Arc）。

**决定：** `fill_before` 和 `find_wrapping` 签名增加 `schema: &Schema` 参数，在调用时按类型名查 Schema。API 从 TS 的 `fillBefore(after, toEnd)` 变为 `fill_before(schema, after, to_end, start_index)`，与 Decision 6（出边存类型名）保持一致。

**范围：**
- `ContentMatch::match_fragment(schema: &Schema, frag: &Fragment, start: usize, end: usize) -> Option<Arc<ContentMatch>>` — 约 15 行，逐个子节点调用 `match_type`
- `NodeType::valid_content(schema: &Schema, frag: &Fragment) -> bool` — 调用 `match_fragment`，替换 `valid_content_stub`
- `Node::content_match_at(schema: &Schema, index: usize) -> Option<Arc<ContentMatch>>` — 替换 `content_match_at_stub`

**产出：** match_fragment/valid_content/content_match_at 可用，现有测试全部通过。

**依赖：** Phase 3.3-c

---

## Phase 3.4-b：fill_before + create_and_fill + fill 测试

**文件：**
- `src/model/content.rs` — 实现 `fill_before`
- `src/model/schema.rs` — 实现 `NodeType::create_and_fill`
- `src/model/tests_content.rs` — fill_before 系列 28 个测试

**范围：**
- `NodeType::create_and_fill(schema: &Arc<Schema>, attrs, marks) -> Option<Node>` — 用 `content_match.fill_before(schema, Fragment::empty(), true, 0)` 填充内容，替换 `create_and_fill_stub`
- `ContentMatch::fill_before(schema: &Schema, after: &Fragment, to_end: bool, start_index: usize) -> Option<Fragment>` — DFS，记忆化已访问 ContentMatch 指针，对每条 edge 调用 `has_required_attrs` 和 `create_and_fill`
- `tests_content.rs` fill/fill3 辅助函数 + 28 个 fillBefore 测试（完整移植 test-content.ts fillBefore 块）

**产出：** fill_before 和 create_and_fill 实现完毕，28 个 fill 测试全部通过。

**依赖：** Phase 3.4-a

---

## Phase 3.4-c：find_wrapping + Node 方法补全 + 剩余测试

**文件：**
- `src/model/content.rs` — 实现 `find_wrapping`
- `src/model/node.rs` — 实现 `check`, `can_replace`, `can_replace_with`, `can_append`
- `src/model/tests_content.rs` — matchType 系列测试（补全未覆盖部分）
- `src/model/tests_node.rs` — 补写 4 个 check 测试

**范围：**
- `ContentMatch::find_wrapping(schema: &Schema, target: &str) -> Option<Vec<String>>` — BFS，wrap_cache（已有 Mutex<HashMap>），跳过 is_leaf/has_required_attrs 的类型
- `Node::check(schema: &Schema)` — 调用 valid_content + check_attrs + 校验 marks，替换 `check_stub`
- `Node::can_replace(schema, from, to, replacement, start, end) -> bool`
- `Node::can_replace_with(schema, from, to, type_name, marks) -> bool`
- `Node::can_append(schema, other) -> bool`
- tests_content.rs matchType 系列中未被 content.rs 内联测试覆盖的用例
- tests_node.rs 补写：invalid_content、marks_in_wrong_places、incorrect_mark_sets、wrong_attribute_types

**产出：** ContentMatch 全部方法完整，Node::check 系列可用，所有 tests_content.rs 和 tests_node.rs 通过。

**依赖：** Phase 3.4-b

---

## Phase 4.1：Slice

**文件：** `src/model/replace.rs`（Slice 部分）+ `src/model/tests_slice.rs`

**范围：** Slice struct（content/open_start/open_end）、size、eq、to_string、to_json、from_json、max_open、empty 常量、insert_at、remove_between

**产出：** Slice 可构建，tests_slice.rs 全部通过。

**依赖：** Phase 3.4（Fragment）

---

## Phase 4.2：ResolvedPos struct + resolve 算法

**文件：** `src/model/resolved_pos.rs`（核心部分）

**范围：** ResolvedPos struct 定义、`resolve()` 静态方法（路径解析算法）、`resolve_no_cache()`、基础测试验证 resolve 正确。

**产出：** 给定文档和位置，可正确构建 ResolvedPos 路径。

**依赖：** Phase 3.4（Node/Fragment）

---

## Phase 4.3：ResolvedPos 简单访问器

**文件：** `src/model/resolved_pos.rs`（续）

**范围：** depth、parent、doc、node、index、index_after、start、end、before、after、text_offset、node_after、node_before、pos_at_index

**产出：** 所有基础访问器可用，基础导航测试通过。

**依赖：** Phase 4.2

---

## Phase 4.4：ResolvedPos 复杂方法 + NodeRange

**文件：** `src/model/resolved_pos.rs`（续）

**范围：** marks、marks_across、shared_depth、block_range、same_parent、max、min；NodeRange struct（from/to/depth）及所有方法（start/end/parent/start_index/end_index）；tests_resolve.rs 完整测试。

**产出：** ResolvedPos 和 NodeRange 全部完整，tests_resolve.rs 全部通过。

**依赖：** Phase 4.3

---

## Phase 4.5：replace 内部辅助函数

**文件：** `src/model/replace.rs`（replace 算法部分）

**范围：** 所有私有辅助函数：remove_range、insert_into、add_node、add_range、close、check_join、joinable、prepare_slice_for_replace、content_between 等。无独立测试，由 Phase 4.7 集成测试覆盖。

**产出：** 所有辅助函数实现完毕，可编译通过。

**依赖：** Phase 4.1（Slice）+ Phase 4.4（ResolvedPos）

---

## Phase 4.6：replace 算法积木块

**文件：** `src/model/replace.rs`（续）

**范围：** replace_two_way、replace_three_way（两个核心递归函数，是 replace_outer 的构建基础）。

**产出：** 两个算法函数实现完毕，可编译通过。

**依赖：** Phase 4.5

---

## Phase 4.7：replace 主算法 + Node 方法补全

**文件：** `src/model/replace.rs`（续）+ `src/model/node.rs`（补全）

**范围：** replace_outer、replace（公开入口）；node.rs 补全 Node::replace、Node::slice；tests_replace.rs 完整测试。

**产出：** replace 算法完整，Node::replace/Node::slice 可用，tests_replace.rs 全部通过。

**依赖：** Phase 4.6

---

## Phase 5：渲染 Stub 与导出

**文件：**
- `src/model/dom_parser.rs` — DOMParser stub（todo! 占位）
- `src/model/dom_serializer.rs` — DOMSerializer stub（todo! 占位）
- `src/model/mod.rs` — 所有公开类型的 pub use 导出

**产出：** model 模块完整可用，所有公开 API 可从外部访问。

**依赖：** Phase 4 全部完成。

**详细计划：** `docs/superpowers/plans/model-phase5-plan.md`

---

## 完成标准

所有阶段完成后：
- [ ] `cargo test` 全部通过（dom stubs 除外）
- [ ] 所有 TS 测试文件（test-content, test-diff, test-mark, test-node, test-replace, test-resolve, test-slice）均有对应 Rust 测试且通过
- [ ] `src/model/mod.rs` 导出所有公开类型
- [ ] 无 `unsafe`，无非 dom-stub / 外部依赖 stub 的 `todo!()`

---

## 拆分原则（为何这样划分）

- **Fragment 依赖是关键边界**：ContentMatch.fill_before / match_fragment / find_wrapping 都需要 Fragment，而 Fragment 是 Phase 3 才有的。强行在 Phase 2 实现会导致 stub 扩散。
- **每个子阶段独立可编译**：Phase 2.1/2.2/2.3 各自产出可编译、有测试的代码，不依赖后续阶段。
- **stub 只在明确的外部依赖缺失时使用**：凡是本模块内部可以实现的方法，必须在对应阶段写完。
