# Schema + Node 方法完善 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 建立 Schema 中央类型注册表，恢复 ptr_eq 比较语义，实现 Node::slice/replace，补全剩余 Node 方法。

**Architecture:** 三个独立 Phase：(1) Schema::new 两遍遍历建立类型注册表，恢复所有比较语义；(2) Node::slice 和 Node::replace 复用已有 replace.rs 算法；(3) 补全 node_at/text_content/nodes_between/check 及改进 create_and_fill 递归填充。

**Tech Stack:** Rust, std::sync::Arc, std::collections::HashMap

**Spec:** `docs/superpowers/specs/2026-04-05-prosemirror-schema-node-design.md`

---

## 文件清单

| 文件 | 操作 | 职责 |
|------|------|------|
| `src/model/schema.rs` | 修改 | 新增 SchemaSpec/NodeSpec/MarkSpec/Schema/SchemaError；恢复 ptr_eq |
| `src/model/content.rs` | 修改 | match_type/compatible 改用 ptr_eq |
| `src/model/mark.rs` | 修改 | Mark::eq type 部分改用 ptr_eq |
| `src/model/node.rs` | 修改 | 新增 slice/replace/node_at/text_content/nodes_between/check；改进 create_and_fill |
| `src/model/tests_ts.rs` | 修改 | 新增 test-replace 和 test-slice 移植测试 |

---

## Task 1: SchemaSpec / NodeSpec / MarkSpec 数据类型

**Files:**
- Modify: `src/model/schema.rs`

- [ ] **Step 1: 在 schema.rs 顶部新增以下类型定义**

在 `use` 语句之后、`NodeType` 定义之前插入：

```rust
/// Schema 构建错误
#[derive(Debug, Clone)]
pub enum SchemaError {
    EmptyNodes,
    ContentParseError(String),
    UnknownMarkRef(String),
}

impl std::fmt::Display for SchemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaError::EmptyNodes => write!(f, "Schema must have at least one node type"),
            SchemaError::ContentParseError(e) => write!(f, "Content parse error: {}", e),
            SchemaError::UnknownMarkRef(n) => write!(f, "Unknown mark type: {}", n),
        }
    }
}

/// 节点类型构建描述
#[derive(Debug, Clone, Default)]
pub struct NodeSpec {
    /// content 表达式，如 "paragraph+" 或 "inline*"；None 表示叶节点
    pub content: Option<String>,
    /// 允许的 mark："_"=全部，""=无，空格分隔名称列表
    pub marks: Option<String>,
    /// 所属分组，空格分隔，如 "block" 或 "block inline"
    pub group: Option<String>,
    /// 是否行内节点（默认 false = 块级）
    pub inline: bool,
    /// 是否文本节点
    pub is_text: bool,
}

/// 标记类型构建描述
#[derive(Debug, Clone, Default)]
pub struct MarkSpec {
    /// 排序优先级（越小越靠前）
    pub rank: usize,
    /// 排斥的 mark："_"=排斥所有，""=仅排斥自身同类，空格分隔名称列表
    pub excludes: Option<String>,
    /// 光标到边界时是否延伸；None = 默认 true
    pub inclusive: Option<bool>,
}

/// Schema 构建规格
pub struct SchemaSpec {
    /// 有序节点列表；第一个为 topNode
    pub nodes: Vec<(String, NodeSpec)>,
    pub marks: Vec<(String, MarkSpec)>,
}
```

- [ ] **Step 2: 确认编译通过**

```bash
cd /Users/aporicho/Desktop/md-editor && cargo check 2>&1 | head -20
```

期望：零 error（可能有 unused 警告，暂时忽略）

- [ ] **Step 3: Commit**

```bash
git add src/model/schema.rs
git commit -m "feat(schema): add SchemaSpec/NodeSpec/MarkSpec/SchemaError types"
```

---

## Task 2: Schema 结构体与 Schema::new 两遍遍历

**Files:**
- Modify: `src/model/schema.rs`
- Modify: `src/model/mod.rs`（pub use Schema）

- [ ] **Step 1: 在 schema.rs 末尾添加 Schema 结构体和 new 方法**

在文件末尾（现有 `#[cfg(test)]` 块之前）插入：

```rust
/// 中央类型注册表。所有 NodeType / MarkType 均通过此处统一创建。
pub struct Schema {
    pub nodes: HashMap<String, Arc<NodeType>>,
    pub marks: HashMap<String, Arc<MarkType>>,
    /// 文档根节点类型（spec.nodes 第一个）
    pub top_node_type: Arc<NodeType>,
}

impl Schema {
    pub fn new(spec: SchemaSpec) -> Result<Arc<Self>, SchemaError> {
        if spec.nodes.is_empty() {
            return Err(SchemaError::EmptyNodes);
        }

        // ── 第一遍：注册所有 MarkType ────────────────────────────
        // excluded 先留空，第二步填充
        let mut mark_types: HashMap<String, Arc<MarkType>> = HashMap::new();
        for (name, ms) in &spec.marks {
            mark_types.insert(name.clone(), Arc::new(MarkType {
                name: name.clone(),
                rank: ms.rank,
                excluded: vec![],
                inclusive: ms.inclusive,
            }));
        }

        // 填充 excluded（需要所有 MarkType 都存在后才能解析）
        for (name, ms) in &spec.marks {
            let excluded = Self::resolve_excluded(name, ms, &mark_types)?;
            // 此时每个 Arc<MarkType> 只被 mark_types 持有，get_mut 安全
            let mt = mark_types.get_mut(name).unwrap();
            Arc::get_mut(mt).unwrap().excluded = excluded;
        }

        // ── 第一遍：注册所有 NodeType（content_match = None）────
        let mut node_types: HashMap<String, Arc<NodeType>> = HashMap::new();
        for (name, ns) in &spec.nodes {
            node_types.insert(name.clone(), Arc::new(NodeType {
                name: name.clone(),
                groups: ns.group.as_deref()
                    .map(|g| g.split_whitespace().map(String::from).collect())
                    .unwrap_or_default(),
                is_block: !ns.inline,
                is_text: ns.is_text,
                inline_content: false,   // 第二遍填充
                mark_set: None,          // 第二遍填充
                content_match: None,     // 第二遍填充
            }));
        }

        // ── 第二遍：填充 content_match / inline_content / mark_set
        for (name, ns) in &spec.nodes {
            let nt = node_types.get_mut(name).unwrap();
            let nt_mut = Arc::get_mut(nt).unwrap();

            // content_match
            if let Some(ref expr) = ns.content {
                let cm = ContentMatch::parse(expr, &node_types)
                    .map_err(SchemaError::ContentParseError)?;
                nt_mut.inline_content = cm.inline_content();
                nt_mut.content_match = Some(cm);
            }

            // mark_set
            nt_mut.mark_set = Self::resolve_mark_set(&ns.marks, &mark_types)?;
        }

        let top_node_type = Arc::clone(node_types.values().next().unwrap());

        Ok(Arc::new(Schema { nodes: node_types, marks: mark_types, top_node_type }))
    }

    /// 解析 NodeSpec.marks 字段为 Option<Vec<Arc<MarkType>>>
    fn resolve_mark_set(
        marks: &Option<String>,
        mark_types: &HashMap<String, Arc<MarkType>>,
    ) -> Result<Option<Vec<Arc<MarkType>>>, SchemaError> {
        match marks.as_deref() {
            None | Some("_") => Ok(None), // None 或 "_" = 全部允许
            Some("") => Ok(Some(vec![])), // "" = 不允许任何 mark
            Some(s) => {
                let set = s.split_whitespace()
                    .map(|name| {
                        mark_types.get(name)
                            .map(Arc::clone)
                            .ok_or_else(|| SchemaError::UnknownMarkRef(name.to_string()))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Some(set))
            }
        }
    }

    /// 解析 MarkSpec.excludes 字段为 Vec<Arc<MarkType>>
    fn resolve_excluded(
        self_name: &str,
        ms: &MarkSpec,
        mark_types: &HashMap<String, Arc<MarkType>>,
    ) -> Result<Vec<Arc<MarkType>>, SchemaError> {
        match ms.excludes.as_deref() {
            None | Some("") => {
                // 默认排斥自身
                Ok(mark_types.get(self_name).map(|m| vec![Arc::clone(m)]).unwrap_or_default())
            }
            Some("_") => {
                // 排斥所有 mark
                Ok(mark_types.values().map(Arc::clone).collect())
            }
            Some(s) => {
                s.split_whitespace()
                    .map(|name| {
                        mark_types.get(name)
                            .map(Arc::clone)
                            .ok_or_else(|| SchemaError::UnknownMarkRef(name.to_string()))
                    })
                    .collect()
            }
        }
    }
}
```

在 `schema.rs` 顶部的 `use` 语句中补充：

```rust
use std::collections::HashMap;
```

- [ ] **Step 2: 在 mod.rs 中 pub use Schema**

在 `src/model/mod.rs` 末尾添加：

```rust
pub use schema::{Schema, SchemaSpec, NodeSpec, MarkSpec, SchemaError};
```

- [ ] **Step 3: 编写 Schema::new 基本测试**

在 `src/model/schema.rs` 的 `#[cfg(test)]` 块中添加：

```rust
#[test]
fn schema_new_basic() {
    let spec = SchemaSpec {
        nodes: vec![
            ("doc".into(), NodeSpec { content: Some("paragraph+".into()), ..Default::default() }),
            ("paragraph".into(), NodeSpec {
                content: Some("text*".into()),
                group: Some("block".into()),
                inline: false,
                ..Default::default()
            }),
            ("text".into(), NodeSpec { inline: true, is_text: true, ..Default::default() }),
        ],
        marks: vec![
            ("bold".into(), MarkSpec { rank: 0, ..Default::default() }),
        ],
    };
    let schema = Schema::new(spec).expect("schema build should succeed");
    assert!(schema.nodes.contains_key("doc"));
    assert!(schema.nodes.contains_key("paragraph"));
    assert!(schema.nodes.contains_key("text"));
    assert!(schema.marks.contains_key("bold"));
    assert_eq!(schema.top_node_type.name, "doc");
}

#[test]
fn schema_new_empty_nodes_fails() {
    let spec = SchemaSpec { nodes: vec![], marks: vec![] };
    assert!(Schema::new(spec).is_err());
}

#[test]
fn schema_same_type_ptr_eq() {
    let spec = SchemaSpec {
        nodes: vec![
            ("doc".into(), NodeSpec { content: Some("p+".into()), ..Default::default() }),
            ("p".into(), NodeSpec { group: Some("block".into()), ..Default::default() }),
        ],
        marks: vec![],
    };
    let schema = Schema::new(spec).unwrap();
    let p1 = schema.nodes.get("p").unwrap();
    let p2 = schema.nodes.get("p").unwrap();
    assert!(Arc::ptr_eq(p1, p2), "同名类型应是同一 Arc 实例");
}
```

- [ ] **Step 4: 运行测试**

```bash
cd /Users/aporicho/Desktop/md-editor && cargo test schema_new 2>&1
```

期望：3 个测试全部 PASS

- [ ] **Step 5: Commit**

```bash
git add src/model/schema.rs src/model/mod.rs
git commit -m "feat(schema): add Schema struct with two-pass build and ptr_eq guarantees"
```

---

## Task 3: 恢复 ptr_eq 比较语义

**Files:**
- Modify: `src/model/content.rs`
- Modify: `src/model/mark.rs`

- [ ] **Step 1: content.rs — match_type 改用 ptr_eq**

在 `content.rs` 中找到 `match_type` 方法，将：

```rust
if edge.node_type.name == node_type.name {
```

改为：

```rust
if Arc::ptr_eq(&edge.node_type, node_type) {
```

- [ ] **Step 2: content.rs — compatible 改用 ptr_eq**

找到 `compatible` 方法，将：

```rust
if a.node_type.name == b.node_type.name {
```

改为：

```rust
if Arc::ptr_eq(&a.node_type, &b.node_type) {
```

- [ ] **Step 3: mark.rs — Mark::eq 改用 ptr_eq**

找到 `Mark::eq` 方法，将：

```rust
self.mark_type.name == other.mark_type.name && self.attrs == other.attrs
```

改为：

```rust
Arc::ptr_eq(&self.mark_type, &other.mark_type) && self.attrs == other.attrs
```

- [ ] **Step 4: schema.rs — MarkType::excludes 改用 ptr_eq**

找到 `MarkType::excludes` 方法，将：

```rust
self.excluded.iter().any(|e| e.name == other.name)
```

改为：

```rust
self.excluded.iter().any(|e| Arc::ptr_eq(e, other))
```

注意：`excludes` 的参数类型需改为 `&Arc<MarkType>`：

```rust
pub fn excludes(&self, other: &Arc<MarkType>) -> bool {
    self.excluded.iter().any(|e| Arc::ptr_eq(e, other))
}
```

相应地，`mark.rs` 中 `add_to_set` 调用处改为传 `&other.mark_type`：

```rust
if self.mark_type.excludes(&other.mark_type) {
```

改为：

```rust
if self.mark_type.excludes(&Arc::clone(&other.mark_type)) {
// 或者直接改 excludes 参数为 Arc<MarkType>（两种都可以，保持一致即可）
```

实际上最简洁的做法是让 `excludes` 接收 `&Arc<MarkType>`，调用处已经持有 `Arc<MarkType>`，直接传引用：

```rust
// mark.rs add_to_set 中：
if self.mark_type.excludes(&other.mark_type) {   // other.mark_type 是 Arc<MarkType>
```

```rust
// schema.rs MarkType::excludes：
pub fn excludes(&self, other: &Arc<MarkType>) -> bool {
    self.excluded.iter().any(|e| Arc::ptr_eq(e, other))
}
```

- [ ] **Step 5: 运行全量测试，确认无回归**

```bash
cd /Users/aporicho/Desktop/md-editor && cargo test 2>&1
```

期望：所有已有测试 PASS（注意：ptr_eq 只在 Schema 构建的实例上保证；现有测试用 make_nt/make_mt 直接构造的 Arc 仍用名称比较会失败——检查是否有测试依赖名称比较并更新它们）

- [ ] **Step 6: Commit**

```bash
git add src/model/content.rs src/model/mark.rs src/model/schema.rs
git commit -m "refactor(schema): restore ptr_eq comparison semantics via Schema singleton"
```

---

## Task 4: Node::slice()

**Files:**
- Modify: `src/model/node.rs`

- [ ] **Step 1: 在 node.rs 中引入依赖**

在 `node.rs` 顶部 `use` 区域补充：

```rust
use super::replace::Slice;
use super::resolvedpos::ResolvedPos;
```

- [ ] **Step 2: 编写失败测试**

在 `src/model/node.rs` 的 `#[cfg(test)]` 块中添加：

```rust
#[test]
fn node_slice_basic() {
    // doc → paragraph("hello")
    // pos 布局：0[1 h e l l o 6]7
    // slice(1, 6) 应返回 open_start=1, open_end=1, content=[text "hello"]
    let text_nt = Arc::new(NodeType {
        name: "text".into(), groups: vec![], is_block: false,
        is_text: true, inline_content: false, mark_set: None, content_match: None,
    });
    let para_nt = Arc::new(NodeType {
        name: "paragraph".into(), groups: vec![], is_block: true,
        is_text: false, inline_content: true, mark_set: None, content_match: None,
    });
    let doc_nt = Arc::new(NodeType {
        name: "doc".into(), groups: vec![], is_block: true,
        is_text: false, inline_content: false, mark_set: None, content_match: None,
    });
    let txt = Node { node_type: Arc::clone(&text_nt), attrs: std::collections::BTreeMap::new(),
                     content: super::fragment::Fragment::empty(), marks: vec![], text: Some("hello".into()) };
    let para = Node { node_type: Arc::clone(&para_nt), attrs: std::collections::BTreeMap::new(),
                      content: super::fragment::Fragment::from_array(vec![txt]), marks: vec![], text: None };
    let doc = Node { node_type: Arc::clone(&doc_nt), attrs: std::collections::BTreeMap::new(),
                     content: super::fragment::Fragment::from_array(vec![para]), marks: vec![], text: None };

    let slice = doc.slice(1, 6).unwrap();
    assert_eq!(slice.open_start, 1);
    assert_eq!(slice.open_end, 1);
    assert_eq!(slice.content.child_count(), 1); // 一个 paragraph
}

#[test]
fn node_slice_empty_range() {
    let doc_nt = Arc::new(NodeType {
        name: "doc".into(), groups: vec![], is_block: true,
        is_text: false, inline_content: false, mark_set: None, content_match: None,
    });
    let doc = Node { node_type: doc_nt, attrs: std::collections::BTreeMap::new(),
                     content: super::fragment::Fragment::empty(), marks: vec![], text: None };
    let slice = doc.slice(0, 0).unwrap();
    assert_eq!(slice.size(), 0);
}
```

- [ ] **Step 3: 运行测试，确认 FAIL（方法未定义）**

```bash
cd /Users/aporicho/Desktop/md-editor && cargo test node_slice 2>&1 | head -20
```

期望：编译错误 "no method named `slice`"

- [ ] **Step 4: 实现 Node::slice**

在 `node.rs` 的 `impl Node` 块中添加：

```rust
/// 从 from 到 to 位置剪出一个 Slice。
pub fn slice(&self, from: usize, to: usize) -> Result<Slice, String> {
    if from > to || to > self.content.size {
        return Err(format!("slice({}, {}) out of range (size={})", from, to, self.content.size));
    }
    if from == to {
        return Ok(Slice::empty());
    }
    let from_pos = ResolvedPos::resolve(self, from)?;
    let to_pos = ResolvedPos::resolve(self, to)?;
    let depth = from_pos.shared_depth(to);
    let start = from_pos.start(Some(depth as isize));
    let node = from_pos.node(Some(depth as isize));
    let content = node.content.cut(from - start, to - start);
    Ok(Slice::new(content, from_pos.depth() - depth, to_pos.depth() - depth))
}
```

- [ ] **Step 5: 运行测试，确认 PASS**

```bash
cd /Users/aporicho/Desktop/md-editor && cargo test node_slice 2>&1
```

期望：2 个测试 PASS

- [ ] **Step 6: Commit**

```bash
git add src/model/node.rs
git commit -m "feat(node): implement Node::slice"
```

---

## Task 5: Node::replace()

**Files:**
- Modify: `src/model/node.rs`

- [ ] **Step 1: 在 node.rs 中引入 replace 函数**

在 `node.rs` 顶部 `use` 区域补充：

```rust
use super::replace::{replace as do_replace, ReplaceError, Slice};
```

（如果 Task 4 已引入 `Slice`，只需追加 `replace as do_replace` 和 `ReplaceError`）

- [ ] **Step 2: 编写失败测试**

注意：replace 算法内部调用 `close()` → `check_content()`，NodeType 必须有正确的 content_match，否则 close 会报内容非法。用 ContentMatch::parse 构造带约束的类型。

```rust
#[test]
fn node_replace_delete() {
    // doc → paragraph("hello")，删除 pos 2..4（"el"）
    // 期望：paragraph("hlo")
    use super::replace::Slice;
    use super::content::ContentMatch;
    use std::collections::HashMap;

    // 构造带 content_match 的类型，close() 才能通过 check_content
    let text_nt = Arc::new(NodeType {
        name: "text".into(), groups: vec![], is_block: false,
        is_text: true, inline_content: false, mark_set: None, content_match: None,
    });
    let mut types: HashMap<String, Arc<NodeType>> = HashMap::new();
    types.insert("text".into(), Arc::clone(&text_nt));

    let cm_para = ContentMatch::parse("text*", &types).unwrap();
    let para_nt = Arc::new(NodeType {
        name: "paragraph".into(), groups: vec![], is_block: true,
        is_text: false, inline_content: true, mark_set: None,
        content_match: Some(cm_para),
    });
    types.insert("paragraph".into(), Arc::clone(&para_nt));

    let cm_doc = ContentMatch::parse("paragraph+", &types).unwrap();
    let doc_nt = Arc::new(NodeType {
        name: "doc".into(), groups: vec![], is_block: true,
        is_text: false, inline_content: false, mark_set: None,
        content_match: Some(cm_doc),
    });

    let txt = Node { node_type: Arc::clone(&text_nt), attrs: std::collections::BTreeMap::new(),
                     content: super::fragment::Fragment::empty(), marks: vec![], text: Some("hello".into()) };
    let para = Node { node_type: Arc::clone(&para_nt), attrs: std::collections::BTreeMap::new(),
                      content: super::fragment::Fragment::from_array(vec![txt]), marks: vec![], text: None };
    let doc = Node { node_type: Arc::clone(&doc_nt), attrs: std::collections::BTreeMap::new(),
                     content: super::fragment::Fragment::from_array(vec![para]), marks: vec![], text: None };

    let new_doc = doc.replace(2, 4, &Slice::empty()).unwrap();
    let new_para = new_doc.content.child(0);
    assert_eq!(new_para.content.child(0).text(), Some("hlo"));
}
```

- [ ] **Step 3: 运行测试，确认 FAIL**

```bash
cd /Users/aporicho/Desktop/md-editor && cargo test node_replace_delete 2>&1 | head -10
```

期望：编译错误 "no method named `replace`"

- [ ] **Step 4: 实现 Node::replace**

在 `node.rs` 的 `impl Node` 块中添加：

```rust
/// 用 slice 替换 from..to，返回新文档（原节点不变）。
pub fn replace(&self, from: usize, to: usize, slice: &Slice) -> Result<Node, ReplaceError> {
    let from_pos = ResolvedPos::resolve(self, from).map_err(ReplaceError)?;
    let to_pos   = ResolvedPos::resolve(self, to).map_err(ReplaceError)?;
    do_replace(&from_pos, &to_pos, slice)
}
```

- [ ] **Step 5: 运行测试，确认 PASS**

```bash
cd /Users/aporicho/Desktop/md-editor && cargo test node_replace 2>&1
```

期望：PASS

- [ ] **Step 6: Commit**

```bash
git add src/model/node.rs
git commit -m "feat(node): implement Node::replace"
```

---

## Task 6: 移植 test-replace.ts 和 test-slice.ts 测试

**Files:**
- Modify: `src/model/tests_ts.rs`

- [ ] **Step 1: 在 tests_ts.rs 末尾新增 replace/slice 测试辅助函数**

注意：replace 内部调用 `close()` → `check_content()`，所有节点类型必须有正确的 content_match；否则非空内容会报验证失败。用 ContentMatch::parse 构建带约束的测试 schema。

```rust
// ═════════════════════════════════════════════
//  test-replace.ts / test-slice.ts 移植
// ═════════════════════════════════════════════

/// 构造带 content_match 的 schema，用于 replace/slice 测试。
/// replace 内部的 close() 会调用 check_content，必须有正确约束才能通过。
fn make_replace_schema() -> (Arc<NodeType>, Arc<NodeType>, Arc<NodeType>) {
    // text: 叶节点，content_match=None
    let text_nt = Arc::new(NodeType {
        name: "text".into(), groups: vec![], is_block: false,
        is_text: true, inline_content: true, mark_set: None, content_match: None,
    });
    let mut types: HashMap<String, Arc<NodeType>> = HashMap::new();
    types.insert("text".into(), Arc::clone(&text_nt));

    // paragraph: 允许 text*
    let cm_para = super::content::ContentMatch::parse("text*", &types).unwrap();
    let para_nt = Arc::new(NodeType {
        name: "paragraph".into(), groups: vec![], is_block: true,
        is_text: false, inline_content: true, mark_set: None,
        content_match: Some(cm_para),
    });
    types.insert("paragraph".into(), Arc::clone(&para_nt));

    // doc: 允许 paragraph+
    let cm_doc = super::content::ContentMatch::parse("paragraph+", &types).unwrap();
    let doc_nt = Arc::new(NodeType {
        name: "doc".into(), groups: vec![], is_block: true,
        is_text: false, inline_content: false, mark_set: None,
        content_match: Some(cm_doc),
    });

    (doc_nt, para_nt, text_nt)
}

fn make_simple_doc_r() -> Node {
    // doc → paragraph("hello")
    // pos 布局：0[1 h e l l o 6]7
    let (doc_nt, para_nt, text_nt) = make_replace_schema();
    let txt = Node { node_type: Arc::clone(&text_nt), attrs: BTreeMap::new(),
                     content: Fragment::empty(), marks: vec![], text: Some("hello".into()) };
    let para = Node { node_type: Arc::clone(&para_nt), attrs: BTreeMap::new(),
                      content: Fragment::from_array(vec![txt]), marks: vec![], text: None };
    Node { node_type: doc_nt, attrs: BTreeMap::new(),
           content: Fragment::from_array(vec![para]), marks: vec![], text: None }
}

fn make_two_para_doc_r() -> Node {
    // doc → paragraph("foo") paragraph("bar")
    // pos 布局：0[1 f o o 4]5[6 b a r 9]10
    let (doc_nt, para_nt, text_nt) = make_replace_schema();
    let mk_para = |s: &str| -> Node {
        let t = Node { node_type: Arc::clone(&text_nt), attrs: BTreeMap::new(),
                       content: Fragment::empty(), marks: vec![], text: Some(s.into()) };
        Node { node_type: Arc::clone(&para_nt), attrs: BTreeMap::new(),
               content: Fragment::from_array(vec![t]), marks: vec![], text: None }
    };
    Node { node_type: doc_nt, attrs: BTreeMap::new(),
           content: Fragment::from_array(vec![mk_para("foo"), mk_para("bar")]),
           marks: vec![], text: None }
}
```

- [ ] **Step 2: 新增 slice 测试**

slice 不调用 close()，可以继续用 test_schema()（content_match=None）。

```rust
#[test]
fn slice_whole_doc() {
    let s = test_schema();
    // doc → p("hello")，content.size = 7
    let d = doc(&s, vec![p(&s, vec![txt(&s, "hello")])]);
    let sl = d.slice(0, d.content.size).unwrap();
    assert_eq!(sl.open_start, 0);
    assert_eq!(sl.open_end, 0);
}

#[test]
fn slice_inside_paragraph() {
    let s = test_schema();
    let d = doc(&s, vec![p(&s, vec![txt(&s, "hello")])]);
    // slice(1, 6)：进入 paragraph 内部
    // pos 1 = 开始（depth=1），pos 6 = 结束（depth=1）
    let sl = d.slice(1, 6).unwrap();
    assert_eq!(sl.open_start, 1);
    assert_eq!(sl.open_end, 1);
    assert_eq!(sl.content.child_count(), 1);
    assert_eq!(sl.content.child(0).node_type.name, "paragraph");
}

#[test]
fn slice_text_only() {
    let s = test_schema();
    let d = doc(&s, vec![p(&s, vec![txt(&s, "hello")])]);
    // slice(2, 5)：从 "h" 之后到 "o" 之前，截取 "ell"（depth=2）
    let sl = d.slice(2, 5).unwrap();
    assert_eq!(sl.open_start, 2);
    assert_eq!(sl.open_end, 2);
}
```

- [ ] **Step 3: 新增 replace 测试**

replace 调用 close() → check_content，必须用 make_simple_doc_r() / make_two_para_doc_r()（带正确 content_match）。

```rust
#[test]
fn replace_delete_text() {
    // doc → p("hello")，删除 "ell"（pos 2..5）→ p("ho")
    let d = make_simple_doc_r();
    let result = d.replace(2, 5, &Slice::empty()).unwrap();
    let text = result.content.child(0).content.child(0).text().unwrap();
    assert_eq!(text, "ho");
}

#[test]
fn replace_insert_text() {
    // doc → p("hello")，在 pos 3 插入 "XY" → p("helXYlo")
    let (_, _, text_nt) = make_replace_schema();
    let d = make_simple_doc_r();
    let insert_node = Node {
        node_type: Arc::clone(&text_nt),
        attrs: BTreeMap::new(),
        content: Fragment::empty(),
        marks: vec![],
        text: Some("XY".into()),
    };
    let insert_slice = Slice::new(Fragment::from_array(vec![insert_node]), 0, 0);
    let result = d.replace(3, 3, &insert_slice).unwrap();
    let text = result.content.child(0).content.child(0).text().unwrap();
    assert_eq!(text, "helXYlo");
}

#[test]
fn replace_cross_paragraph() {
    // doc → p("foo") p("bar")
    // pos 布局：0[1 f o o 4]5[6 b a r 9]10
    // 删除 pos 3..8（从 p("foo") 内 "fo|o" 到 p("bar") 内 "ba|r"）
    // replace 将两段落合并 → p("for")（"fo" + "r"）
    let d = make_two_para_doc_r();
    let result = d.replace(3, 8, &Slice::empty()).unwrap();
    assert_eq!(result.content.child_count(), 1);
    let text = result.content.child(0).content.child(0).text().unwrap();
    assert_eq!(text, "for");
}
```

- [ ] **Step 4: 运行全部 tests_ts 测试**

```bash
cd /Users/aporicho/Desktop/md-editor && cargo test --test-output immediate 2>&1 | tail -30
```

期望：所有已有测试 + 新增 slice/replace 测试全部 PASS

- [ ] **Step 5: Commit**

```bash
git add src/model/tests_ts.rs
git commit -m "test: port test-replace.ts and test-slice.ts to Rust"
```

---

## Task 7: Node 辅助方法（node_at / text_content / nodes_between / check）

**Files:**
- Modify: `src/model/node.rs`

- [ ] **Step 1: 实现并测试 node_at**

先写失败测试（在 node.rs `#[cfg(test)]` 块）：

```rust
#[test]
fn node_node_at() {
    let text_nt = Arc::new(NodeType {
        name: "text".into(), groups: vec![], is_block: false,
        is_text: true, inline_content: false, mark_set: None, content_match: None,
    });
    let para_nt = Arc::new(NodeType {
        name: "paragraph".into(), groups: vec![], is_block: true,
        is_text: false, inline_content: true, mark_set: None, content_match: None,
    });
    let doc_nt = Arc::new(NodeType {
        name: "doc".into(), groups: vec![], is_block: true,
        is_text: false, inline_content: false, mark_set: None, content_match: None,
    });
    let txt = Node { node_type: Arc::clone(&text_nt), attrs: std::collections::BTreeMap::new(),
                     content: super::fragment::Fragment::empty(), marks: vec![], text: Some("hi".into()) };
    let para = Node { node_type: Arc::clone(&para_nt), attrs: std::collections::BTreeMap::new(),
                      content: super::fragment::Fragment::from_array(vec![txt]), marks: vec![], text: None };
    let doc = Node { node_type: Arc::clone(&doc_nt), attrs: std::collections::BTreeMap::new(),
                     content: super::fragment::Fragment::from_array(vec![para]), marks: vec![], text: None };
    // pos=0: paragraph 节点之前的位置，node_at(0) = paragraph
    assert_eq!(doc.node_at(0).map(|n| n.node_type.name.as_str()), Some("paragraph"));
    // pos=1: 进入 paragraph 内部，node_at(1) = text "hi"
    assert_eq!(doc.node_at(1).map(|n| n.node_type.name.as_str()), Some("text"));
}
```

运行确认 FAIL，然后实现：

```rust
/// 返回绝对位置 pos 处的节点（从当前节点的内容开始计算）。
pub fn node_at(&self, mut pos: usize) -> Option<&Node> {
    let mut node = self;
    loop {
        let (index, offset) = node.content.find_index(pos);
        let child = node.content.maybe_child(index)?;
        if offset == pos || child.is_text() {
            return Some(child);
        }
        pos -= offset + 1;
        node = child;
    }
}
```

- [ ] **Step 2: 实现并测试 text_content**

失败测试：

```rust
#[test]
fn node_text_content() {
    let text_nt = Arc::new(NodeType {
        name: "text".into(), groups: vec![], is_block: false,
        is_text: true, inline_content: false, mark_set: None, content_match: None,
    });
    let para_nt = Arc::new(NodeType {
        name: "paragraph".into(), groups: vec![], is_block: true,
        is_text: false, inline_content: true, mark_set: None, content_match: None,
    });
    let doc_nt = Arc::new(NodeType {
        name: "doc".into(), groups: vec![], is_block: true,
        is_text: false, inline_content: false, mark_set: None, content_match: None,
    });
    let t1 = Node { node_type: Arc::clone(&text_nt), attrs: std::collections::BTreeMap::new(),
                    content: super::fragment::Fragment::empty(), marks: vec![], text: Some("hello".into()) };
    let t2 = Node { node_type: Arc::clone(&text_nt), attrs: std::collections::BTreeMap::new(),
                    content: super::fragment::Fragment::empty(), marks: vec![], text: Some(" world".into()) };
    let para = Node { node_type: Arc::clone(&para_nt), attrs: std::collections::BTreeMap::new(),
                      content: super::fragment::Fragment::from_array(vec![t1, t2]), marks: vec![], text: None };
    let doc = Node { node_type: Arc::clone(&doc_nt), attrs: std::collections::BTreeMap::new(),
                     content: super::fragment::Fragment::from_array(vec![para]), marks: vec![], text: None };
    assert_eq!(doc.text_content(), "hello world");
}
```

实现：

```rust
/// 把整棵子树的文本内容拼成字符串。
pub fn text_content(&self) -> String {
    if let Some(ref t) = self.text {
        return t.clone();
    }
    let mut result = String::new();
    for i in 0..self.content.child_count() {
        result.push_str(&self.content.child(i).text_content());
    }
    result
}
```

- [ ] **Step 3: 实现并测试 nodes_between**

失败测试：

```rust
#[test]
fn node_nodes_between() {
    let text_nt = Arc::new(NodeType {
        name: "text".into(), groups: vec![], is_block: false,
        is_text: true, inline_content: false, mark_set: None, content_match: None,
    });
    let para_nt = Arc::new(NodeType {
        name: "paragraph".into(), groups: vec![], is_block: true,
        is_text: false, inline_content: true, mark_set: None, content_match: None,
    });
    let doc_nt = Arc::new(NodeType {
        name: "doc".into(), groups: vec![], is_block: true,
        is_text: false, inline_content: false, mark_set: None, content_match: None,
    });
    let txt = Node { node_type: Arc::clone(&text_nt), attrs: std::collections::BTreeMap::new(),
                     content: super::fragment::Fragment::empty(), marks: vec![], text: Some("hi".into()) };
    let para = Node { node_type: Arc::clone(&para_nt), attrs: std::collections::BTreeMap::new(),
                      content: super::fragment::Fragment::from_array(vec![txt]), marks: vec![], text: None };
    let doc = Node { node_type: Arc::clone(&doc_nt), attrs: std::collections::BTreeMap::new(),
                     content: super::fragment::Fragment::from_array(vec![para]), marks: vec![], text: None };
    let mut visited: Vec<String> = vec![];
    doc.nodes_between(0, doc.content.size, &mut |node, _pos, _parent, _idx| {
        visited.push(node.node_type.name.clone());
        true
    });
    assert!(visited.contains(&"paragraph".to_string()));
    assert!(visited.contains(&"text".to_string()));
}
```

实现：

```rust
/// 遍历 from..to 范围内所有节点，对每个调用 f(node, pos, parent, index)。
/// f 返回 false 时跳过该节点的子树。
pub fn nodes_between<F>(&self, from: usize, to: usize, f: &mut F)
where
    F: FnMut(&Node, usize, Option<&Node>, usize) -> bool,
{
    self.nodes_between_inner(from, to, f, 0, None, 0);
}

fn nodes_between_inner<F>(
    &self,
    from: usize,
    to: usize,
    f: &mut F,
    node_start: usize,
    parent: Option<&Node>,
    index: usize,
) where
    F: FnMut(&Node, usize, Option<&Node>, usize) -> bool,
{
    if !f(self, node_start, parent, index) {
        return;
    }
    let mut pos = node_start + 1;
    for i in 0..self.content.child_count() {
        let child = self.content.child(i);
        let end = pos + child.node_size();
        if pos < to && end > from {
            child.nodes_between_inner(from, to, f, pos, Some(self), i);
        }
        pos = end;
    }
}
```

- [ ] **Step 4: 实现并测试 check**

失败测试：

```rust
#[test]
fn node_check_valid() {
    use std::collections::HashMap;
    use super::content::ContentMatch;
    let text_nt = Arc::new(NodeType {
        name: "text".into(), groups: vec![], is_block: false,
        is_text: true, inline_content: false, mark_set: None, content_match: None,
    });
    let mut types: HashMap<String, Arc<NodeType>> = HashMap::new();
    types.insert("text".into(), Arc::clone(&text_nt));
    let cm = ContentMatch::parse("text*", &types).unwrap();
    let para_nt = Arc::new(NodeType {
        name: "paragraph".into(), groups: vec![], is_block: true,
        is_text: false, inline_content: true, mark_set: None,
        content_match: Some(cm),
    });
    let txt = Node { node_type: Arc::clone(&text_nt), attrs: std::collections::BTreeMap::new(),
                     content: super::fragment::Fragment::empty(), marks: vec![], text: Some("ok".into()) };
    let para = Node { node_type: Arc::clone(&para_nt), attrs: std::collections::BTreeMap::new(),
                      content: super::fragment::Fragment::from_array(vec![txt]), marks: vec![], text: None };
    assert!(para.check().is_ok());
}
```

实现：

```rust
/// 递归验证整棵树内容合法。
pub fn check(&self) -> Result<(), String> {
    self.node_type.check_content(&self.content)?;
    for i in 0..self.content.child_count() {
        self.content.child(i).check()?;
    }
    Ok(())
}
```

- [ ] **Step 5: 运行全量测试**

```bash
cd /Users/aporicho/Desktop/md-editor && cargo test 2>&1 | tail -20
```

期望：全部 PASS

- [ ] **Step 6: Commit**

```bash
git add src/model/node.rs
git commit -m "feat(node): add node_at, text_content, nodes_between, check"
```

---

## Task 8: 改进 create_and_fill 递归填充

**Files:**
- Modify: `src/model/schema.rs`

- [ ] **Step 1: 编写失败测试**

```rust
#[test]
fn create_and_fill_nested() {
    // blockquote 要求 paragraph+，paragraph 要求 text*（允许空）
    // create_and_fill(blockquote) 应能递归填出 blockquote(paragraph())
    use std::collections::HashMap;
    use super::content::ContentMatch;

    let text_nt = Arc::new(NodeType {
        name: "text".into(), groups: vec![], is_block: false,
        is_text: true, inline_content: false, mark_set: None, content_match: None,
    });
    let mut types: HashMap<String, Arc<NodeType>> = HashMap::new();
    types.insert("text".into(), Arc::clone(&text_nt));

    let cm_para = ContentMatch::parse("text*", &types).unwrap();
    let para_nt = Arc::new(NodeType {
        name: "paragraph".into(), groups: vec![], is_block: true,
        is_text: false, inline_content: true, mark_set: None,
        content_match: Some(cm_para),
    });
    types.insert("paragraph".into(), Arc::clone(&para_nt));

    let cm_bq = ContentMatch::parse("paragraph+", &types).unwrap();
    let bq_nt = Arc::new(NodeType {
        name: "blockquote".into(), groups: vec![], is_block: true,
        is_text: false, inline_content: false, mark_set: None,
        content_match: Some(cm_bq),
    });

    let result = bq_nt.create_and_fill();
    assert!(result.is_some(), "blockquote.create_and_fill() 应成功");
    let node = result.unwrap();
    assert_eq!(node.content.child_count(), 1);
    assert_eq!(node.content.child(0).node_type.name, "paragraph");
}
```

- [ ] **Step 2: 运行确认 FAIL**

```bash
cd /Users/aporicho/Desktop/md-editor && cargo test create_and_fill_nested 2>&1
```

期望：FAIL（返回 None，因为当前只填一层）

- [ ] **Step 3: 改进 create_and_fill 实现**

将 `schema.rs` 中的 `create_and_fill` 方法替换为：

```rust
pub fn create_and_fill(self: &Arc<Self>) -> Option<Node> {
    if self.has_required_attrs() {
        return None;
    }
    let content = if let Some(ref cm) = self.content_match {
        if cm.valid_end {
            Fragment::empty()
        } else {
            // BFS 找最短合法填充序列
            let filled = cm.fill_before(&Fragment::empty(), true, 0)?;
            // 递归填充每个子节点
            let mut children = Vec::new();
            for i in 0..filled.child_count() {
                let child_type = &filled.child(i).node_type;
                let child = child_type.create_and_fill()?;
                children.push(child);
            }
            Fragment::from_array(children)
        }
    } else {
        Fragment::empty()
    };
    Some(Node {
        node_type: Arc::clone(self),
        attrs: Attrs::new(),
        content,
        marks: vec![],
        text: None,
    })
}
```

- [ ] **Step 4: 运行测试**

```bash
cd /Users/aporicho/Desktop/md-editor && cargo test create_and_fill 2>&1
```

期望：PASS

- [ ] **Step 5: 运行全量测试**

```bash
cd /Users/aporicho/Desktop/md-editor && cargo test 2>&1 | tail -10
```

期望：全部 PASS

- [ ] **Step 6: Commit**

```bash
git add src/model/schema.rs
git commit -m "feat(schema): improve create_and_fill with recursive nested filling"
```

---

## 最终验证

- [ ] **运行完整测试套件**

```bash
cd /Users/aporicho/Desktop/md-editor && cargo test 2>&1
```

期望：所有测试 PASS，零 warning

- [ ] **验证 Schema ptr_eq 保证**

```bash
cd /Users/aporicho/Desktop/md-editor && cargo test schema_same_type_ptr_eq 2>&1
```

期望：PASS
