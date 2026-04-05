# model 模块阶段计划

**设计文档：** `docs/superpowers/specs/2026-04-05-model-design.md`  
**参考源码：** `reference/prosemirror-model/src/`  
**实现目录：** `src/model/`

---

## 阶段总览

```
Phase 1 → Phase 2 → Phase 3 → Phase 4 → Phase 5
```

| 阶段 | 名称 | 文件 | 前置依赖 | 状态 |
|---|---|---|---|---|
| Phase 1 | 基础类型与 Spec | errors, compare_deep, render_output, schema spec 类型, Cargo.toml | 无 | ✅ 完成 |
| Phase 2 | ContentMatch + Schema 核心 | content.rs, schema.rs（NodeType/MarkType/Schema::new） | Phase 1 | ⬜ 待开始 |
| Phase 3 | 文档类型 | mark.rs, diff.rs, fragment.rs, node.rs | Phase 2 | ⬜ 待开始 |
| Phase 4 | 导航与替换 | resolved_pos.rs, replace.rs, tests_content.rs | Phase 3 | ⬜ 待开始 |
| Phase 5 | 渲染 Stub 与导出 | dom_parser.rs, dom_serializer.rs, mod.rs | Phase 4 | ⬜ 待开始 |

---

## Phase 1：基础类型与 Spec

**文件：**
- `Cargo.toml` — 添加 indexmap, serde, serde_json 依赖
- `src/model/errors.rs` — SchemaError, ReplaceError, SyntaxError, RangeError
- `src/model/compare_deep.rs` — compare_deep 工具函数
- `src/model/render_output.rs` — RenderOutput 占位类型
- `src/model/schema.rs`（部分）— NodeSpec, MarkSpec, AttrSpec, SchemaSpec, Whitespace, ExcludeSpec, AttrValidator, Attrs 类型别名

**产出：** 可编译，errors 和 compare_deep 有完整测试。

**详细计划：** `docs/superpowers/plans/model-phase1-plan.md`

---

## Phase 2：ContentMatch + Schema 核心

**文件：**
- `src/model/content.rs` — ContentMatch 完整实现（含 NFA→DFA 解析器）
- `src/model/schema.rs`（续）— NodeType, MarkType, Schema::new（两遍构建）

**产出：** 可以从 SchemaSpec 构建 Schema，ContentMatch 可解析内容表达式。Schema 有完整测试。

**依赖：** Phase 1 的所有类型必须已定义。

**注意：** ContentMatch 的测试（tests_content.rs）依赖 Schema，在本阶段一并完成。

**详细计划：** `docs/superpowers/plans/model-phase2-plan.md`

---

## Phase 3：文档类型

**文件：**
- `src/model/mark.rs` — Mark + 所有方法 + `src/model/tests_mark.rs`
- `src/model/diff.rs` — find_diff_start, find_diff_end
- `src/model/fragment.rs` — Fragment + 所有方法（含 diff 测试）
- `src/model/node.rs` — Node + 所有方法 + `src/model/tests_node.rs`

**产出：** 可创建和操作文档树，所有文档类型测试通过。

**依赖：** Phase 2（需要 Schema 创建测试文档）。

**详细计划：** `docs/superpowers/plans/model-phase3-plan.md`

---

## Phase 4：导航与替换

**文件：**
- `src/model/resolved_pos.rs` — ResolvedPos, NodeRange + `src/model/tests_resolve.rs`
- `src/model/replace.rs` — Slice, replace 算法 + `src/model/tests_replace.rs` + `src/model/tests_slice.rs`

**产出：** 可解析文档位置，可执行 slice/replace 操作，所有相关测试通过。

**依赖：** Phase 3（需要完整的 Node/Fragment）。

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

所有 5 个阶段完成后：
- [ ] `cargo test` 全部通过（dom stubs 除外）
- [ ] 所有 TS 测试文件（test-content, test-diff, test-mark, test-node, test-replace, test-resolve, test-slice）均有对应 Rust 测试且通过
- [ ] `src/model/mod.rs` 导出所有公开类型
- [ ] 无 `unsafe`，无非 dom-stub 的 `todo!()`
