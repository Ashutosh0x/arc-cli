//! Property tests for hook event matching.

use proptest::prelude::*;
use regex::Regex;

fn matches_event(
    hook_event: &str,
    hook_pattern: Option<&str>,
    actual_event: &str,
    actual_tool: Option<&str>,
) -> bool {
    if hook_event != actual_event {
        return false;
    }

    match (hook_pattern, actual_tool) {
        (Some(pattern), Some(tool)) => {
            Regex::new(pattern)
                .map(|re| re.is_match(tool))
                .unwrap_or(false)
        }
        (Some(_), None) => false,
        (None, _) => true,
    }
}

proptest! {
    #[test]
    fn same_event_no_pattern_always_matches(
        event in "[A-Z][a-zA-Z]{2,20}"
    ) {
        prop_assert!(matches_event(&event, None, &event, None));
        prop_assert!(matches_event(&event, None, &event, Some("any_tool")));
    }

    #[test]
    fn different_event_never_matches(
        event1 in "[A-Z][a-zA-Z]{2,10}",
        event2 in "[A-Z][a-zA-Z]{2,10}"
    ) {
        prop_assume!(event1 != event2);
        prop_assert!(!matches_event(&event1, None, &event2, None));
    }

    #[test]
    fn exact_tool_pattern_matches_exact_tool(
        event in "[A-Z][a-zA-Z]{2,10}",
        tool in "[a-z_]{2,15}"
    ) {
        let pattern = format!("^{tool}$");
        prop_assert!(matches_event(&event, Some(&pattern), &event, Some(&tool)));
    }

    #[test]
    fn exact_tool_pattern_rejects_other_tools(
        event in "[A-Z][a-zA-Z]{2,10}",
        tool1 in "[a-z]{2,10}",
        tool2 in "[a-z]{2,10}"
    ) {
        prop_assume!(tool1 != tool2);
        let pattern = format!("^{tool1}$");
        prop_assert!(!matches_event(&event, Some(&pattern), &event, Some(&tool2)));
    }
}
