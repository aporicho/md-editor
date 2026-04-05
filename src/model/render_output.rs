/// Rendering output placeholder type.
///
/// Replaces the browser DOM output types (DOMOutputSpec) from prosemirror-model.
/// This is a placeholder until the comit-text rendering layer is implemented.
/// At that point, this type will be replaced with the actual comit-text output type.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RenderOutput(pub serde_json::Value);
