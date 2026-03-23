// SPDX-License-Identifier: MIT
use crate::state::{DiffBlock, DiffLine, Hunk};
use similar::{ChangeTag, TextDiff};

/// Compute a structured DiffBlock from raw old/new content
pub fn compute_diff(file_path: &str, old: &str, new: &str) -> DiffBlock {
    let text_diff = TextDiff::from_lines(old, new);

    let mut hunks: Vec<Hunk> = Vec::new();
    let mut additions = 0usize;
    let mut deletions = 0usize;

    for group in text_diff.grouped_ops(3) {
        let mut hunk = Hunk {
            old_start: group.first().map(|op| op.old_range().start).unwrap_or(0),
            new_start: group.first().map(|op| op.new_range().start).unwrap_or(0),
            context: Vec::new(),
        };

        for op in &group {
            for change in text_diff.iter_changes(op) {
                let line = change.value().trim_end_matches('\n').to_string();
                match change.tag() {
                    ChangeTag::Equal => hunk.context.push(DiffLine::Context(line)),
                    ChangeTag::Insert => {
                        hunk.context.push(DiffLine::Add(line));
                        additions += 1;
                    },
                    ChangeTag::Delete => {
                        hunk.context.push(DiffLine::Del(line));
                        deletions += 1;
                    },
                }
            }
        }
        hunks.push(hunk);
    }

    DiffBlock {
        file_path: file_path.to_string(),
        additions,
        deletions,
        hunks,
        expanded: false,
        accepted: None,
    }
}

/// Filter: only blocks with actual changes
pub fn filter_meaningful(blocks: &[DiffBlock]) -> Vec<&DiffBlock> {
    blocks
        .iter()
        .filter(|b| b.additions + b.deletions > 0)
        .collect()
}
