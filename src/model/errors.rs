#![allow(dead_code)] // TODO: Phase 2 完成后移除

use std::fmt;

/// Schema 构建或节点校验失败时抛出的错误。
#[derive(Debug, Clone, PartialEq)]

pub struct SchemaError(pub String);

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SchemaError: {}", self.0)
    }
}

impl std::error::Error for SchemaError {}

/// Node::replace 操作失败时抛出的错误。
#[derive(Debug, Clone, PartialEq)]

pub struct ReplaceError(pub String);

impl fmt::Display for ReplaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ReplaceError: {}", self.0)
    }
}

impl std::error::Error for ReplaceError {}

/// ContentMatch::parse 解析内容表达式失败时抛出的错误。
#[derive(Debug, Clone, PartialEq)]

pub struct SyntaxError(pub String);

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SyntaxError: {}", self.0)
    }
}

impl std::error::Error for SyntaxError {}

/// 位置越界等范围错误。
#[derive(Debug, Clone, PartialEq)]

pub struct RangeError(pub String);

impl fmt::Display for RangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RangeError: {}", self.0)
    }
}

impl std::error::Error for RangeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_error_display() {
        let e = SchemaError("bad schema".to_string());
        assert_eq!(e.to_string(), "SchemaError: bad schema");
    }

    #[test]
    fn replace_error_display() {
        let e = ReplaceError("bad replace".to_string());
        assert_eq!(e.to_string(), "ReplaceError: bad replace");
    }

    #[test]
    fn syntax_error_display() {
        let e = SyntaxError("bad expr".to_string());
        assert_eq!(e.to_string(), "SyntaxError: bad expr");
    }

    #[test]
    fn range_error_display() {
        let e = RangeError("out of range".to_string());
        assert_eq!(e.to_string(), "RangeError: out of range");
    }

    #[test]
    fn errors_implement_std_error() {
        let e: Box<dyn std::error::Error> = Box::new(SchemaError("x".to_string()));
        assert_eq!(e.to_string(), "SchemaError: x");
    }
}
