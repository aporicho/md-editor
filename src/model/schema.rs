#![allow(dead_code)] // TODO: Phase 2 完成后移除

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use indexmap::IndexMap;
use serde_json::Value;
use typed_builder::TypedBuilder;

use crate::model::render_output::RenderOutput;

/// An object mapping attribute names to values.
///
/// Corresponds to `Attrs = {readonly [attr: string]: any}` in TypeScript.
pub type Attrs = Arc<HashMap<String, Value>>;

/// Rendering hook type for a node: given a node's attrs, returns a RenderOutput.
pub type NodeRenderFn = Arc<dyn Fn(&Value) -> RenderOutput + Send + Sync>;
/// Leaf text extractor type for a node: given a node's attrs, returns a plain-text string.
pub type NodeLeafTextFn = Arc<dyn Fn(&Value) -> String + Send + Sync>;
/// Debug string producer type for a node.
pub type NodeDebugFn = Arc<dyn Fn(&Value) -> String + Send + Sync>;
/// Rendering hook type for a mark: given mark attrs and whether it's an opening mark.
pub type MarkRenderFn = Arc<dyn Fn(&Value, bool) -> RenderOutput + Send + Sync>;
/// Custom attribute validator function type.
pub type AttrValidatorFn = Arc<dyn Fn(&Value) -> Result<(), String> + Send + Sync>;

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
#[derive(Clone)]
pub enum AttrValidator {
    /// A `|`-separated list of allowed JSON primitive type names
    /// (e.g. `"string|null"`). Validated against the JSON value's type name.
    Types(Vec<String>),
    /// A custom validation function. Should return `Err(message)` on failure.
    Func(AttrValidatorFn),
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
#[derive(TypedBuilder, Clone, Debug)]
pub struct AttrSpec {
    /// The default value for this attribute. `None` means the attribute is required.
    #[builder(default)]
    pub default: Option<Value>,
    /// Optional validator for this attribute's value.
    #[builder(default)]
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
#[derive(TypedBuilder, Clone)]
pub struct NodeSpec {
    /// The content expression for this node (e.g. `"block+"`, `"inline*"`).
    #[builder(default)]
    pub content: Option<String>,
    /// Space-separated mark names/groups allowed inside this node.
    /// `"_"` = all marks, `""` = no marks, `None` = default (all for inline content).
    #[builder(default)]
    pub marks: Option<String>,
    /// Space-separated group names this node belongs to (e.g. `"block"`).
    #[builder(default)]
    pub group: Option<String>,
    /// Should be set to true for inline nodes (implied for text nodes).
    #[builder(default)]
    pub inline: bool,
    /// True for nodes that are not leaf nodes but don't have directly editable content.
    #[builder(default)]
    pub atom: bool,
    /// Attribute specs for this node type.
    #[builder(default = IndexMap::new())]
    pub attrs: IndexMap<String, AttrSpec>,
    /// Whether this node can be selected as a node selection. Defaults to true.
    #[builder(default = true)]
    pub selectable: bool,
    /// Whether this node can be dragged without being selected. Defaults to false.
    #[builder(default)]
    pub draggable: bool,
    /// Whether this node contains code (affects some command behavior).
    #[builder(default)]
    pub code: bool,
    /// Whitespace handling override. Defaults to `Pre` when `code` is true.
    #[builder(default)]
    pub whitespace: Option<Whitespace>,
    /// When enabled, enables both `defining_as_context` and `defining_for_content`.
    #[builder(default)]
    pub defining: bool,
    /// Whether this node is a context-defining parent during replace operations.
    #[builder(default)]
    pub defining_as_context: bool,
    /// Whether this node is a content-defining wrapper during replace operations.
    #[builder(default)]
    pub defining_for_content: bool,
    /// Whether editing operations won't cross node boundaries.
    #[builder(default)]
    pub isolating: bool,
    /// Whether this inline leaf node acts as a linebreak replacement.
    #[builder(default)]
    pub linebreak_replacement: bool,
    /// Rendering hook (cosmic-text placeholder). Called to render a node.
    #[builder(default, setter(strip_option))]
    pub to_render: Option<NodeRenderFn>,
    /// Returns a plain-text string for leaf node content (used in textBetween).
    #[builder(default, setter(strip_option))]
    pub leaf_text: Option<NodeLeafTextFn>,
    /// Returns a debug string representation of this node.
    #[builder(default, setter(strip_option))]
    pub to_debug_string: Option<NodeDebugFn>,
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
#[derive(TypedBuilder, Clone)]
pub struct MarkSpec {
    /// Attribute specs for this mark type.
    #[builder(default = IndexMap::new())]
    pub attrs: IndexMap<String, AttrSpec>,
    /// Whether this mark is active at the cursor when positioned at its end. Defaults to true.
    #[builder(default = true)]
    pub inclusive: bool,
    /// Space-separated mark names/groups this mark excludes.
    /// `"_"` = all, `""` = none, `None` = only excludes itself (default).
    #[builder(default)]
    pub excludes: Option<String>,
    /// Space-separated group names this mark belongs to.
    #[builder(default)]
    pub group: Option<String>,
    /// Whether this mark spans multiple adjacent nodes when serialized. Defaults to true.
    #[builder(default = true)]
    pub spanning: bool,
    /// Whether this mark's content is code.
    #[builder(default)]
    pub code: bool,
    /// Rendering hook (cosmic-text placeholder). Called to render a mark.
    #[builder(default, setter(strip_option))]
    pub to_render: Option<MarkRenderFn>,
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
    /// The mark types in this schema. The order determines mark set sorting and parse rule priority.
    pub marks: IndexMap<String, MarkSpec>,
    /// The name of the default top-level node. Defaults to `"doc"`.
    pub top_node: Option<String>,
}

/// Merge provided attribute values with spec defaults.
///
/// For node types where all attrs have a default value (or which don't
/// have any attributes), build up a single reusable default attribute
/// object, and use it for all nodes that don't specify specific attributes.
///
/// Corresponds to `computeAttrs` in TypeScript.
pub(crate) fn compute_attrs(
    specs: &IndexMap<String, AttrSpec>,
    values: Option<&std::collections::HashMap<String, serde_json::Value>>,
) -> Result<Attrs, crate::model::errors::SchemaError> {
    use std::collections::HashMap;
    let mut built: HashMap<String, serde_json::Value> = HashMap::new();
    for (name, spec) in specs {
        let given = values.and_then(|v| v.get(name));
        let value = if let Some(v) = given {
            v.clone()
        } else if let Some(d) = &spec.default {
            d.clone()
        } else {
            return Err(crate::model::errors::SchemaError(format!(
                "No value supplied for attribute {}",
                name
            )));
        };
        built.insert(name.clone(), value);
    }
    Ok(Arc::new(built))
}

/// Pre-compute a reusable default Attrs map.
///
/// Returns `None` if any attribute has no default (i.e. is required).
/// Returns `Some(attrs)` with all defaults filled in otherwise.
///
/// Corresponds to `defaultAttrs` in TypeScript.
pub(crate) fn default_attrs(specs: &IndexMap<String, AttrSpec>) -> Option<Attrs> {
    compute_attrs(specs, None).ok()
}

/// Like nodes, marks (which are associated with nodes to signify
/// things like emphasis or being part of a link) are
/// tagged with type objects, which are instantiated once per `Schema`.
///
/// Corresponds to `MarkType` in TypeScript.
/// Note: `schema` back-reference removed (design decision 5 — avoids circular Arc).
#[derive(Clone)]
pub struct MarkType {
    /// The name of the mark type.
    pub name: String,
    /// @internal rank determines mark set ordering.
    pub rank: usize,
    /// Whether this mark should be active when the cursor is positioned at its end.
    pub inclusive: bool,
    /// The exclude relationship for this mark type.
    /// Filled in by `Schema::new`; defaults to `ExcludeSpec::SelfOnly`.
    pub excluded: ExcludeSpec,
    /// The spec on which the type is based.
    pub spec: MarkSpec,
    /// Pre-computed default attributes (None if any attribute is required).
    pub(crate) default_attrs: Option<Attrs>,
}

/// Node types are objects allocated once per `Schema` and used to
/// tag `Node` instances. They contain information about the node type,
/// such as its name and what kind of node it represents.
///
/// Corresponds to `NodeType` in TypeScript.
/// Note: `schema` back-reference removed (design decision 5 — avoids circular Arc).
#[derive(Clone)]
pub struct NodeType {
    /// The name the node type has in this schema.
    pub name: String,
    /// True if this is a block-level node (`!(spec.inline || name == "text")`).
    pub is_block: bool,
    /// True if this is the text node type.
    pub is_text: bool,
    /// True if this node type has inline content.
    /// Set during Schema construction from `content_match.inline_content`.
    pub inline_content: bool,
    /// The groups this node type belongs to (derived from `spec.group`).
    pub groups: Vec<String>,
    /// The starting match state of the node type's content expression.
    /// Initialised to `ContentMatch::empty()`; filled in by `Schema::new`.
    pub content_match: Arc<crate::model::content::ContentMatch>,
    /// The set of marks allowed in this node. `None` means all marks are allowed.
    /// Filled in by `Schema::new`.
    pub mark_set: Option<Vec<Arc<MarkType>>>,
    /// The node type's whitespace handling mode.
    pub whitespace: Whitespace,
    /// The spec that this type is based on.
    pub spec: NodeSpec,
    /// Pre-computed default attributes (None if any attribute is required).
    pub(crate) default_attrs: Option<Attrs>,
}

impl NodeType {
    /// True if this is an inline type.
    ///
    /// True if this is an inline type.
    pub fn is_inline(&self) -> bool {
        !self.is_block
    }

    /// True if this is a textblock type, a block that contains inline content.
    ///
    /// True if this is a textblock type, a block that contains inline
    /// content.
    pub fn is_textblock(&self) -> bool {
        self.is_block && self.inline_content
    }

    /// True for node types that allow no content.
    ///
    /// True for node types that allow no content.
    pub fn is_leaf(&self) -> bool {
        self.content_match.edge_count() == 0 && self.content_match.valid_end
    }

    /// True when this node is an atom, i.e. when it does not have
    /// directly editable content.
    pub fn is_atom(&self) -> bool {
        self.is_leaf() || self.spec.atom
    }

    /// Return true when this node type is part of the given group.
    ///
    /// Return true when this node type is part of the given
    /// [group](#model.NodeSpec.group).
    pub fn is_in_group(&self, group: &str) -> bool {
        self.groups.iter().any(|g| g == group)
    }

    /// The node type's whitespace option.
    ///
    /// The node type's [whitespace](#model.NodeSpec.whitespace) option.
    pub fn whitespace(&self) -> Whitespace {
        self.spec.whitespace.unwrap_or(if self.spec.code {
            Whitespace::Pre
        } else {
            Whitespace::Normal
        })
    }

    /// Tells you whether this node type has any required attributes.
    ///
    /// Tells you whether this node type has any required attributes.
    pub fn has_required_attrs(&self) -> bool {
        self.spec.attrs.values().any(|a| a.is_required())
    }

    /// Indicates whether this node allows some of the same content as
    /// the given node type.
    pub fn compatible_content(&self, other: &NodeType) -> bool {
        std::ptr::eq(self, other) || self.content_match.compatible(&other.content_match)
    }

    /// Compute attributes, merging provided values with spec defaults.
    ///
    /// @internal
    pub(crate) fn compute_attrs_for(
        &self,
        attrs: Option<&std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<Attrs, crate::model::errors::SchemaError> {
        if attrs.is_none() {
            if let Some(defaults) = &self.default_attrs {
                return Ok(defaults.clone());
            }
        }
        compute_attrs(&self.spec.attrs, attrs)
    }

    /// Check whether the given mark type is allowed in this node.
    ///
    /// Check whether the given mark type is allowed in this node.
    pub fn allows_mark_type(&self, mark_type: &Arc<MarkType>) -> bool {
        self.mark_set
            .as_ref()
            .map_or(true, |set| set.iter().any(|m| Arc::ptr_eq(m, mark_type)))
    }

    // ---- Phase 3 stubs (依赖 Fragment / Node / Mark) ----

    /// Create a `Node` of this type.
    ///
    /// NOTE: stub — requires Node/Fragment/Mark (Phase 3).
    #[allow(unused)]
    pub fn create_stub(&self) -> ! {
        todo!("NodeType::create requires Node/Fragment/Mark — implement in Phase 3")
    }

    /// Like `create`, but check the given content against the node type's content restrictions.
    ///
    /// NOTE: stub — requires Fragment (Phase 3).
    #[allow(unused)]
    pub fn create_checked_stub(&self) -> ! {
        todo!("NodeType::create_checked requires Fragment — implement in Phase 3")
    }

    /// Like `create`, but fill in missing content automatically.
    ///
    /// NOTE: stub — requires Fragment (Phase 3).
    #[allow(unused)]
    pub fn create_and_fill_stub(&self) -> ! {
        todo!("NodeType::create_and_fill requires Fragment — implement in Phase 3")
    }

    /// Returns true if the given fragment is valid content for this node type.
    ///
    /// NOTE: stub — requires Fragment (Phase 3).
    #[allow(unused)]
    pub fn valid_content_stub(&self) -> ! {
        todo!("NodeType::valid_content requires Fragment — implement in Phase 3")
    }

    /// Test whether the given set of marks are allowed in this node.
    ///
    /// NOTE: stub — requires Mark (Phase 3).
    #[allow(unused)]
    pub fn allows_marks_stub(&self) -> ! {
        todo!("NodeType::allows_marks requires Mark — implement in Phase 3")
    }

    /// Removes the marks that are not allowed in this node from the given set.
    ///
    /// NOTE: stub — requires Mark (Phase 3).
    #[allow(unused)]
    pub fn allowed_marks_stub(&self) -> ! {
        todo!("NodeType::allowed_marks requires Mark — implement in Phase 3")
    }
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
            validate: Some(AttrValidator::Func(Arc::new(|v| {
                if v.is_number() { Ok(()) } else { Err("not a number".to_string()) }
            }))),
        };
        assert!(spec.validate_value(&json!(42)).is_ok());
        assert!(spec.validate_value(&json!("x")).is_err());
    }

    #[test]
    fn schema_spec_construction() {
        let spec = SchemaSpec {
            nodes: IndexMap::new(),
            marks: IndexMap::new(),
            top_node: None,
        };
        assert!(spec.top_node.is_none());
    }

    #[test]
    fn whitespace_variants() {
        assert_eq!(Whitespace::Normal, Whitespace::Normal);
        assert_eq!(Whitespace::Pre, Whitespace::Pre);
        assert_ne!(Whitespace::Normal, Whitespace::Pre);
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

    #[test]
    fn node_spec_builder_defaults() {
        let spec = NodeSpec::builder().build();
        assert!(spec.content.is_none());
        assert!(spec.selectable);
        assert!(!spec.inline);
    }

    #[test]
    fn node_spec_builder_with_content() {
        let spec = NodeSpec::builder()
            .content(Some("block+".to_string()))
            .group(Some("block".to_string()))
            .build();
        assert_eq!(spec.content.as_deref(), Some("block+"));
        assert_eq!(spec.group.as_deref(), Some("block"));
        assert!(spec.selectable);
    }

    #[test]
    fn mark_spec_builder_defaults() {
        let spec = MarkSpec::builder().build();
        assert!(spec.inclusive);
        assert!(spec.spanning);
        assert!(!spec.code);
    }

    #[test]
    fn attr_spec_builder() {
        let spec = AttrSpec::builder()
            .default(Some(json!(42)))
            .build();
        assert_eq!(spec.default, Some(json!(42)));
        assert!(spec.validate.is_none());
    }

    // ---- compute_attrs / default_attrs ----
    use std::collections::HashMap as StdHashMap;

    #[test]
    fn compute_attrs_uses_spec_defaults() {
        let mut specs: IndexMap<String, AttrSpec> = IndexMap::new();
        specs.insert("level".to_string(), AttrSpec { default: Some(json!(1)), validate: None });
        let result = compute_attrs(&specs, None).unwrap();
        assert_eq!(result.get("level"), Some(&json!(1)));
    }

    #[test]
    fn compute_attrs_provided_values_override_defaults() {
        let mut specs: IndexMap<String, AttrSpec> = IndexMap::new();
        specs.insert("level".to_string(), AttrSpec { default: Some(json!(1)), validate: None });
        let mut vals: StdHashMap<String, serde_json::Value> = StdHashMap::new();
        vals.insert("level".to_string(), json!(3));
        let result = compute_attrs(&specs, Some(&vals)).unwrap();
        assert_eq!(result.get("level"), Some(&json!(3)));
    }

    #[test]
    fn compute_attrs_errors_on_missing_required() {
        let mut specs: IndexMap<String, AttrSpec> = IndexMap::new();
        specs.insert("href".to_string(), AttrSpec { default: None, validate: None });
        let err = compute_attrs(&specs, None).unwrap_err();
        assert!(err.0.contains("href"));
    }

    #[test]
    fn compute_attrs_empty_specs_returns_empty_map() {
        let specs: IndexMap<String, AttrSpec> = IndexMap::new();
        let result = compute_attrs(&specs, None).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn default_attrs_some_when_all_have_defaults() {
        let mut specs: IndexMap<String, AttrSpec> = IndexMap::new();
        specs.insert("level".to_string(), AttrSpec { default: Some(json!(1)), validate: None });
        assert!(default_attrs(&specs).is_some());
    }

    #[test]
    fn default_attrs_none_when_any_required() {
        let mut specs: IndexMap<String, AttrSpec> = IndexMap::new();
        specs.insert("href".to_string(), AttrSpec { default: None, validate: None });
        assert!(default_attrs(&specs).is_none());
    }

    // ---- NodeType 谓词测试 ----
    use crate::model::content::ContentMatch;

    fn make_paragraph_nodetype() -> NodeType {
        let nt_map: IndexMap<String, bool> = [("text".to_string(), true)].into_iter().collect();
        let g: IndexMap<String, Vec<String>> = IndexMap::new();
        let r: IndexMap<String, bool> = IndexMap::new();
        let cm = ContentMatch::parse("text*", &nt_map, &g, &r).unwrap();
        NodeType {
            name: "paragraph".to_string(),
            is_block: true,
            is_text: false,
            inline_content: cm.inline_content,
            groups: vec!["block".to_string()],
            content_match: cm,
            mark_set: None,
            whitespace: Whitespace::Normal,
            spec: NodeSpec::default(),
            default_attrs: None,
        }
    }

    fn make_text_nodetype() -> NodeType {
        NodeType {
            name: "text".to_string(),
            is_block: false,
            is_text: true,
            inline_content: false,
            groups: vec![],
            content_match: ContentMatch::empty(),
            mark_set: None,
            whitespace: Whitespace::Normal,
            spec: NodeSpec::default(),
            default_attrs: None,
        }
    }

    fn make_image_nodetype() -> NodeType {
        NodeType {
            name: "image".to_string(),
            is_block: false,
            is_text: false,
            inline_content: false,
            groups: vec!["inline".to_string()],
            content_match: ContentMatch::empty(),
            mark_set: None,
            whitespace: Whitespace::Normal,
            spec: NodeSpec { atom: true, ..NodeSpec::default() },
            default_attrs: None,
        }
    }

    #[test]
    fn nodetype_paragraph_is_block_not_inline() {
        let p = make_paragraph_nodetype();
        assert!(p.is_block);
        assert!(!p.is_text);
        assert!(!p.is_inline());
    }

    #[test]
    fn nodetype_text_is_inline_and_text() {
        let t = make_text_nodetype();
        assert!(t.is_text);
        assert!(t.is_inline());
        assert!(!t.is_block);
    }

    #[test]
    fn nodetype_paragraph_is_textblock() {
        let p = make_paragraph_nodetype();
        assert!(p.is_textblock());
    }

    #[test]
    fn nodetype_image_is_leaf_and_atom() {
        let img = make_image_nodetype();
        assert!(img.is_leaf());
        assert!(img.is_atom());
    }

    #[test]
    fn nodetype_paragraph_is_not_leaf() {
        let p = make_paragraph_nodetype();
        assert!(!p.is_leaf());
        assert!(!p.is_atom());
    }

    #[test]
    fn nodetype_is_in_group() {
        let p = make_paragraph_nodetype();
        assert!(p.is_in_group("block"));
        assert!(!p.is_in_group("inline"));
    }

    #[test]
    fn nodetype_whitespace_normal_by_default() {
        let p = make_paragraph_nodetype();
        assert_eq!(p.whitespace(), Whitespace::Normal);
    }

    #[test]
    fn nodetype_whitespace_pre_when_code_true() {
        let code_node = NodeType {
            spec: NodeSpec { code: true, ..NodeSpec::default() },
            ..make_paragraph_nodetype()
        };
        assert_eq!(code_node.whitespace(), Whitespace::Pre);
    }

    #[test]
    fn nodetype_whitespace_spec_override() {
        let pre_node = NodeType {
            spec: NodeSpec { whitespace: Some(Whitespace::Pre), ..NodeSpec::default() },
            ..make_paragraph_nodetype()
        };
        assert_eq!(pre_node.whitespace(), Whitespace::Pre);
    }

    #[test]
    fn nodetype_has_required_attrs_false_when_empty() {
        let p = make_paragraph_nodetype();
        assert!(!p.has_required_attrs());
    }

    #[test]
    fn nodetype_has_required_attrs_true_when_any_required() {
        use indexmap::indexmap;
        let p = NodeType {
            spec: NodeSpec {
                attrs: indexmap! {
                    "href".to_string() => AttrSpec { default: None, validate: None }
                },
                ..NodeSpec::default()
            },
            ..make_paragraph_nodetype()
        };
        assert!(p.has_required_attrs());
    }
}

// Suppress unused import warning for fmt (only used in removed manual Debug impl)
#[allow(unused_imports)]
use fmt as _;
