// SPDX-License-Identifier: MIT
//! Diff Context Snippet Generator
//!
//! Shows head+tail around changed lines with merged ranges.

pub fn get_diff_context_snippet(original: &str, new_content: &str, context_lines: usize) -> String {
    if original.is_empty() {
        return new_content.to_string();
    }

    let old_lines: Vec<&str> = original.lines().collect();
    let new_lines: Vec<&str> = new_content.lines().collect();

    // Find changed line ranges using a simple diff
    let mut changed_ranges: Vec<(usize, usize)> = Vec::new();
    let max_len = old_lines.len().max(new_lines.len());

    let mut i = 0;
    while i < max_len {
        let old_line = old_lines.get(i).copied().unwrap_or("");
        let new_line = new_lines.get(i).copied().unwrap_or("");
        if old_line != new_line {
            let start = i;
            while i < max_len {
                let ol = old_lines.get(i).copied().unwrap_or("");
                let nl = new_lines.get(i).copied().unwrap_or("");
                if ol == nl {
                    break;
                }
                i += 1;
            }
            changed_ranges.push((start, i));
        } else {
            i += 1;
        }
    }

    if changed_ranges.is_empty() {
        return new_content.to_string();
    }

    // Expand ranges with context
    let mut expanded: Vec<(usize, usize)> = changed_ranges
        .iter()
        .map(|(start, end)| {
            let ctx_start = start.saturating_sub(context_lines);
            let ctx_end = (*end + context_lines).min(new_lines.len());
            (ctx_start, ctx_end)
        })
        .collect();

    // Sort and merge overlapping ranges
    expanded.sort_by_key(|r| r.0);
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for range in expanded {
        if let Some(last) = merged.last_mut() {
            if range.0 <= last.1 {
                last.1 = last.1.max(range.1);
                continue;
            }
        }
        merged.push(range);
    }

    // Build output
    let mut output = Vec::new();
    let mut last_end = 0;
    for (start, end) in &merged {
        if *start > last_end {
            output.push("...".to_string());
        }
        for line in &new_lines[*start..*end] {
            output.push(line.to_string());
        }
        last_end = *end;
    }
    if last_end < new_lines.len() {
        output.push("...".to_string());
    }

    output.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_changes() {
        let content = "line1\nline2\nline3";
        assert_eq!(get_diff_context_snippet(content, content, 2), content);
    }

    #[test]
    fn test_empty_original() {
        let new = "new content";
        assert_eq!(get_diff_context_snippet("", new, 2), new);
    }

    #[test]
    fn test_single_change() {
        let old = "a\nb\nc\nd\ne\nf\ng\nh";
        let new = "a\nb\nc\nX\ne\nf\ng\nh";
        let result = get_diff_context_snippet(old, new, 1);
        assert!(result.contains("X"));
        assert!(result.contains("..."));
    }
}
