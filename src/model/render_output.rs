/// Rendering output placeholder type.
///
/// Replaces the browser DOM output types (DOMOutputSpec) from prosemirror-model.
/// This is a placeholder until the cosmic-text rendering layer is implemented.
/// At that point, this type will be replaced with the actual cosmic-text output type.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum RenderOutput {
    /// Placeholder variant — will be replaced once the cosmic-text layer is defined.
    Pending,
}
