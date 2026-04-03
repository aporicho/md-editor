// comparedeep.ts 在 Rust 中不需要。
// ProseMirror 用 compareDeep 递归比较 JS 对象，
// Rust 通过 #[derive(PartialEq)] 自动实现，无需手写。
//
// 此文件仅作为与 prosemirror-model/src/comparedeep.ts 的对照占位。
