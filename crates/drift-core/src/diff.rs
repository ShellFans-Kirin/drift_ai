//! Text-diff helpers.
//!
//! Wraps `similar` to produce unified-diff strings and derive before/after
//! line ranges from a diff, matching the proposal's `line_ranges_*` fields.

use similar::TextDiff;

/// Compute a unified diff between `before` and `after`.
pub fn unified_diff(before: &str, after: &str, file: &str) -> String {
    let diff = TextDiff::from_lines(before, after);
    let mut unified = diff.unified_diff();
    unified
        .header(
            &format!("a/{}", file.trim_start_matches('/')),
            &format!("b/{}", file.trim_start_matches('/')),
        )
        .context_radius(3)
        .to_string()
}

/// Walk the diff and extract the touched line ranges.
/// Returns `(ranges_before, ranges_after)` as vectors of `(start, end)`
/// 1-indexed inclusive spans.
pub fn line_ranges(before: &str, after: &str) -> (Vec<(u32, u32)>, Vec<(u32, u32)>) {
    let diff = TextDiff::from_lines(before, after);

    let mut ranges_before: Vec<(u32, u32)> = Vec::new();
    let mut ranges_after: Vec<(u32, u32)> = Vec::new();

    for group in diff.grouped_ops(0) {
        if group.is_empty() {
            continue;
        }
        let first = &group[0];
        let last = group.last().unwrap();
        let b_start = first.old_range().start as u32 + 1;
        let b_end = last.old_range().end as u32;
        let a_start = first.new_range().start as u32 + 1;
        let a_end = last.new_range().end as u32;
        if b_end >= b_start {
            ranges_before.push((b_start, b_end));
        }
        if a_end >= a_start {
            ranges_after.push((a_start, a_end));
        }
    }
    (ranges_before, ranges_after)
}

pub fn sha256_hex(content: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(content.as_bytes());
    format!("{:x}", h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_ranges_simple_insert() {
        let b = "one\ntwo\nthree\n";
        let a = "one\ntwo\ninserted\nthree\n";
        let (rb, ra) = line_ranges(b, a);
        assert!(!ra.is_empty());
        // the insertion sits after line 2; verify it's non-empty and sensible
        assert!(ra.iter().any(|&(s, e)| s <= 3 && e >= 3));
        assert!(rb.len() <= ra.len() + 1);
    }

    #[test]
    fn sha_is_deterministic() {
        assert_eq!(sha256_hex("hello"), sha256_hex("hello"));
    }
}
