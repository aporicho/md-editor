use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use indexmap::IndexMap;
use serde_json::Value;

use crate::model::render_output::RenderOutput;

/// An object mapping attribute names to values.
///
/// Corresponds to `Attrs = {readonly [attr: string]: any}` in TypeScript.
#[allow(dead_code)]
pub type Attrs = Arc<HashMap<String, Value>>;

/// Rendering hook type for a node: given a node's attrs, returns a RenderOutput.
pub type NodeRenderFn = Box<dyn Fn(&Value) -> RenderOutput + Send + Sync>;
/// Leaf text extractor type for a node: given a node's attrs, returns a plain-text string.
pub type NodeLeafTextFn = Box<dyn Fn(&Value) -> String + Send + Sync>;
/// Debug string producer type for a node.
pub type NodeDebugFn = Box<dyn Fn(&Value) -> String + Send + Sync>;
/// Rendering hook type for a mark: given mark attrs and whether it's an opening mark.
pub type MarkRenderFn = Box<dyn Fn(&Value, bool) -> RenderOutput + Send + Sync>;
/// Custom attribute validator function type.
pub type AttrValidatorFn = Box<dyn Fn(&Value) -> Result<(), String> + Send + Sync>;

/// Whitespace handling mode for a node type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
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
#[allow(dead_code)]
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
    #[allow(dead_code)]
    pub fn excludes(&self, self_name: &str, other_name: &str) -> bool {
        match self {
            ExcludeSpec::SelfOnly => self_name == other_name,
            ExcludeSpec::All => true,
            ExcludeSpec::Named(names) => names.iter().any(|n| n == other_name),
        }
    }
}

/// Validates attribute values. Used in AttrSpec.
#[allow(dead_code)]
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
#[derive(Debug)]
#[allow(dead_code)]
pub struct AttrSpec {
    /// The default value for this attribute. `None` means the attribute is required.
    pub default: Option<Value>,
    /// Optional validator for this attribute's value.
    pub validate: Option<AttrValidator>,
}

impl AttrSpec {
    /// Returns true if this attribute must be explicitly provided (no default).
    #[allow(dead_code)]
    pub fn is_required(&self) -> bool {
        self.default.is_none()
    }

    /// Validates a value against this spec. Returns `Err` if validation fails.
    #[allow(dead_code)]
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
#[allow(dead_code)]
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
    pub to_render: Option<NodeRenderFn>,
    /// Returns a plain-text string for leaf node content (used in textBetween).
    pub leaf_text: Option<NodeLeafTextFn>,
    /// Returns a debug string representation of this node.
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
#[allow(dead_code)]
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
#[allow(dead_code)]
pub struct SchemaSpec {
    /// The node types in this schema, in order.
    pub nodes: IndexMap<String, NodeSpec>,
    /// The mark types in this schema. The order determines mark set sorting and parse rule priority.
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
}
