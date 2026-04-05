# model 模块阶段计划

**设计文档：** `docs/superpowers/specs/2026-04-05-model-design.md`  
**参考源码：** `reference/prosemirror-model/src/`  
**实现目录：** `src/model/`

---

## 阶段总览

```
Phase 1 → Phase 2.1 → Phase 2.2 → Phase 2.3 → Phase 3 → Phase 4 → Phase 5
```

| 阶段 | 名称 | 文件 | 前置依赖 | 状态 |
|---|---|---|---|---|
| Phase 1 | 基础类型与 Spec | errors, compare_deep, render_output, schema spec 类型, Cargo.toml | 无 | ✅ 完成 |
| Phase 2.1 | ContentMatch DFA 核心 | content.rs（不含 Fragment 依赖方法） | Phase 1 | ✅ 完成 |
| Phase 2.2 | NodeType + MarkType | schema.rs 续（NodeType、MarkType） | Phase 2.1 | ✅ 完成 |
| Phase 2.3 | Schema::new 两遍构建 | schema.rs 续（Schema::new）+ tests_schema | Phase 2.2 | ⬜ 待开始 |
| Phase 3 | 文档类型 + ContentMatch 完整 | mark.rs, diff.rs, fragment.rs, node.rs + content.rs 续 + tests_content | Phase 2.3 | ⬜ 待开始 |
| Phase 4 | 导航与替换 | resolved_pos.rs, replace.rs + tests | Phase 3 | ⬜ 待开始 |
| Phase 5 | 渲染 Stub 与导出 | dom_parser.rs, dom_serializer.rs, mod.rs | Phase 4 | ⬜ 待开始 |

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

## Phase 3：文档类型 + ContentMatch 完整

**文件：**
- `src/model/mark.rs` — Mark + 所有方法 + `src/model/tests_mark.rs`
- `src/model/diff.rs` — find_diff_start, find_diff_end
- `src/model/fragment.rs` — Fragment + 所有方法
- `src/model/node.rs` — Node + 所有方法 + `src/model/tests_node.rs`
- `src/model/content.rs`（续）— 补全 match_fragment, fill_before, find_wrapping + `src/model/tests_content.rs`

**产出：** 可创建和操作文档树，所有 ContentMatch 方法完整，tests_content.rs 全部通过。

**依赖：** Phase 2.3（需要 Schema 创建测试文档）。

**详细计划：** `docs/superpowers/plans/model-phase3-plan.md`

---

## Phase 4：导航与替换

**文件：**
- `src/model/resolved_pos.rs` — ResolvedPos, NodeRange + `src/model/tests_resolve.rs`
- `src/model/replace.rs` — Slice, replace 算法 + `src/model/tests_replace.rs` + `src/model/tests_slice.rs`

**产出：** 可解析文档位置，可执行 slice/replace 操作，所有相关测试通过。

**依赖：** Phase 3。

**详细计划：** `docs/superpowers/plans/model-phase4-plan.md`

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
