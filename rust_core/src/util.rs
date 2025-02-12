// helper utility functions

use std::fmt::Display;

// convert any displayable value to a string
pub fn as_str<T: Display>(value: T) -> String {
    value.to_string()
}

// compute median from a slice of f64 values (used for data period calculations)
pub fn data_period(diffs: &[f64]) -> Option<f64> {
    let mut sorted = diffs.to_vec();
    if sorted.is_empty() {
        return None;
    }
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let len = sorted.len();
    if len % 2 == 0 {
        Some((sorted[len/2 - 1] + sorted[len/2]) / 2.0)
    } else {
        Some(sorted[len/2])
    }
}