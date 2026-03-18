#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TaskType {
    Coding,
    Reasoning,
    QuickFix,
    Explanation,
    General,
}

pub struct TaskClassifier;

impl TaskClassifier {
    pub fn classify(prompt: &str) -> TaskType {
        let p = prompt.to_lowercase();
        if p.contains("refactor") || p.contains("implement") || p.contains("code") {
            TaskType::Coding
        } else if p.contains("why") || p.contains("explain") || p.contains("how") {
            TaskType::Explanation
        } else if p.contains("fix") || p.contains("bug") || p.contains("error") {
            TaskType::QuickFix
        } else if p.contains("think") || p.contains("plan") || p.contains("architecture") {
            TaskType::Reasoning
        } else {
            TaskType::General
        }
    }
}
