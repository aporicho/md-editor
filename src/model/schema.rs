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
        if attrs.is_none() && let Some(defaults) = &self.default_attrs {
            return Ok(defaults.clone());
        }
        compute_attrs(&self.spec.attrs, attrs)
    }

    /// Check whether the given mark type is allowed in this node.
    ///
    /// Check whether the given mark type is allowed in this node.
    pub fn allows_mark_type(&self, mark_type: &Arc<MarkType>) -> bool {
        self.mark_set
            .as_ref()
            .is_none_or(|set| set.iter().any(|m| Arc::ptr_eq(m, mark_type)))
    }

    // ---- Phase 3 implementations ----

    /// Create a `Node` of this type.
    ///
    /// Create a `Node` of this type. The given attributes are
    /// checked and defaulted (you can pass `None` to use the type's
    /// defaults entirely, if no required attributes exist). `content`
    /// may be a `Fragment` or `None`.
    pub fn create(
        self: &Arc<Self>,
        attrs: Option<&std::collections::HashMap<String, serde_json::Value>>,
        content: Option<crate::model::fragment::Fragment>,
        marks: Vec<crate::model::mark::Mark>,
    ) -> Result<crate::model::node::Node, crate::model::errors::SchemaError> {
        if self.is_text {
            return Err(crate::model::errors::SchemaError(
                "NodeType.create can't construct text nodes".to_string(),
            ));
        }
        let attrs = self.compute_attrs_for(attrs)?;
        let content = content.unwrap_or_else(crate::model::fragment::Fragment::empty);
        Ok(crate::model::node::Node::new(self.clone(), attrs, content, marks))
    }

    /// Like `create`, but check the given content against the node type's content restrictions.
    ///
    /// Like `create`, but check the given content
    /// against the node type's content restrictions, and return an error
    /// if it doesn't match.
    ///
    /// NOTE: full content validation (valid_content) requires match_fragment (Phase 3.4).
    /// For now, creates without content check; Phase 3.4 will enable the check.
    pub fn create_checked(
        self: &Arc<Self>,
        attrs: Option<&std::collections::HashMap<String, serde_json::Value>>,
        content: Option<crate::model::fragment::Fragment>,
        marks: Vec<crate::model::mark::Mark>,
    ) -> Result<crate::model::node::Node, crate::model::errors::RangeError> {
        let attrs = self.compute_attrs_for(attrs).map_err(|e| crate::model::errors::RangeError(e.0))?;
        let content = content.unwrap_or_else(crate::model::fragment::Fragment::empty);
        Ok(crate::model::node::Node::new(self.clone(), attrs, content, marks))
    }

    /// Like `create`, but fill in missing content automatically.
    ///
    /// If no fitting content can be constructed, returns `None`.
    pub fn create_and_fill(
        self: &Arc<Self>,
        attrs: Option<&std::collections::HashMap<String, serde_json::Value>>,
        marks: Vec<crate::model::mark::Mark>,
    ) -> Option<crate::model::node::Node> {
        if self.is_text {
            return None;
        }
        let attrs = self.compute_attrs_for(attrs).ok()?;
        // Start with empty content
        let content = crate::model::fragment::Fragment::empty();
        // match_fragment(empty, 0, 0) 迭代 0 次，返回初始 match state 本身（content_match 起始状态）
        let matched = self.content_match.match_fragment(&content, 0, 0)?;
        // Fill to reach a valid end from matched state
        let after = matched.fill_before(
            &crate::model::fragment::Fragment::empty(),
            true,
            0,
        )?;
        Some(crate::model::node::Node::new(
            self.clone(),
            attrs,
            content.append(&after),
            marks,
        ))
    }

    /// Returns true if the given fragment is valid content for this node type.
    pub fn valid_content(&self, content: &crate::model::fragment::Fragment) -> bool {
        match self.content_match.match_fragment(content, 0, content.child_count()) {
            None => false,
            Some(m) if !m.valid_end => false,
            Some(_) => {
                for i in 0..content.child_count() {
                    if let Ok(child) = content.child(i)
                        && !self.allows_marks(&child.marks)
                    {
                        return false;
                    }
                }
                true
            }
        }
    }

    /// Test whether the given set of marks are allowed in this node.
    ///
    /// Test whether the given set of marks are allowed in this node.
    pub fn allows_marks(&self, marks: &[crate::model::mark::Mark]) -> bool {
        if self.mark_set.is_none() {
            return true;
        }
        marks.iter().all(|m| self.allows_mark_type(&m.mark_type))
    }

    /// Removes the marks that are not allowed in this node from the given set.
    ///
    /// Removes the marks that are not allowed in this node from the given set.
    pub fn allowed_marks(&self, marks: &[crate::model::mark::Mark]) -> Vec<crate::model::mark::Mark> {
        if self.mark_set.is_none() {
            return marks.to_vec();
        }
        marks.iter().filter(|m| self.allows_mark_type(&m.mark_type)).cloned().collect()
    }
}

impl MarkType {
    /// Queries whether a given mark type is excluded by this one.
    ///
    /// Queries whether a given mark type is
    /// [excluded](#model.MarkSpec.excludes) by this one.
    pub fn excludes(&self, other: &MarkType) -> bool {
        self.excluded.excludes(&self.name, &other.name)
    }

    /// Tells you whether this mark type has any required attributes.
    pub fn has_required_attrs(&self) -> bool {
        self.spec.attrs.values().any(|a| a.is_required())
    }

    /// Compute attributes, merging provided values with spec defaults.
    ///
    /// @internal
    pub(crate) fn compute_attrs_for(
        &self,
        attrs: Option<&std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<Attrs, crate::model::errors::SchemaError> {
        if attrs.is_none() && let Some(defaults) = &self.default_attrs {
            return Ok(defaults.clone());
        }
        compute_attrs(&self.spec.attrs, attrs)
    }

    // ---- Phase 3 stubs (依赖 Mark) ----

    /// Create a mark of this type. `attrs` may be `None` or a map containing
    /// only some of the mark's attributes. The others, if they have defaults,
    /// will be added.
    ///
    /// Create a mark of this type. `attrs` may be `null` or an object
    /// containing only some of the mark's attributes. The others, if
    /// they have defaults, will be added.
    pub fn create(
        &self,
        arc_self: &Arc<Self>,
        attrs: Option<&std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<crate::model::mark::Mark, crate::model::errors::SchemaError> {
        let attrs = compute_attrs(&self.spec.attrs, attrs)?;
        Ok(crate::model::mark::Mark {
            mark_type: arc_self.clone(),
            attrs,
        })
    }

    /// When there is a mark of this type in the given set, a new set
    /// without it is returned. Otherwise, the input set is returned.
    ///
    /// When there is a mark of this type in the given set, a new set
    /// without it is returned. Otherwise, the input set is returned.
    pub fn remove_from_set(
        &self,
        set: &[crate::model::mark::Mark],
    ) -> Vec<crate::model::mark::Mark> {
        set.iter()
            .filter(|m| !std::ptr::eq(m.mark_type.as_ref(), self))
            .cloned()
            .collect()
    }

    /// Tests whether there is a mark of this type in the given set.
    ///
    /// Tests whether there is a mark of this type in the given set.
    pub fn is_in_set<'a>(
        &self,
        set: &'a [crate::model::mark::Mark],
    ) -> Option<&'a crate::model::mark::Mark> {
        set.iter()
            .find(|m| std::ptr::eq(m.mark_type.as_ref(), self))
    }
}

/// Gather MarkType instances for a list of mark name/group tokens.
///
/// Each token is either a direct mark type name or a group name that expands
/// to all marks whose spec.group contains that name.
fn gather_marks(
    tokens: &[&str],
    mark_types: &IndexMap<String, Arc<MarkType>>,
) -> Result<Vec<Arc<MarkType>>, crate::model::errors::SchemaError> {
    use crate::model::errors::SchemaError;
    let mut found: Vec<Arc<MarkType>> = vec![];
    for &token in tokens {
        if let Some(mt) = mark_types.get(token) {
            found.push(mt.clone());
        } else {
            let mut any = false;
            for mt in mark_types.values() {
                if mt
                    .spec
                    .group
                    .as_deref()
                    .is_some_and(|g| g.split_whitespace().any(|s| s == token))
                {
                    found.push(mt.clone());
                    any = true;
                }
            }
            if !any {
                return Err(SchemaError(format!("Unknown mark type: '{}'", token)));
            }
        }
    }
    Ok(found)
}

/// Resolve a NodeSpec.marks expression to a mark set.
///
/// - None + inline_content=true  → None (all marks allowed)
/// - None + inline_content=false → Some([]) (no marks)
/// - "_"                         → None (all marks allowed)
/// - ""                          → Some([]) (no marks)
/// - "em strong …"               → Some(gathered list)
fn resolve_mark_set(
    marks_expr: Option<&str>,
    mark_types: &IndexMap<String, Arc<MarkType>>,
    inline_content: bool,
) -> Result<Option<Vec<Arc<MarkType>>>, crate::model::errors::SchemaError> {
    match marks_expr {
        Some("_") => Ok(None),
        Some("") => Ok(Some(vec![])),
        Some(s) => {
            let tokens: Vec<&str> = s.split_whitespace().collect();
            Ok(Some(gather_marks(&tokens, mark_types)?))
        }
        None => {
            if inline_content {
                Ok(None)
            } else {
                Ok(Some(vec![]))
            }
        }
    }
}

/// Resolve a MarkSpec.excludes expression to an ExcludeSpec.
///
/// - None      → SelfOnly (default: a mark excludes marks of the same type)
/// - "_"       → All
/// - ""        → Named([]) (excludes nothing)
/// - "em …"    → Named(resolved type names, expanding group names)
fn resolve_excluded(
    excludes_expr: Option<&str>,
    _self_name: &str,
    mark_types: &IndexMap<String, Arc<MarkType>>,
) -> Result<ExcludeSpec, crate::model::errors::SchemaError> {
    use crate::model::errors::SchemaError;
    match excludes_expr {
        None => Ok(ExcludeSpec::SelfOnly),
        Some("_") => Ok(ExcludeSpec::All),
        Some("") => Ok(ExcludeSpec::Named(vec![])),
        Some(s) => {
            let mut names: Vec<String> = vec![];
            for token in s.split_whitespace() {
                if mark_types.contains_key(token) {
                    if !names.contains(&token.to_string()) {
                        names.push(token.to_string());
                    }
                } else {
                    let mut any = false;
                    for (mname, mt) in mark_types {
                        if mt
                            .spec
                            .group
                            .as_deref()
                            .is_some_and(|g| g.split_whitespace().any(|gr| gr == token))
                        {
                            if !names.contains(mname) {
                                names.push(mname.clone());
                            }
                            any = true;
                        }
                    }
                    if !any {
                        return Err(SchemaError(format!("Unknown mark type: '{}'", token)));
                    }
                }
            }
            Ok(ExcludeSpec::Named(names))
        }
    }
}

/// A document schema. Holds node type and mark type objects for the nodes and marks
/// that may occur in conforming documents, and provides functionality for
/// creating and deserializing such documents.
///
/// Corresponds to `Schema` in TypeScript.
pub struct Schema {
    /// An object mapping the schema's node names to node type objects.
    pub nodes: IndexMap<String, Arc<NodeType>>,
    /// A map from mark names to mark type objects.
    pub marks: IndexMap<String, Arc<MarkType>>,
    /// The type of the default top node for this schema.
    pub top_node_type: Arc<NodeType>,
    /// The linebreak replacement node defined in this schema, if any.
    pub linebreak_replacement: Option<Arc<NodeType>>,
    /// The spec on which the schema is based.
    pub spec: SchemaSpec,
}

impl fmt::Debug for Schema {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Schema")
            .field("nodes", &self.nodes.keys().collect::<Vec<_>>())
            .field("marks", &self.marks.keys().collect::<Vec<_>>())
            .field("top_node_type", &self.top_node_type.name)
            .finish()
    }
}

impl Schema {
    /// Construct a schema from a schema specification.
    ///
    /// Construct a schema from a schema [specification](#model.SchemaSpec).
    pub fn new(spec: SchemaSpec) -> Result<Arc<Self>, crate::model::errors::SchemaError> {
        use crate::model::content::ContentMatch;
        use crate::model::errors::SchemaError;

        let top_name = spec.top_node.as_deref().unwrap_or("doc");

        // --- Validation ---
        if !spec.nodes.contains_key(top_name) {
            return Err(SchemaError(format!(
                "Schema is missing its top node type ('{}')",
                top_name
            )));
        }
        if !spec.nodes.contains_key("text") {
            return Err(SchemaError("Every schema needs a 'text' type".to_string()));
        }
        if spec.nodes.get("text").is_some_and(|ns| !ns.attrs.is_empty()) {
            return Err(SchemaError(
                "The text node type should not have attributes".to_string(),
            ));
        }
        for name in spec.marks.keys() {
            if spec.nodes.contains_key(name.as_str()) {
                return Err(SchemaError(format!(
                    "{} can not be both a node and a mark",
                    name
                )));
            }
        }

        // --- First pass: create NodeType Arcs (refcount = 1) ---
        let mut node_types: IndexMap<String, Arc<NodeType>> = IndexMap::new();
        for (name, ns) in &spec.nodes {
            let is_block = !(ns.inline || name == "text");
            let groups: Vec<String> = ns
                .group
                .as_deref()
                .map(|g| g.split_whitespace().map(|s| s.to_string()).collect())
                .unwrap_or_default();
            let whitespace = ns.whitespace.unwrap_or(if ns.code {
                Whitespace::Pre
            } else {
                Whitespace::Normal
            });
            let da = default_attrs(&ns.attrs);
            node_types.insert(
                name.clone(),
                Arc::new(NodeType {
                    name: name.clone(),
                    is_block,
                    is_text: name == "text",
                    inline_content: false,
                    groups,
                    content_match: ContentMatch::empty(),
                    mark_set: None,
                    whitespace,
                    spec: ns.clone(),
                    default_attrs: da,
                }),
            );
        }

        // --- First pass: create MarkType Arcs (refcount = 1) ---
        let mut mark_types: IndexMap<String, Arc<MarkType>> = IndexMap::new();
        for (rank, (name, ms)) in spec.marks.iter().enumerate() {
            let da = default_attrs(&ms.attrs);
            mark_types.insert(
                name.clone(),
                Arc::new(MarkType {
                    name: name.clone(),
                    rank,
                    inclusive: ms.inclusive,
                    excluded: ExcludeSpec::SelfOnly,
                    spec: ms.clone(),
                    default_attrs: da,
                }),
            );
        }

        // --- Build helper maps for ContentMatch::parse ---
        let groups_map: IndexMap<String, Vec<String>> = {
            let mut map: IndexMap<String, Vec<String>> = IndexMap::new();
            for (name, nt) in &node_types {
                for g in &nt.groups {
                    map.entry(g.clone()).or_default().push(name.clone());
                }
            }
            map
        };

        // --- Second pass: fill MarkType.excluded (must happen before NodeType fills,
        //     because resolve_mark_set clones Arc<MarkType> which would raise refcount > 1) ---
        for (name, ms) in &spec.marks {
            let excluded =
                resolve_excluded(ms.excludes.as_deref(), name, &mark_types)?;
            Arc::get_mut(mark_types.get_mut(name).unwrap()).unwrap().excluded = excluded;
        }

        // --- Third pass: fill NodeType fields (refcount = 1, Arc::get_mut safe) ---
        // Step 3a: parse all content expressions while holding immutable ref to node_types
        let content_matches: IndexMap<String, Arc<crate::model::content::ContentMatch>> = {
            let mut map = IndexMap::new();
            for (name, ns) in &spec.nodes {
                let expr = ns.content.as_deref().unwrap_or("");
                let cm = ContentMatch::parse(expr, &node_types, &groups_map)
                    .map_err(|e| SchemaError(e.0))?;
                map.insert(name.clone(), cm);
            }
            map
        };
        // Step 3b: fill NodeType fields.
        //
        // We cannot use Arc::get_mut here because ContentMatch edges hold Weak<NodeType>
        // references, and Arc::get_mut returns None when any Weak pointer exists (even if
        // strong_count == 1). Since we are the sole strong owner and these Weak references
        // are not actively used during construction, we safely use raw pointer mutation.
        //
        // Safety: Arc::strong_count == 1 (only node_types holds these Arcs), so no other
        // thread or owner can access the NodeType data concurrently at this point.
        let mut linebreak_replacement: Option<Arc<NodeType>> = None;
        for (name, ns) in &spec.nodes {
            let cm = content_matches[name].clone();
            let arc_ref = node_types.get_mut(name).unwrap();
            debug_assert_eq!(Arc::strong_count(arc_ref), 1, "NodeType '{}' should have strong_count=1 during Schema construction", name);
            // SAFETY: strong_count == 1 (asserted above); Weak references exist but are not
            // being upgraded concurrently; mutation is complete before Arc is shared.
            let nt = unsafe { &mut *(Arc::as_ptr(arc_ref) as *mut NodeType) };
            nt.inline_content = cm.inline_content;
            let mark_set = resolve_mark_set(ns.marks.as_deref(), &mark_types, nt.inline_content)?;
            nt.content_match = cm;
            nt.mark_set = mark_set;
            if ns.linebreak_replacement {
                if linebreak_replacement.is_some() {
                    return Err(SchemaError(
                        "Multiple linebreak nodes defined".to_string(),
                    ));
                }
                if !nt.is_inline() || !nt.is_leaf() {
                    return Err(SchemaError(
                        "Linebreak replacement nodes must be inline leaf nodes".to_string(),
                    ));
                }
                linebreak_replacement = Some(node_types[name].clone());
            }
        }

        let top_node_type = node_types[top_name].clone();
        Ok(Arc::new(Schema {
            nodes: node_types,
            marks: mark_types,
            top_node_type,
            linebreak_replacement,
            spec,
        }))
    }

    /// Look up a node type by name.
    ///
    /// @internal
    pub fn node_type(&self, name: &str) -> Result<&Arc<NodeType>, crate::model::errors::SchemaError> {
        self.nodes.get(name).ok_or_else(|| {
            crate::model::errors::SchemaError(format!("Unknown node type: {}", name))
        })
    }

    /// Look up a mark type by name.
    ///
    /// @internal
    pub fn mark_type(&self, name: &str) -> Result<&Arc<MarkType>, crate::model::errors::SchemaError> {
        self.marks.get(name).ok_or_else(|| {
            crate::model::errors::SchemaError(format!("Unknown mark type: {}", name))
        })
    }

    // ---- Phase 3 implementations ----

    /// Create a node in this schema.
    ///
    /// Create a node in this schema. The `type` may be a string or a
    /// `NodeType` instance. Attributes will be extended with defaults,
    /// `content` may be a `Fragment`, `null`, a `Node`, or an array of
    /// nodes.
    pub fn node(
        &self,
        type_name: &str,
        attrs: Option<&std::collections::HashMap<String, serde_json::Value>>,
        content: Option<crate::model::fragment::Fragment>,
        marks: Vec<crate::model::mark::Mark>,
    ) -> Result<crate::model::node::Node, crate::model::errors::SchemaError> {
        let nt = self.node_type(type_name)?;
        nt.create(attrs, content, marks)
    }

    /// Create a text node in the schema. Empty text nodes are not allowed.
    ///
    /// Create a text node in the schema. Empty text nodes are not allowed.
    pub fn text(
        &self,
        text: String,
        marks: Vec<crate::model::mark::Mark>,
    ) -> crate::model::node::Node {
        let text_type = self.nodes["text"].clone();
        let attrs = text_type.default_attrs.clone()
            .unwrap_or_else(|| Arc::new(Default::default()));
        crate::model::node::Node::text_node(text_type, attrs, text, marks)
    }

    /// Create a mark with the given type and attributes.
    ///
    /// Create a mark with the given type and attributes.
    pub fn mark(
        &self,
        type_name: &str,
        attrs: Option<&std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<crate::model::mark::Mark, crate::model::errors::SchemaError> {
        let mt = self.mark_type(type_name)?;
        mt.create(mt, attrs)
    }

    /// Deserialize a node from its JSON representation.
    pub fn node_from_json(
        &self,
        json: &serde_json::Value,
    ) -> Result<crate::model::node::Node, crate::model::errors::RangeError> {
        crate::model::node::Node::from_json(self, json)
    }

    /// Deserialize a mark from its JSON representation.
    pub fn mark_from_json(
        &self,
        json: &serde_json::Value,
    ) -> Result<crate::model::mark::Mark, crate::model::errors::RangeError> {
        crate::model::mark::Mark::from_json(self, json)
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
        let text_arc = Arc::new(NodeType {
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
        });
        let nt_map: IndexMap<String, Arc<NodeType>> =
            [("text".to_string(), text_arc)].into_iter().collect();
        let g: IndexMap<String, Vec<String>> = IndexMap::new();
        let cm = ContentMatch::parse("text*", &nt_map, &g).unwrap();
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

    #[test]
    fn nodetype_allows_mark_type_none_means_all() {
        let p = make_paragraph_nodetype();
        let mt = Arc::new(MarkType {
            name: "em".to_string(),
            rank: 0,
            inclusive: true,
            excluded: ExcludeSpec::SelfOnly,
            spec: MarkSpec::default(),
            default_attrs: None,
        });
        // mark_set = None → all marks allowed
        assert!(p.allows_mark_type(&mt));
    }

    #[test]
    fn nodetype_allows_mark_type_restricted_set() {
        let mt_em = Arc::new(MarkType {
            name: "em".to_string(),
            rank: 0,
            inclusive: true,
            excluded: ExcludeSpec::SelfOnly,
            spec: MarkSpec::default(),
            default_attrs: None,
        });
        let mt_strong = Arc::new(MarkType {
            name: "strong".to_string(),
            rank: 1,
            inclusive: true,
            excluded: ExcludeSpec::SelfOnly,
            spec: MarkSpec::default(),
            default_attrs: None,
        });
        // mark_set = Some([em]) → only em allowed
        let p = NodeType {
            mark_set: Some(vec![mt_em.clone()]),
            ..make_paragraph_nodetype()
        };
        assert!(p.allows_mark_type(&mt_em));
        assert!(!p.allows_mark_type(&mt_strong));
    }

    #[test]
    fn nodetype_compatible_content_same_node() {
        let p = make_paragraph_nodetype();
        assert!(p.compatible_content(&p));
    }

    #[test]
    fn nodetype_compute_attrs_for_empty_spec() {
        let p = make_paragraph_nodetype(); // spec.attrs is empty
        let result = p.compute_attrs_for(None).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn nodetype_compute_attrs_for_returns_defaults() {
        use indexmap::indexmap;
        use std::collections::HashMap as StdHashMap;
        let p = NodeType {
            spec: NodeSpec {
                attrs: indexmap! {
                    "level".to_string() => AttrSpec { default: Some(json!(1)), validate: None }
                },
                ..NodeSpec::default()
            },
            default_attrs: None,
            ..make_paragraph_nodetype()
        };
        let result = p.compute_attrs_for(None).unwrap();
        assert_eq!(result.get("level"), Some(&json!(1)));
        // override
        let mut vals: StdHashMap<String, serde_json::Value> = StdHashMap::new();
        vals.insert("level".to_string(), json!(3));
        let result2 = p.compute_attrs_for(Some(&vals)).unwrap();
        assert_eq!(result2.get("level"), Some(&json!(3)));
    }

    // ---- MarkType 测试 ----

    fn make_em_marktype() -> MarkType {
        MarkType {
            name: "em".to_string(),
            rank: 0,
            inclusive: true,
            excluded: ExcludeSpec::SelfOnly,
            spec: MarkSpec::default(),
            default_attrs: None,
        }
    }

    fn make_strong_marktype() -> MarkType {
        MarkType {
            name: "strong".to_string(),
            rank: 1,
            inclusive: true,
            excluded: ExcludeSpec::SelfOnly,
            spec: MarkSpec::default(),
            default_attrs: None,
        }
    }

    #[test]
    fn marktype_excludes_self_only_same_name() {
        let em = make_em_marktype();
        assert!(em.excludes(&em));
    }

    #[test]
    fn marktype_excludes_self_only_different_name() {
        let em = make_em_marktype();
        let strong = make_strong_marktype();
        assert!(!em.excludes(&strong));
    }

    #[test]
    fn marktype_excludes_all() {
        let mt = MarkType {
            excluded: ExcludeSpec::All,
            ..make_em_marktype()
        };
        assert!(mt.excludes(&make_strong_marktype()));
        assert!(mt.excludes(&make_em_marktype()));
    }

    #[test]
    fn marktype_excludes_named_list() {
        let mt = MarkType {
            excluded: ExcludeSpec::Named(vec!["strong".to_string()]),
            ..make_em_marktype()
        };
        assert!(mt.excludes(&make_strong_marktype()));
        assert!(!mt.excludes(&make_em_marktype()));
    }

    #[test]
    fn marktype_has_required_attrs_false_when_empty() {
        let em = make_em_marktype();
        assert!(!em.has_required_attrs());
    }

    #[test]
    fn marktype_has_required_attrs_true_when_any_required() {
        use indexmap::indexmap;
        let mt = MarkType {
            spec: MarkSpec {
                attrs: indexmap! {
                    "href".to_string() => AttrSpec { default: None, validate: None }
                },
                ..MarkSpec::default()
            },
            ..make_em_marktype()
        };
        assert!(mt.has_required_attrs());
    }

    #[test]
    fn marktype_compute_attrs_for_empty_spec() {
        let em = make_em_marktype();
        let result = em.compute_attrs_for(None).unwrap();
        assert!(result.is_empty());
    }

    // ---- Schema 基础测试 ----

    fn minimal_schema_spec() -> SchemaSpec {
        use indexmap::indexmap;
        SchemaSpec {
            nodes: indexmap! {
                "doc".to_string() => NodeSpec {
                    content: Some("paragraph+".to_string()),
                    ..NodeSpec::default()
                },
                "paragraph".to_string() => NodeSpec {
                    content: Some("text*".to_string()),
                    group: Some("block".to_string()),
                    ..NodeSpec::default()
                },
                "text".to_string() => NodeSpec {
                    inline: true,
                    ..NodeSpec::default()
                },
            },
            marks: IndexMap::new(),
            top_node: None,
        }
    }

    #[test]
    fn schema_new_builds_successfully() {
        let s = Schema::new(minimal_schema_spec()).unwrap();
        assert!(s.nodes.contains_key("doc"));
        assert!(s.nodes.contains_key("paragraph"));
        assert!(s.nodes.contains_key("text"));
    }

    #[test]
    fn schema_top_node_type_is_doc() {
        let s = Schema::new(minimal_schema_spec()).unwrap();
        assert_eq!(s.top_node_type.name, "doc");
    }

    #[test]
    fn schema_error_missing_top_node() {
        let mut spec = minimal_schema_spec();
        spec.top_node = Some("missing".to_string());
        assert!(Schema::new(spec).is_err());
    }

    #[test]
    fn schema_error_missing_text_node() {
        use indexmap::indexmap;
        let spec = SchemaSpec {
            nodes: indexmap! {
                "doc".to_string() => NodeSpec { content: Some("paragraph+".to_string()), ..NodeSpec::default() },
                "paragraph".to_string() => NodeSpec::default(),
            },
            marks: IndexMap::new(),
            top_node: None,
        };
        let result = Schema::new(spec);
        assert!(result.is_err());
        assert!(result.unwrap_err().0.contains("text"));
    }

    #[test]
    fn schema_error_text_node_has_attrs() {
        use indexmap::indexmap;
        let mut spec = minimal_schema_spec();
        if let Some(ns) = spec.nodes.get_mut("text") {
            ns.attrs = indexmap! {
                "x".to_string() => AttrSpec { default: None, validate: None }
            };
        }
        assert!(Schema::new(spec).is_err());
    }

    #[test]
    fn schema_error_name_both_node_and_mark() {
        let mut spec = minimal_schema_spec();
        spec.marks.insert("paragraph".to_string(), MarkSpec::default());
        assert!(Schema::new(spec).is_err());
    }

    #[test]
    fn schema_paragraph_content_match_valid_end() {
        let s = Schema::new(minimal_schema_spec()).unwrap();
        let p = &s.nodes["paragraph"];
        assert!(p.content_match.valid_end); // "text*" can match zero texts
    }

    #[test]
    fn schema_paragraph_is_textblock() {
        let s = Schema::new(minimal_schema_spec()).unwrap();
        let p = &s.nodes["paragraph"];
        assert!(p.is_block);
        assert!(p.inline_content);
        assert!(p.is_textblock());
    }

    #[test]
    fn schema_text_is_leaf_inline() {
        let s = Schema::new(minimal_schema_spec()).unwrap();
        let t = &s.nodes["text"];
        assert!(t.is_text);
        assert!(t.is_leaf());
        assert!(t.is_inline());
    }

    #[test]
    fn schema_block_node_has_no_marks_by_default() {
        let s = Schema::new(minimal_schema_spec()).unwrap();
        let doc = &s.nodes["doc"];
        assert_eq!(doc.mark_set.as_ref().map(|v| v.len()), Some(0));
    }

    #[test]
    fn schema_inline_node_allows_all_marks_by_default() {
        let s = Schema::new(minimal_schema_spec()).unwrap();
        let p = &s.nodes["paragraph"];
        assert!(p.mark_set.is_none());
    }

    fn schema_with_marks_spec() -> SchemaSpec {
        use indexmap::indexmap;
        SchemaSpec {
            nodes: indexmap! {
                "doc".to_string() => NodeSpec {
                    content: Some("paragraph+".to_string()),
                    ..NodeSpec::default()
                },
                "paragraph".to_string() => NodeSpec {
                    content: Some("text*".to_string()),
                    marks: Some("em strong".to_string()),
                    group: Some("block".to_string()),
                    ..NodeSpec::default()
                },
                "text".to_string() => NodeSpec { inline: true, ..NodeSpec::default() },
            },
            marks: indexmap! {
                "em".to_string() => MarkSpec::default(),
                "strong".to_string() => MarkSpec::default(),
            },
            top_node: None,
        }
    }

    #[test]
    fn schema_builds_with_marks() {
        let s = Schema::new(schema_with_marks_spec()).unwrap();
        assert!(s.marks.contains_key("em"));
        assert!(s.marks.contains_key("strong"));
    }

    #[test]
    fn schema_marks_get_rank() {
        let s = Schema::new(schema_with_marks_spec()).unwrap();
        assert_eq!(s.marks["em"].rank, 0);
        assert_eq!(s.marks["strong"].rank, 1);
    }

    #[test]
    fn schema_mark_set_limited_to_specified() {
        let s = Schema::new(schema_with_marks_spec()).unwrap();
        let p = &s.nodes["paragraph"];
        let mark_set = p.mark_set.as_ref().unwrap();
        assert_eq!(mark_set.len(), 2);
        assert!(mark_set.iter().any(|m| m.name == "em"));
        assert!(mark_set.iter().any(|m| m.name == "strong"));
    }

    #[test]
    fn schema_mark_excludes_self_only_by_default() {
        let s = Schema::new(schema_with_marks_spec()).unwrap();
        let em = &s.marks["em"];
        assert!(em.excludes(em));
        assert!(!em.excludes(&s.marks["strong"]));
    }

    #[test]
    fn schema_mark_excludes_all() {
        use indexmap::indexmap;
        let spec = SchemaSpec {
            nodes: minimal_schema_spec().nodes,
            marks: indexmap! {
                "code".to_string() => MarkSpec {
                    excludes: Some("_".to_string()),
                    ..MarkSpec::default()
                },
                "em".to_string() => MarkSpec::default(),
            },
            top_node: None,
        };
        let s = Schema::new(spec).unwrap();
        let code = &s.marks["code"];
        let em = &s.marks["em"];
        assert!(code.excludes(em));
        assert!(code.excludes(code));
    }

    #[test]
    fn schema_mark_set_all_with_underscore() {
        use indexmap::indexmap;
        let spec = SchemaSpec {
            nodes: {
                let mut nodes = minimal_schema_spec().nodes;
                nodes.get_mut("paragraph").unwrap().marks = Some("_".to_string());
                nodes
            },
            marks: indexmap! {
                "em".to_string() => MarkSpec::default(),
            },
            top_node: None,
        };
        let s = Schema::new(spec).unwrap();
        let p = &s.nodes["paragraph"];
        assert!(p.mark_set.is_none());
    }

    #[test]
    fn schema_error_bad_content_expression() {
        use indexmap::indexmap;
        let spec = SchemaSpec {
            nodes: indexmap! {
                "doc".to_string() => NodeSpec {
                    content: Some("nonexistent_type+".to_string()),
                    ..NodeSpec::default()
                },
                "text".to_string() => NodeSpec { inline: true, ..NodeSpec::default() },
            },
            marks: IndexMap::new(),
            top_node: None,
        };
        assert!(Schema::new(spec).is_err());
    }

    #[test]
    fn schema_node_type_lookup_ok() {
        let s = Schema::new(minimal_schema_spec()).unwrap();
        let nt = s.node_type("paragraph").unwrap();
        assert_eq!(nt.name, "paragraph");
    }

    #[test]
    fn schema_node_type_lookup_err() {
        let s = Schema::new(minimal_schema_spec()).unwrap();
        assert!(s.node_type("nonexistent").is_err());
    }

    #[test]
    fn schema_mark_type_lookup_ok() {
        let s = Schema::new(schema_with_marks_spec()).unwrap();
        let mt = s.mark_type("em").unwrap();
        assert_eq!(mt.name, "em");
    }

    #[test]
    fn schema_mark_type_lookup_err() {
        let s = Schema::new(schema_with_marks_spec()).unwrap();
        assert!(s.mark_type("nonexistent").is_err());
    }
}

// Suppress unused import warning for fmt (only used in removed manual Debug impl)
#[allow(unused_imports)]
use fmt as _;
