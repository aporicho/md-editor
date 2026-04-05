# model Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 model 模块的基础类型：错误类型、compare_deep 工具、RenderOutput 占位、以及所有 Spec/配置结构体。

**Architecture:** 5 个独立文件，无相互依赖。errors.rs 和 compare_deep.rs 有完整单元测试。schema.rs 在本阶段只写 Spec 类型（NodeSpec/MarkSpec/AttrSpec/SchemaSpec/Whitespace/ExcludeSpec/AttrValidator/Attrs），NodeType/MarkType/Schema::new 留给 Phase 2。

**Tech Stack:** Rust 2024 edition, indexmap 2, serde 1, serde_json 1

---

## 文件结构

```
Cargo.toml                          ← 添加依赖
src/model/errors.rs                 ← SchemaError, ReplaceError, SyntaxError, RangeError
src/model/compare_deep.rs           ← compare_deep(a, b) -> bool
src/model/render_output.rs          ← RenderOutput 占位类型
src/model/schema.rs                 ← Spec 类型（本阶段部分实现）
src/model/mod.rs                    ← 声明子模块（本阶段只 mod 声明，不 pub use）
```

---

## Task 1: 添加依赖

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: 更新 Cargo.toml**

```toml
[package]
name = "md-editor"
version = "0.1.0"
edition = "2024"

[dependencies]
iced = { version = "0.14", features = ["advanced", "highlighter", "markdown"] }
open = "5"
indexmap = { version = "2", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **Step 2: 验证编译**

```bash
cargo check
```

Expected: 无报错

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore(model): add indexmap, serde, serde_json dependencies"
```

---

## Task 2: errors.rs

**Files:**
- Create: `src/model/errors.rs`
- Modify: `src/model/mod.rs`

- [ ] **Step 1: 写测试**

在 `src/model/errors.rs` 末尾添加测试模块（先写测试，文件其余部分暂为空）：

```rust
// src/model/errors.rs (测试先行，实现后补)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_error_display() {
        let e = SchemaError("bad schema".to_string());
        assert_eq!(e.to_string(), "SchemaError: bad schema");
    }

    #[test]
    fn replace_error_display() {
        let e = ReplaceError("bad replace".to_string());
        assert_eq!(e.to_string(), "ReplaceError: bad replace");
    }

    #[test]
    fn syntax_error_display() {
        let e = SyntaxError("bad expr".to_string());
        assert_eq!(e.to_string(), "SyntaxError: bad expr");
    }

    #[test]
    fn range_error_display() {
        let e = RangeError("out of range".to_string());
        assert_eq!(e.to_string(), "RangeError: out of range");
    }

    #[test]
    fn errors_implement_std_error() {
        let e: Box<dyn std::error::Error> = Box::new(SchemaError("x".to_string()));
        assert_eq!(e.to_string(), "SchemaError: x");
    }
}
```

- [ ] **Step 2: 在 mod.rs 声明模块**

```rust
// src/model/mod.rs
pub mod errors;
```

- [ ] **Step 3: 运行测试，确认失败**

```bash
cargo test -p md-editor model::errors
```

Expected: FAIL（类型未定义）

- [ ] **Step 4: 实现 errors.rs**

```rust
// src/model/errors.rs
use std::fmt;

/// Schema 构建或节点校验失败时抛出的错误。
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaError(pub String);

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SchemaError: {}", self.0)
    }
}

impl std::error::Error for SchemaError {}

/// Node::replace 操作失败时抛出的错误。
#[derive(Debug, Clone, PartialEq)]
pub struct ReplaceError(pub String);

impl fmt::Display for ReplaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ReplaceError: {}", self.0)
    }
}

impl std::error::Error for ReplaceError {}

/// ContentMatch::parse 解析内容表达式失败时抛出的错误。
#[derive(Debug, Clone, PartialEq)]
pub struct SyntaxError(pub String);

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SyntaxError: {}", self.0)
    }
}

impl std::error::Error for SyntaxError {}

/// 位置越界等范围错误。
#[derive(Debug, Clone, PartialEq)]
pub struct RangeError(pub String);

impl fmt::Display for RangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RangeError: {}", self.0)
    }
}

impl std::error::Error for RangeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_error_display() {
        let e = SchemaError("bad schema".to_string());
        assert_eq!(e.to_string(), "SchemaError: bad schema");
    }

    #[test]
    fn replace_error_display() {
        let e = ReplaceError("bad replace".to_string());
        assert_eq!(e.to_string(), "ReplaceError: bad replace");
    }

    #[test]
    fn syntax_error_display() {
        let e = SyntaxError("bad expr".to_string());
        assert_eq!(e.to_string(), "SyntaxError: bad expr");
    }

    #[test]
    fn range_error_display() {
        let e = RangeError("out of range".to_string());
        assert_eq!(e.to_string(), "RangeError: out of range");
    }

    #[test]
    fn errors_implement_std_error() {
        let e: Box<dyn std::error::Error> = Box::new(SchemaError("x".to_string()));
        assert_eq!(e.to_string(), "SchemaError: x");
    }
}
```

- [ ] **Step 5: 运行测试，确认通过**

```bash
cargo test -p md-editor model::errors
```

Expected:
```
running 5 tests
test model::errors::tests::schema_error_display ... ok
test model::errors::tests::replace_error_display ... ok
test model::errors::tests::syntax_error_display ... ok
test model::errors::tests::range_error_display ... ok
test model::errors::tests::errors_implement_std_error ... ok
test result: ok. 5 passed; 0 failed
```

- [ ] **Step 6: Commit**

```bash
git add src/model/errors.rs src/model/mod.rs
git commit -m "feat(model): add error types SchemaError, ReplaceError, SyntaxError, RangeError"
```

---

## Task 3: compare_deep.rs

**Files:**
- Create: `src/model/compare_deep.rs`
- Modify: `src/model/mod.rs`

TS 原文：`reference/prosemirror-model/src/comparedeep.ts`

- [ ] **Step 1: 写测试**

```rust
// src/model/compare_deep.rs（测试先行）
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn equal_primitives() {
        assert!(compare_deep(&json!(1), &json!(1)));
        assert!(compare_deep(&json!("a"), &json!("a")));
        assert!(compare_deep(&json!(true), &json!(true)));
        assert!(compare_deep(&json!(null), &json!(null)));
    }

    #[test]
    fn unequal_primitives() {
        assert!(!compare_deep(&json!(1), &json!(2)));
        assert!(!compare_deep(&json!("a"), &json!("b")));
        assert!(!compare_deep(&json!(true), &json!(false)));
    }

    #[test]
    fn type_mismatch() {
        assert!(!compare_deep(&json!(1), &json!("1")));
        assert!(!compare_deep(&json!(null), &json!(false)));
        assert!(!compare_deep(&json!(1), &json!(null)));
    }

    #[test]
    fn equal_objects() {
        assert!(compare_deep(&json!({"a": 1, "b": 2}), &json!({"a": 1, "b": 2})));
    }

    #[test]
    fn unequal_objects() {
        assert!(!compare_deep(&json!({"a": 1}), &json!({"a": 2})));
        assert!(!compare_deep(&json!({"a": 1}), &json!({"b": 1})));
        assert!(!compare_deep(&json!({"a": 1}), &json!({"a": 1, "b": 2})));
    }

    #[test]
    fn equal_arrays() {
        assert!(compare_deep(&json!([1, 2, 3]), &json!([1, 2, 3])));
        assert!(compare_deep(&json!([]), &json!([])));
    }

    #[test]
    fn unequal_arrays() {
        assert!(!compare_deep(&json!([1, 2]), &json!([1, 3])));
        assert!(!compare_deep(&json!([1, 2]), &json!([1, 2, 3])));
    }

    #[test]
    fn nested() {
        assert!(compare_deep(
            &json!({"a": [1, {"b": 2}]}),
            &json!({"a": [1, {"b": 2}]})
        ));
        assert!(!compare_deep(
            &json!({"a": [1, {"b": 2}]}),
            &json!({"a": [1, {"b": 3}]})
        ));
    }
}
```

- [ ] **Step 2: 在 mod.rs 声明模块**

```rust
// src/model/mod.rs
pub mod errors;
pub mod compare_deep;
```

- [ ] **Step 3: 运行测试，确认失败**

```bash
cargo test -p md-editor model::compare_deep
```

Expected: FAIL（函数未定义）

- [ ] **Step 4: 实现 compare_deep.rs**

```rust
// src/model/compare_deep.rs
use serde_json::Value;

/// Recursively compare two JSON values for deep equality.
pub fn compare_deep(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Number(a), Value::Number(b)) => a == b,
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| compare_deep(x, y))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter().all(|(k, v)| b.get(k).map_or(false, |bv| compare_deep(v, bv)))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn equal_primitives() {
        assert!(compare_deep(&json!(1), &json!(1)));
        assert!(compare_deep(&json!("a"), &json!("a")));
        assert!(compare_deep(&json!(true), &json!(true)));
        assert!(compare_deep(&json!(null), &json!(null)));
    }

    #[test]
    fn unequal_primitives() {
        assert!(!compare_deep(&json!(1), &json!(2)));
        assert!(!compare_deep(&json!("a"), &json!("b")));
        assert!(!compare_deep(&json!(true), &json!(false)));
    }

    #[test]
    fn type_mismatch() {
        assert!(!compare_deep(&json!(1), &json!("1")));
        assert!(!compare_deep(&json!(null), &json!(false)));
        assert!(!compare_deep(&json!(1), &json!(null)));
    }

    #[test]
    fn equal_objects() {
        assert!(compare_deep(&json!({"a": 1, "b": 2}), &json!({"a": 1, "b": 2})));
    }

    #[test]
    fn unequal_objects() {
        assert!(!compare_deep(&json!({"a": 1}), &json!({"a": 2})));
        assert!(!compare_deep(&json!({"a": 1}), &json!({"b": 1})));
        assert!(!compare_deep(&json!({"a": 1}), &json!({"a": 1, "b": 2})));
    }

    #[test]
    fn equal_arrays() {
        assert!(compare_deep(&json!([1, 2, 3]), &json!([1, 2, 3])));
        assert!(compare_deep(&json!([]), &json!([])));
    }

    #[test]
    fn unequal_arrays() {
        assert!(!compare_deep(&json!([1, 2]), &json!([1, 3])));
        assert!(!compare_deep(&json!([1, 2]), &json!([1, 2, 3])));
    }

    #[test]
    fn nested() {
        assert!(compare_deep(
            &json!({"a": [1, {"b": 2}]}),
            &json!({"a": [1, {"b": 2}]})
        ));
        assert!(!compare_deep(
            &json!({"a": [1, {"b": 2}]}),
            &json!({"a": [1, {"b": 3}]})
        ));
    }
}
```

- [ ] **Step 5: 运行测试，确认通过**

```bash
cargo test -p md-editor model::compare_deep
```

Expected:
```
running 8 tests
test model::compare_deep::tests::equal_primitives ... ok
test model::compare_deep::tests::unequal_primitives ... ok
test model::compare_deep::tests::type_mismatch ... ok
test model::compare_deep::tests::equal_objects ... ok
test model::compare_deep::tests::unequal_objects ... ok
test model::compare_deep::tests::equal_arrays ... ok
test model::compare_deep::tests::unequal_arrays ... ok
test model::compare_deep::tests::nested ... ok
test result: ok. 8 passed; 0 failed
```

- [ ] **Step 6: Commit**

```bash
git add src/model/compare_deep.rs src/model/mod.rs
git commit -m "feat(model): add compare_deep for recursive JSON value equality"
```

---

## Task 4: render_output.rs

**Files:**
- Create: `src/model/render_output.rs`
- Modify: `src/model/mod.rs`

- [ ] **Step 1: 创建 render_output.rs**

```rust
// src/model/render_output.rs

/// Rendering output placeholder type.
///
/// Replaces the browser DOM output types (DOMOutputSpec) from prosemirror-model.
/// This is a placeholder until the cosmic-text rendering layer is implemented.
/// At that point, this type will be replaced with the actual cosmic-text output type.
#[derive(Debug, Clone)]
pub struct RenderOutput(pub serde_json::Value);
```

- [ ] **Step 2: 在 mod.rs 声明模块**

```rust
// src/model/mod.rs
pub mod errors;
pub mod compare_deep;
pub mod render_output;
```

- [ ] **Step 3: 验证编译**

```bash
cargo check
```

Expected: 无报错

- [ ] **Step 4: Commit**

```bash
git add src/model/render_output.rs src/model/mod.rs
git commit -m "feat(model): add RenderOutput placeholder type for cosmic-text integration"
```

---

## Task 5: schema.rs — Spec 类型

**Files:**
- Create: `src/model/schema.rs`
- Modify: `src/model/mod.rs`

TS 原文：`reference/prosemirror-model/src/schema.ts`（NodeSpec、MarkSpec、AttributeSpec、SchemaSpec 接口部分）

- [ ] **Step 1: 写测试**

```rust
// 在 src/model/schema.rs 末尾写测试（文件其余部分暂为空）
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn node_spec_default() {
        let spec = NodeSpec::default();
        assert!(spec.content.is_none());
        assert!(spec.marks.is_none());
        assert!(spec.group.is_none());
        assert!(!spec.inline);
        assert!(!spec.atom);
        assert!(!spec.code);
        assert!(spec.selectable);   // TS default: true
        assert!(!spec.draggable);
        assert!(!spec.isolating);
        assert!(!spec.defining);
        assert!(!spec.linebreak_replacement);
    }

    #[test]
    fn mark_spec_default() {
        let spec = MarkSpec::default();
        assert!(spec.inclusive);    // TS default: true
        assert!(spec.spanning);     // TS default: true
        assert!(!spec.code);
        assert!(spec.excludes.is_none());
        assert!(spec.group.is_none());
    }

    #[test]
    fn attr_spec_with_default() {
        let spec = AttrSpec { default: Some(json!("hello")), validate: None };
        assert!(spec.is_required() == false);
        assert_eq!(spec.default, Some(json!("hello")));
    }

    #[test]
    fn attr_spec_required() {
        let spec = AttrSpec { default: None, validate: None };
        assert!(spec.is_required());
    }

    #[test]
    fn schema_spec_top_node_default() {
        let spec = SchemaSpec {
            nodes: indexmap::IndexMap::new(),
            marks: indexmap::IndexMap::new(),
            top_node: None,
        };
        assert!(spec.top_node.is_none());
    }

    #[test]
    fn whitespace_variants() {
        let _normal = Whitespace::Normal;
        let _pre = Whitespace::Pre;
    }

    #[test]
    fn exclude_spec_self_only() {
        let es = ExcludeSpec::SelfOnly;
        assert!(es.excludes("em", "em"));
        assert!(!es.excludes("em", "strong"));
    }

    #[test]
    fn exclude_spec_all() {
        let es = ExcludeSpec::All;
        assert!(es.excludes("em", "strong"));
        assert!(es.excludes("em", "em"));
    }

    #[test]
    fn exclude_spec_named() {
        let es = ExcludeSpec::Named(vec!["strong".to_string(), "code".to_string()]);
        assert!(es.excludes("em", "strong"));
        assert!(es.excludes("em", "code"));
        assert!(!es.excludes("em", "link"));
    }
}
```

- [ ] **Step 2: 在 mod.rs 声明模块**

```rust
// src/model/mod.rs
pub mod errors;
pub mod compare_deep;
pub mod render_output;
pub mod schema;
```

- [ ] **Step 3: 运行测试，确认失败**

```bash
cargo test -p md-editor model::schema::tests
```

Expected: FAIL（类型未定义）

- [ ] **Step 4: 实现 schema.rs（Spec 类型部分）**

```rust
// src/model/schema.rs
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use indexmap::IndexMap;
use serde_json::Value;

use crate::model::render_output::RenderOutput;

// NOTE: Node 和 Mark 在 Phase 3 实现，此处仅做类型前向声明占位。
// 渲染 hook 字段（to_render 等）使用函数指针类型，Phase 3 完成后类型自动对齐。

/// An object mapping attribute names to values.
///
/// Corresponds to `Attrs = {readonly [attr: string]: any}` in TypeScript.
pub type Attrs = Arc<HashMap<String, Value>>;

/// Whitespace handling mode for a node type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Whitespace {
    /// Collapse whitespace in normal mode (default).
    Normal,
    /// Preserve spaces inside the node.
    Pre,
}

/// Describes the exclude relationship for a mark type.
///
/// Replaces `excluded: MarkType[]` from TypeScript to avoid Arc circular references.
#[derive(Debug, Clone, PartialEq)]
pub enum ExcludeSpec {
    /// Only excludes marks of the same type (default behavior).
    SelfOnly,
    /// Excludes all marks in the schema (corresponds to `"_"`).
    All,
    /// Excludes marks with the listed names.
    Named(Vec<String>),
}

impl ExcludeSpec {
    /// Returns true if a mark with `self_name` excludes a mark with `other_name`.
    pub fn excludes(&self, self_name: &str, other_name: &str) -> bool {
        match self {
            ExcludeSpec::SelfOnly => self_name == other_name,
            ExcludeSpec::All => true,
            ExcludeSpec::Named(names) => names.iter().any(|n| n == other_name),
        }
    }
}

/// Validates attribute values. Used in AttrSpec.
pub enum AttrValidator {
    /// A `|`-separated list of allowed JSON primitive type names
    /// (e.g. `"string|null"`). Validated against the JSON value's type name.
    Types(Vec<String>),
    /// A custom validation function. Should return `Err(message)` on failure.
    Func(Box<dyn Fn(&Value) -> Result<(), String> + Send + Sync>),
}

impl fmt::Debug for AttrValidator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AttrValidator::Types(types) => write!(f, "Types({:?})", types),
            AttrValidator::Func(_) => write!(f, "Func(<fn>)"),
        }
    }
}

/// Used to define attributes on nodes or marks.
///
/// Corresponds to `AttributeSpec` in TypeScript.
#[derive(Debug)]
pub struct AttrSpec {
    /// The default value for this attribute. `None` means the attribute is required.
    pub default: Option<Value>,
    /// Optional validator for this attribute's value.
    pub validate: Option<AttrValidator>,
}

impl AttrSpec {
    /// Returns true if this attribute must be explicitly provided (no default).
    pub fn is_required(&self) -> bool {
        self.default.is_none()
    }

    /// Validates a value against this spec. Returns `Err` if validation fails.
    pub fn validate_value(&self, value: &Value) -> Result<(), String> {
        match &self.validate {
            None => Ok(()),
            Some(AttrValidator::Types(types)) => {
                let type_name = match value {
                    Value::Null => "null",
                    Value::Bool(_) => "boolean",
                    Value::Number(_) => "number",
                    Value::String(_) => "string",
                    Value::Array(_) => "array",
                    Value::Object(_) => "object",
                };
                if types.iter().any(|t| t == type_name) {
                    Ok(())
                } else {
                    Err(format!("Expected type {:?}, got {}", types, type_name))
                }
            }
            Some(AttrValidator::Func(f)) => f(value),
        }
    }
}

/// A description of a node type, used when defining a schema.
///
/// Corresponds to `NodeSpec` interface in TypeScript.
/// DOM-related fields (toDOM, parseDOM) are replaced by rendering hooks
/// for the cosmic-text layer. See design doc for rationale.
pub struct NodeSpec {
    /// The content expression for this node (e.g. `"block+"`, `"inline*"`).
    pub content: Option<String>,
    /// Space-separated mark names/groups allowed inside this node.
    /// `"_"` = all marks, `""` = no marks, `None` = default (all for inline content).
    pub marks: Option<String>,
    /// Space-separated group names this node belongs to (e.g. `"block"`).
    pub group: Option<String>,
    /// Should be set to true for inline nodes (implied for text nodes).
    pub inline: bool,
    /// True for nodes that are not leaf nodes but don't have directly editable content.
    pub atom: bool,
    /// Attribute specs for this node type.
    pub attrs: IndexMap<String, AttrSpec>,
    /// Whether this node can be selected as a node selection. Defaults to true.
    pub selectable: bool,
    /// Whether this node can be dragged without being selected. Defaults to false.
    pub draggable: bool,
    /// Whether this node contains code (affects some command behavior).
    pub code: bool,
    /// Whitespace handling override. Defaults to `Pre` when `code` is true.
    pub whitespace: Option<Whitespace>,
    /// When enabled, enables both `defining_as_context` and `defining_for_content`.
    pub defining: bool,
    /// Whether this node is a context-defining parent during replace operations.
    pub defining_as_context: bool,
    /// Whether this node is a content-defining wrapper during replace operations.
    pub defining_for_content: bool,
    /// Whether editing operations won't cross node boundaries.
    pub isolating: bool,
    /// Whether this inline leaf node acts as a linebreak replacement.
    pub linebreak_replacement: bool,
    /// Rendering hook (cosmic-text placeholder). Called to render a node.
    pub to_render: Option<Box<dyn Fn(&Value) -> RenderOutput + Send + Sync>>,
    /// Returns a plain-text string for leaf node content (used in textBetween).
    pub leaf_text: Option<Box<dyn Fn(&Value) -> String + Send + Sync>>,
    /// Returns a debug string representation of this node.
    pub to_debug_string: Option<Box<dyn Fn(&Value) -> String + Send + Sync>>,
}

impl Default for NodeSpec {
    fn default() -> Self {
        Self {
            content: None,
            marks: None,
            group: None,
            inline: false,
            atom: false,
            attrs: IndexMap::new(),
            selectable: true,
            draggable: false,
            code: false,
            whitespace: None,
            defining: false,
            defining_as_context: false,
            defining_for_content: false,
            isolating: false,
            linebreak_replacement: false,
            to_render: None,
            leaf_text: None,
            to_debug_string: None,
        }
    }
}

/// A description of a mark type, used when defining a schema.
///
/// Corresponds to `MarkSpec` interface in TypeScript.
pub struct MarkSpec {
    /// Attribute specs for this mark type.
    pub attrs: IndexMap<String, AttrSpec>,
    /// Whether this mark is active at the cursor when positioned at its end. Defaults to true.
    pub inclusive: bool,
    /// Space-separated mark names/groups this mark excludes.
    /// `"_"` = all, `""` = none, `None` = only excludes itself (default).
    pub excludes: Option<String>,
    /// Space-separated group names this mark belongs to.
    pub group: Option<String>,
    /// Whether this mark spans multiple adjacent nodes when serialized. Defaults to true.
    pub spanning: bool,
    /// Whether this mark's content is code.
    pub code: bool,
    /// Rendering hook (cosmic-text placeholder). Called to render a mark.
    pub to_render: Option<Box<dyn Fn(&Value, bool) -> RenderOutput + Send + Sync>>,
}

impl Default for MarkSpec {
    fn default() -> Self {
        Self {
            attrs: IndexMap::new(),
            inclusive: true,
            excludes: None,
            group: None,
            spanning: true,
            code: false,
            to_render: None,
        }
    }
}

/// An object describing a schema, as passed to the Schema constructor.
///
/// Corresponds to `SchemaSpec` interface in TypeScript.
pub struct SchemaSpec {
    /// The node types in this schema, in order.
    pub nodes: IndexMap<String, NodeSpec>,
    /// The mark types in this schema, in order.
    pub marks: IndexMap<String, MarkSpec>,
    /// The name of the default top-level node. Defaults to `"doc"`.
    pub top_node: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn node_spec_default() {
        let spec = NodeSpec::default();
        assert!(spec.content.is_none());
        assert!(spec.marks.is_none());
        assert!(spec.group.is_none());
        assert!(!spec.inline);
        assert!(!spec.atom);
        assert!(!spec.code);
        assert!(spec.selectable);
        assert!(!spec.draggable);
        assert!(!spec.isolating);
        assert!(!spec.defining);
        assert!(!spec.linebreak_replacement);
    }

    #[test]
    fn mark_spec_default() {
        let spec = MarkSpec::default();
        assert!(spec.inclusive);
        assert!(spec.spanning);
        assert!(!spec.code);
        assert!(spec.excludes.is_none());
        assert!(spec.group.is_none());
    }

    #[test]
    fn attr_spec_with_default() {
        let spec = AttrSpec { default: Some(json!("hello")), validate: None };
        assert!(!spec.is_required());
        assert_eq!(spec.default, Some(json!("hello")));
    }

    #[test]
    fn attr_spec_required() {
        let spec = AttrSpec { default: None, validate: None };
        assert!(spec.is_required());
    }

    #[test]
    fn attr_spec_validate_types_ok() {
        let spec = AttrSpec {
            default: None,
            validate: Some(AttrValidator::Types(vec!["string".to_string()])),
        };
        assert!(spec.validate_value(&json!("hello")).is_ok());
        assert!(spec.validate_value(&json!(42)).is_err());
    }

    #[test]
    fn attr_spec_validate_func() {
        let spec = AttrSpec {
            default: None,
            validate: Some(AttrValidator::Func(Box::new(|v| {
                if v.is_number() { Ok(()) } else { Err("not a number".to_string()) }
            }))),
        };
        assert!(spec.validate_value(&json!(42)).is_ok());
        assert!(spec.validate_value(&json!("x")).is_err());
    }

    #[test]
    fn schema_spec_top_node_default() {
        let spec = SchemaSpec {
            nodes: IndexMap::new(),
            marks: IndexMap::new(),
            top_node: None,
        };
        assert!(spec.top_node.is_none());
    }

    #[test]
    fn whitespace_variants() {
        let _normal = Whitespace::Normal;
        let _pre = Whitespace::Pre;
    }

    #[test]
    fn exclude_spec_self_only() {
        let es = ExcludeSpec::SelfOnly;
        assert!(es.excludes("em", "em"));
        assert!(!es.excludes("em", "strong"));
    }

    #[test]
    fn exclude_spec_all() {
        let es = ExcludeSpec::All;
        assert!(es.excludes("em", "strong"));
        assert!(es.excludes("em", "em"));
    }

    #[test]
    fn exclude_spec_named() {
        let es = ExcludeSpec::Named(vec!["strong".to_string(), "code".to_string()]);
        assert!(es.excludes("em", "strong"));
        assert!(es.excludes("em", "code"));
        assert!(!es.excludes("em", "link"));
    }
}
```

- [ ] **Step 5: 运行测试，确认通过**

```bash
cargo test -p md-editor model::schema::tests
```

Expected:
```
running 11 tests
test model::schema::tests::node_spec_default ... ok
test model::schema::tests::mark_spec_default ... ok
test model::schema::tests::attr_spec_with_default ... ok
test model::schema::tests::attr_spec_required ... ok
test model::schema::tests::attr_spec_validate_types_ok ... ok
test model::schema::tests::attr_spec_validate_func ... ok
test model::schema::tests::schema_spec_top_node_default ... ok
test model::schema::tests::whitespace_variants ... ok
test model::schema::tests::exclude_spec_self_only ... ok
test model::schema::tests::exclude_spec_all ... ok
test model::schema::tests::exclude_spec_named ... ok
test result: ok. 11 passed; 0 failed
```

- [ ] **Step 6: Commit**

```bash
git add src/model/schema.rs src/model/mod.rs
git commit -m "feat(model): add spec types NodeSpec, MarkSpec, AttrSpec, SchemaSpec, ExcludeSpec, Whitespace"
```

---

## Task 6: 全量验证

- [ ] **Step 1: 运行所有测试**

```bash
cargo test -p md-editor
```

Expected:
```
running 24 tests
test model::errors::tests::* ... ok (5 tests)
test model::compare_deep::tests::* ... ok (8 tests)
test model::schema::tests::* ... ok (11 tests)
test result: ok. 24 passed; 0 failed
```

- [ ] **Step 2: 检查无警告**

```bash
cargo clippy -p md-editor -- -D warnings
```

Expected: 无 warnings 或 errors

- [ ] **Step 3: 确认 Phase 1 完成，更新阶段计划**

将 `docs/superpowers/plans/model-phases.md` 中 Phase 1 状态从 `⬜ 待开始` 改为 `✅ 完成`。

```bash
git add docs/superpowers/plans/model-phases.md
git commit -m "docs: mark model Phase 1 as complete"
```
