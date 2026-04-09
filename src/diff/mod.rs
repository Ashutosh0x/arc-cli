// ARC CLI — Real Diff Engine using `similar` crate
// Produces semantic line-by-line diffs, not fake UI.

use similar::{ChangeTag, TextDiff};

use crate::models::{DiffLine, DiffResult};

/// Compute a real line-by-line diff between old and new content.
pub fn compute_diff(file_path: &str, old: &str, new: &str) -> DiffResult {
    let text_diff = TextDiff::from_lines(old, new);

    let mut lines = Vec::new();
    let mut additions: usize = 0;
    let mut deletions: usize = 0;

    for change in text_diff.iter_all_changes() {
        let content = change.to_string();
        match change.tag() {
            ChangeTag::Insert => {
                additions += 1;
                lines.push(DiffLine::Added(content));
            }
            ChangeTag::Delete => {
                deletions += 1;
                lines.push(DiffLine::Removed(content));
            }
            ChangeTag::Equal => {
                lines.push(DiffLine::Unchanged(content));
            }
        }
    }

    DiffResult {
        file_path: file_path.to_string(),
        old_content: old.to_string(),
        new_content: new.to_string(),
        lines,
        additions,
        deletions,
    }
}

/// Format a diff result as a unified diff string (for display / export).
pub fn format_unified(diff: &DiffResult) -> String {
    let text_diff = TextDiff::from_lines(&diff.old_content, &diff.new_content);
    text_diff
        .unified_diff()
        .header(&format!("a/{}", diff.file_path), &format!("b/{}", diff.file_path))
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_diff() {
        let old = "fn main() {\n    println!(\"hello\");\n}\n";
        let new = "fn main() {\n    println!(\"world\");\n    return;\n}\n";

        let result = compute_diff("main.rs", old, new);
        assert_eq!(result.additions, 2);
        assert_eq!(result.deletions, 1);
        assert!(!result.lines.is_empty());
    }

    #[test]
    fn test_no_changes() {
        let content = "line1\nline2\n";
        let result = compute_diff("test.rs", content, content);
        assert_eq!(result.additions, 0);
        assert_eq!(result.deletions, 0);
    }
}
