# transform 模块设计文档

**日期：** 2026-04-05
**状态：** 已确认，待实现
**参考：** `reference/prosemirror-transform/src/`、`docs/superpowers/specs/rust-prosemirror-porting-guide.md`

---

## 概述

将 `prosemirror-transform` TypeScript 库以 Rust 重写，作为编辑器操作层。transform 只依赖 model 模块，负责将文档变更表示为可序列化、可撤销、可映射的原子操作序列。

---

## 设计决策

### 决策 1：Step 用 trait 对象，不用枚举

TS 的 `abstract class Step` 靠继承扩展。Rust 用 trait 对象：

```rust
pub trait Step: Any + Send + Sync {
    fn apply(&self, doc: &Arc<Node>) -> StepResult;
    fn invert(&self, doc: &Arc<Node>) -> Box<dyn Step>;
    fn map(&self, mapping: &dyn Mappable) -> Option<Box<dyn Step>>;
    fn get_map(&self) -> StepMap;
    fn merge(&self, other: &dyn Step) -> Option<Box<dyn Step>>;
    fn to_json(&self) -> Value;
    fn as_any(&self) -> &dyn Any;  // 用于 merge 内部的类型判断
}
```

枚举方案会堵死扩展性——协同编辑、插件系统都依赖自定义 Step，必须用 trait 对象。

### 决策 2：Step 注册表用全局 `OnceLock<RwLock<HashMap>>`

TS 的 `Step.jsonID` 注册到模块级 `stepsByID` 对象。Rust 里用线程安全的全局注册表：

```rust
static STEP_REGISTRY: OnceLock<RwLock<HashMap<
    String,
    Box<dyn Fn(&Schema, &Value) -> Result<Box<dyn Step>, RangeError> + Send + Sync>,
>>> = OnceLock::new();

pub fn register_step(id: &str, from_json: impl Fn(&Schema, &Value) -> Result<Box<dyn Step>, RangeError> + Send + Sync + 'static) {
    STEP_REGISTRY
        .get_or_init(|| RwLock::new(HashMap::new()))
        .write()
        .unwrap()
        .insert(id.to_string(), Box::new(from_json));
}
```

### 决策 3：`merge` 内部用 `as_any` downcast 替代 `instanceof`

TS 里 `merge` 用 `instanceof` 判断类型：
```ts
if (!(other instanceof ReplaceStep)) return null
```

Rust 里通过 `as_any()` downcast：
```rust
fn merge(&self, other: &dyn Step) -> Option<Box<dyn Step>> {
    let other = other.as_any().downcast_ref::<ReplaceStep>()?;
    // ...
}
```

### 决策 4：`StepMap` 用具名 struct 替代平坦数组

TS 的 `ranges: readonly number[]` 将 `[start, oldSize, newSize]` 压缩进平坦数组。Rust 里用具名 struct，可读性更好，内存布局完全相同：

```rust
pub struct MapRange {
    pub start: u32,
    pub old_size: u32,
    pub new_size: u32,
}

pub struct StepMap {
    ranges: Vec<MapRange>,
    inverted: bool,
}
```

### 决策 5：Recovery 值用具名 struct，不做位打包

TS 把 `(index, offset)` 打包进一个 f64，注释里专门说明不能用位移运算（会截断到32位）。Rust 里直接用两个字段：

```rust
struct Recover {
    index: u32,
    offset: u32,
}
```

不需要任何编码和解码，无精度问题。

### 决策 6：`MapResult.delInfo` 用 `bitflags!`

TS 里用裸常量：
```ts
const DEL_BEFORE = 1, DEL_AFTER = 2, DEL_ACROSS = 4, DEL_SIDE = 8
```

Rust 里用 `bitflags` crate：
```rust
bitflags::bitflags! {
    pub struct DelFlags: u8 {
        const DEL_BEFORE = 0b0001;
        const DEL_AFTER  = 0b0010;
        const DEL_ACROSS = 0b0100;
        const DEL_SIDE   = 0b1000;
    }
}
```

类型安全，不会意外传入非法值。

### 决策 7：`Mapping.ownData` 被 Rust 所有权自然消除

TS 里 `Mapping.slice()` 共享底层数组，修改前需要懒惰克隆，靠 `ownData` 标志追踪。Rust 的 `Vec` 天然独占数据，`appendMap` 直接 push，无需任何克隆检查。

`slice()` 返回轻量视图，不共享数据：
```rust
pub struct MappingSlice<'a> {
    maps: &'a [StepMap],
    mirror: Option<&'a [usize]>,
    from: usize,
    to: usize,
}
```

### 决策 8：`Transform` 用毒化模式保留方法链

TS 里失败直接 throw，方法链不中断。Rust 里用毒化模式：

```rust
pub struct Transform {
    pub doc: Arc<Node>,
    pub steps: Vec<Box<dyn Step>>,
    pub docs: Vec<Arc<Node>>,
    pub mapping: Mapping,
    error: Option<TransformError>,  // 毒化字段
}

impl Transform {
    pub fn step(&mut self, step: Box<dyn Step>) -> &mut Self {
        if self.error.is_some() { return self; }  // 已毒化，跳过
        match self.maybe_step(step) {
            Ok(_) => self,
            Err(e) => { self.error = Some(e); self }
        }
    }

    pub fn into_result(self) -> Result<Transform, TransformError> {
        match self.error {
            None => Ok(self),
            Some(e) => Err(e),
        }
    }
}
```

链式调用完全保留，最后统一检查错误。

### 决策 9：`findWrapping` 返回具名 struct

TS 返回 `{type: NodeType, attrs: Attrs | null}[] | null`。Rust 里用具名 struct：

```rust
pub struct WrapperSpec {
    pub node_type: Arc<NodeType>,
    pub attrs: Option<Attrs>,
}
// 返回 Option<Vec<WrapperSpec>>
```

### 决策 10：`Transform.changedRange()` 返回具名 struct

TS 返回 `{from, to} | null`。Rust 里：

```rust
pub struct ChangedRange {
    pub from: usize,
    pub to: usize,
}
// 返回 Option<ChangedRange>
```

### 决策 11：`addMark` 积累元组，不修改已创建的 Step

TS 里 `addMark` 靠 `(removing as any).to = end` 绕过 `readonly` 限制直接修改已创建的 Step 对象，扩展其范围。这是 TS 特有的 hack，Rust 的不可变原则不允许这种写法。

Rust 里先积累 `(start, end, mark)` 元组，最后批量创建 Step：

```rust
let mut to_remove: Vec<(usize, usize, Mark)> = vec![];
let mut to_add: Vec<(usize, usize, Mark)> = vec![];
// 遍历时只积累范围，不创建 Step
// 遍历完成后统一创建并应用
for (start, end, mark) in to_remove {
    tr.step(Box::new(RemoveMarkStep::new(start, end, mark)));
}
```

### 决策 12：`StepResult::from_replace` 用 `match` 替代 try/catch

TS 里用 try/catch 捕获 `ReplaceError`，其他异常继续抛出。Rust 里 `replace` 返回 `Result`，直接 match，不会误吞其他错误：

```rust
pub fn from_replace(doc: &Arc<Node>, from: usize, to: usize, slice: Slice) -> Self {
    match doc.replace(from, to, slice) {
        Ok(new_doc) => StepResult::ok(new_doc),
        Err(e) => StepResult::fail(e.to_string()),
    }
}
```

### 决策 13：`_map` 拆成两个方法

TS 里 `_map(pos, assoc, simple: boolean)` 用 bool 控制返回 `number` 或 `MapResult`，靠动态类型逃脱类型检查。Rust 类型系统不允许同一函数返回不同类型，拆成两个方法：

- `map(&self, pos: usize, assoc: i8) -> usize`
- `map_result(&self, pos: usize, assoc: i8) -> MapResult`

### 决策 14：`Fittable` 可选字段用 `Option`

TS 里 `Fittable` 接口的 `inject?` 和 `wrap?` 是可选字段。Rust 里直接对应：

```rust
struct Fittable {
    slice_depth: usize,
    frontier_depth: usize,
    parent: Option<Arc<Node>>,
    inject: Option<Fragment>,
    wrap: Option<Vec<Arc<NodeType>>>,
}
```

### 决策 15：`MAP_BIAS` 用 `AtomicI8`，保留运行时可配置

TS 里 `ReplaceStep.MAP_BIAS` 是静态可变字段，控制协同编辑时插入位置映射的偏向（前面还是后面）。项目最终要支持协同编辑，这个字段不是历史遗留，是真实需求。

Rust 里用 `AtomicI8` 实现全局可配置，线程安全，无需 `Mutex`：

```rust
static MAP_BIAS: AtomicI8 = AtomicI8::new(1);

// 读取
let bias = MAP_BIAS.load(Ordering::Relaxed);
// 设置（协同编辑场景下可切换为 -1）
MAP_BIAS.store(-1, Ordering::Relaxed);
```

---

## 文件结构

```
src/transform/
  mod.rs           ← 公开导出所有类型
  step.rs          ← Step trait、StepResult、全局注册表
  map.rs           ← StepMap、MapResult、Mapping、Mappable trait
  transform.rs     ← Transform
  replace_step.rs  ← ReplaceStep、ReplaceAroundStep
  mark_step.rs     ← AddMarkStep、RemoveMarkStep、AddNodeMarkStep、RemoveNodeMarkStep
  attr_step.rs     ← AttrStep、DocAttrStep
  replace.rs       ← replaceStep、replaceRange、replaceRangeWith、deleteRange
  mark.rs          ← addMark、removeMark、clearIncompatible
  structure.rs     ← lift、wrap、split、join、setBlockType、setNodeMarkup、
                     liftTarget、findWrapping、canJoin、canSplit、
                     joinPoint、insertPoint、dropPoint
```

---

## 核心类型定义

### Step trait

```rust
pub trait Step: Any + Send + Sync {
    fn apply(&self, doc: &Arc<Node>) -> StepResult;
    fn invert(&self, doc: &Arc<Node>) -> Box<dyn Step>;
    fn map(&self, mapping: &dyn Mappable) -> Option<Box<dyn Step>>;
    fn get_map(&self) -> StepMap { StepMap::empty() }
    fn merge(&self, _other: &dyn Step) -> Option<Box<dyn Step>> { None }
    fn to_json(&self) -> Value;
    fn from_json(schema: &Schema, json: &Value) -> Result<Box<dyn Step>, RangeError>
        where Self: Sized;
    fn as_any(&self) -> &dyn Any;
}
```

### StepResult

```rust
pub struct StepResult {
    pub doc: Option<Arc<Node>>,
    pub failed: Option<String>,
}

impl StepResult {
    pub fn ok(doc: Arc<Node>) -> Self { ... }
    pub fn fail(message: impl Into<String>) -> Self { ... }
    pub fn from_replace(doc: &Arc<Node>, from: usize, to: usize, slice: Slice) -> Self { ... }
    pub fn is_ok(&self) -> bool { self.failed.is_none() }
}
```

### StepMap

```rust
pub struct StepMap {
    ranges: Vec<MapRange>,
    inverted: bool,
}

pub struct MapRange {
    pub start: u32,
    pub old_size: u32,
    pub new_size: u32,
}
```

### MapResult

```rust
pub struct MapResult {
    pub pos: usize,
    flags: DelFlags,
    recover: Option<Recover>,
}

impl MapResult {
    pub fn deleted(&self) -> bool { self.flags.contains(DelFlags::DEL_SIDE) }
    pub fn deleted_before(&self) -> bool { self.flags.intersects(DelFlags::DEL_BEFORE | DelFlags::DEL_ACROSS) }
    pub fn deleted_after(&self) -> bool { self.flags.intersects(DelFlags::DEL_AFTER | DelFlags::DEL_ACROSS) }
    pub fn deleted_across(&self) -> bool { self.flags.contains(DelFlags::DEL_ACROSS) }
}
```

### Mappable trait

```rust
pub trait Mappable {
    fn map(&self, pos: usize, assoc: i8) -> usize;
    fn map_result(&self, pos: usize, assoc: i8) -> MapResult;
}
```

### Transform

```rust
pub struct Transform {
    pub doc: Arc<Node>,
    pub steps: Vec<Box<dyn Step>>,
    pub docs: Vec<Arc<Node>>,
    pub mapping: Mapping,
    error: Option<TransformError>,
}
```

---

## 错误类型

```rust
pub struct TransformError(pub String);
impl std::error::Error for TransformError {}
impl fmt::Display for TransformError { ... }
```

---

## 测试文件对应关系

| TS 测试文件 | Rust 测试文件 |
|---|---|
| `test-trans.ts` | `src/transform/tests_trans.rs` |
| `test-mapping.ts` | `src/transform/tests_mapping.rs` |
| `test-mark.ts` | `src/transform/tests_mark.rs` |
| `test-structure.ts` | `src/transform/tests_structure.rs` |

---

## 外部依赖新增

```toml
bitflags = "2"
```
