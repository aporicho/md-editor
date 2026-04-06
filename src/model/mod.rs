// prosemirror-model Rust 重设计
// 参见 docs/superpowers/specs/rust-prosemirror-porting-guide.md

pub mod errors;
pub mod compare_deep;
pub mod render_output;
pub mod schema;
pub mod content;
pub mod mark;
pub mod fragment;
pub mod node;
pub mod diff;
pub mod replace;
pub mod resolved_pos;
pub mod dom_parser;
pub mod dom_serializer;

#[cfg(test)]
mod tests_mark;
#[cfg(test)]
mod tests_diff;
#[cfg(test)]
pub(crate) mod test_builder;
#[cfg(test)]
mod tests_node;
#[cfg(test)]
mod tests_content;
#[cfg(test)]
mod tests_slice;
#[cfg(test)]
mod tests_resolve;
#[cfg(test)]
mod tests_replace;

// Public re-exports — model 模块公开 API
pub use node::Node;
pub use fragment::Fragment;
pub use mark::Mark;
pub use replace::{Slice, replace};
pub use resolved_pos::{ResolvedPos, NodeRange};
pub use schema::{
    Schema, NodeType, MarkType, NodeSpec, MarkSpec, AttrSpec, SchemaSpec,
    Attrs, Whitespace, ExcludeSpec, AttrValidator,
};
pub use content::ContentMatch;
pub use render_output::RenderOutput;
pub use errors::{SchemaError, ReplaceError, SyntaxError, RangeError};
pub use dom_parser::{DOMParser, ParseOptions};
pub use dom_serializer::DOMSerializer;
