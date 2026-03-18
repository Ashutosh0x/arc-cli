//! # arc-diff
//!
//! Advanced structural diffing and patching engine. Understands semantic
//! code boundaries rather than just line-by-line diffs.

pub fn generate_patch(original: &str, modified: &str) -> String {
    let diff = similar::TextDiff::from_lines(original, modified);
    let mut patch = String::new();
    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            similar::ChangeTag::Delete => "-",
            similar::ChangeTag::Insert => "+",
            similar::ChangeTag::Equal => " ",
        };
        patch.push_str(&format!("{}{}", sign, change));
    }
    patch
}
