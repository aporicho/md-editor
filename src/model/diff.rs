// src/model/diff.rs
#![allow(dead_code)]
use crate::model::fragment::Fragment;

/// Find the first position at which fragment `a` and fragment `b` differ, or `None` if equal.
///
/// NOTE: stub — implement in Task 4.
pub fn find_diff_start(_a: &Fragment, _b: &Fragment, _pos: usize) -> Option<usize> { todo!() }

/// Find the first position from the end at which `a` and `b` differ. Returns `(pos_a, pos_b)` or `None`.
///
/// NOTE: stub — implement in Task 4.
pub fn find_diff_end(_a: &Fragment, _b: &Fragment, _pos_a: usize, _pos_b: usize) -> Option<(usize, usize)> { todo!() }
