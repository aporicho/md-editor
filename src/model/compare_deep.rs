#![allow(dead_code)] // TODO: Phase 2 完成后移除

use serde_json::Value;

/// Recursively compare two JSON values for deep equality.
#[allow(dead_code)]
pub fn compare_deep(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Number(a), Value::Number(b)) => a == b,
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| compare_deep(x, y))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter().all(|(k, v)| b.get(k).is_some_and(|bv| compare_deep(v, bv)))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn equal_primitives() {
        assert!(compare_deep(&json!(1), &json!(1)));
        assert!(compare_deep(&json!("a"), &json!("a")));
        assert!(compare_deep(&json!(true), &json!(true)));
        assert!(compare_deep(&json!(null), &json!(null)));
    }

    #[test]
    fn unequal_primitives() {
        assert!(!compare_deep(&json!(1), &json!(2)));
        assert!(!compare_deep(&json!("a"), &json!("b")));
        assert!(!compare_deep(&json!(true), &json!(false)));
    }

    #[test]
    fn type_mismatch() {
        assert!(!compare_deep(&json!(1), &json!("1")));
        assert!(!compare_deep(&json!(null), &json!(false)));
        assert!(!compare_deep(&json!(1), &json!(null)));
    }

    #[test]
    fn equal_objects() {
        assert!(compare_deep(&json!({"a": 1, "b": 2}), &json!({"a": 1, "b": 2})));
    }

    #[test]
    fn unequal_objects() {
        assert!(!compare_deep(&json!({"a": 1}), &json!({"a": 2})));
        assert!(!compare_deep(&json!({"a": 1}), &json!({"b": 1})));
        assert!(!compare_deep(&json!({"a": 1}), &json!({"a": 1, "b": 2})));
    }

    #[test]
    fn equal_arrays() {
        assert!(compare_deep(&json!([1, 2, 3]), &json!([1, 2, 3])));
        assert!(compare_deep(&json!([]), &json!([])));
    }

    #[test]
    fn unequal_arrays() {
        assert!(!compare_deep(&json!([1, 2]), &json!([1, 3])));
        assert!(!compare_deep(&json!([1, 2]), &json!([1, 2, 3])));
    }

    #[test]
    fn nested() {
        assert!(compare_deep(
            &json!({"a": [1, {"b": 2}]}),
            &json!({"a": [1, {"b": 2}]})
        ));
        assert!(!compare_deep(
            &json!({"a": [1, {"b": 2}]}),
            &json!({"a": [1, {"b": 3}]})
        ));
    }
}
