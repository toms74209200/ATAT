use crate::github::issues::{GitHubIssue, IssueState};
use crate::todo::TodoItem;

#[derive(Debug, Clone, PartialEq)]
pub struct CleanCandidate {
    pub text: String,
    pub issue_number: u64,
}

impl TryFrom<&TodoItem> for CleanCandidate {
    type Error = ();

    fn try_from(item: &TodoItem) -> Result<Self, Self::Error> {
        match (item.is_checked, item.issue_number) {
            (true, Some(n)) => Ok(CleanCandidate {
                text: item.text.clone(),
                issue_number: n,
            }),
            _ => Err(()),
        }
    }
}

pub fn find_removable_items(
    candidates: &[CleanCandidate],
    issues: &[GitHubIssue],
) -> Vec<CleanCandidate> {
    candidates
        .iter()
        .filter(|candidate| {
            issues.iter().any(|issue| {
                issue.number == candidate.issue_number && issue.state == IssueState::Closed
            })
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checked(text: &str, issue_number: u64) -> TodoItem {
        TodoItem {
            text: text.to_string(),
            is_checked: true,
            issue_number: Some(issue_number),
        }
    }

    fn unchecked(text: &str, issue_number: u64) -> TodoItem {
        TodoItem {
            text: text.to_string(),
            is_checked: false,
            issue_number: Some(issue_number),
        }
    }

    fn checked_no_issue(text: &str) -> TodoItem {
        TodoItem {
            text: text.to_string(),
            is_checked: true,
            issue_number: None,
        }
    }

    fn open_issue(number: u64) -> GitHubIssue {
        GitHubIssue {
            number,
            title: String::new(),
            state: IssueState::Open,
        }
    }

    fn closed_issue(number: u64) -> GitHubIssue {
        GitHubIssue {
            number,
            title: String::new(),
            state: IssueState::Closed,
        }
    }

    #[test]
    fn test_try_from_checked_with_issue() {
        let item = checked("Task", 42);
        let candidate = CleanCandidate::try_from(&item).unwrap();
        assert_eq!(candidate.text, "Task");
        assert_eq!(candidate.issue_number, 42);
    }

    #[test]
    fn test_try_from_unchecked_is_err() {
        assert!(CleanCandidate::try_from(&unchecked("Task", 42)).is_err());
    }

    #[test]
    fn test_try_from_checked_without_issue_is_err() {
        assert!(CleanCandidate::try_from(&checked_no_issue("Task")).is_err());
    }

    #[test]
    fn test_find_removable_removes_checked_with_closed_issue() {
        let candidates = vec![CleanCandidate {
            text: "Done".to_string(),
            issue_number: 1,
        }];
        let issues = vec![closed_issue(1)];
        let result = find_removable_items(&candidates, &issues);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].issue_number, 1);
    }

    #[test]
    fn test_find_removable_keeps_candidate_with_open_issue() {
        let candidates = vec![CleanCandidate {
            text: "Still open".to_string(),
            issue_number: 1,
        }];
        let issues = vec![open_issue(1)];
        assert!(find_removable_items(&candidates, &issues).is_empty());
    }

    #[test]
    fn test_find_removable_keeps_candidate_with_no_matching_issue() {
        let candidates = vec![CleanCandidate {
            text: "Unknown".to_string(),
            issue_number: 99,
        }];
        let issues = vec![closed_issue(1)];
        assert!(find_removable_items(&candidates, &issues).is_empty());
    }

    #[test]
    fn test_find_removable_mixed_candidates() {
        let candidates = vec![
            CleanCandidate {
                text: "Remove me".to_string(),
                issue_number: 1,
            },
            CleanCandidate {
                text: "Keep me".to_string(),
                issue_number: 2,
            },
        ];
        let issues = vec![closed_issue(1), open_issue(2)];
        let result = find_removable_items(&candidates, &issues);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].issue_number, 1);
    }

    #[test]
    fn test_find_removable_empty_inputs() {
        assert!(find_removable_items(&[], &[]).is_empty());
    }
}
