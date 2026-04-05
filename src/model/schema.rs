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
}

// Suppress unused import warning for fmt (only used in removed manual Debug impl)
#[allow(unused_imports)]
use fmt as _;
