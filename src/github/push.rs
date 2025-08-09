use crate::github::issues::{GitHubIssue, IssueState};
use crate::todo::TodoItem;
use anyhow::Result;

#[derive(Debug, Clone, PartialEq)]
pub enum GitHubOperation {
    CreateIssue { title: String },
    CloseIssue { number: u64 },
}

pub fn calculate_github_operations(
    todo_items: &[TodoItem],
    github_issues: &[GitHubIssue],
) -> Vec<(TodoItem, GitHubOperation)> {
    todo_items
        .iter()
        .filter_map(|todo| {
            let operation = match (todo.is_checked, todo.issue_number) {
                (false, None) => Some(GitHubOperation::CreateIssue {
                    title: todo.text.clone(),
                }),
                (true, Some(issue_num)) => {
                    match github_issues.iter().find(|issue| issue.number == issue_num) {
                        Some(github_issue) if github_issue.state == IssueState::Open => {
                            Some(GitHubOperation::CloseIssue { number: issue_num })
                        }
                        _ => None,
                    }
                }
                _ => None,
            };
            operation.map(|op| (todo.clone(), op))
        })
        .collect()
}

pub fn calculate_todo_updates<F, G>(
    github_operations: &[(TodoItem, GitHubOperation)],
    issue_creator: F,
    issue_closer: G,
) -> Result<Vec<(TodoItem, Option<u64>)>>
where
    F: Fn(&str) -> Result<u64>,
    G: Fn(u64) -> Result<()>,
{
    github_operations
        .iter()
        .map(|(todo_item, operation)| match operation {
            GitHubOperation::CreateIssue { title } => {
                let issue_number = issue_creator(title)?;
                Ok((todo_item.clone(), Some(issue_number)))
            }
            GitHubOperation::CloseIssue { number } => {
                issue_closer(*number)?;
                Ok((todo_item.clone(), None))
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unchecked_no_issue_creates_issue() {
        let todo_items = vec![TodoItem {
            text: "New task".to_string(),
            is_checked: false,
            issue_number: None,
        }];
        let github_issues = vec![];

        let operations = calculate_github_operations(&todo_items, &github_issues);

        assert_eq!(operations.len(), 1);
        assert_eq!(
            operations[0].1,
            GitHubOperation::CreateIssue {
                title: "New task".to_string()
            }
        );
        assert_eq!(operations[0].0, todo_items[0]);
    }

    #[test]
    fn test_checked_with_open_issue_closes_issue() {
        let todo_items = vec![TodoItem {
            text: "Completed task".to_string(),
            is_checked: true,
            issue_number: Some(123),
        }];
        let github_issues = vec![GitHubIssue {
            number: 123,
            title: "Completed task".to_string(),
            state: IssueState::Open,
        }];

        let operations = calculate_github_operations(&todo_items, &github_issues);

        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].1, GitHubOperation::CloseIssue { number: 123 });
        assert_eq!(operations[0].0, todo_items[0]);
    }

    #[test]
    fn test_checked_with_closed_issue_no_operation() {
        let todo_items = vec![TodoItem {
            text: "Already closed task".to_string(),
            is_checked: true,
            issue_number: Some(123),
        }];
        let github_issues = vec![GitHubIssue {
            number: 123,
            title: "Already closed task".to_string(),
            state: IssueState::Closed,
        }];

        let operations = calculate_github_operations(&todo_items, &github_issues);

        assert_eq!(operations.len(), 0);
    }
    #[test]
    fn test_checked_with_nonexistent_issue_no_operation() {
        let todo_items = vec![TodoItem {
            text: "Task with missing issue".to_string(),
            is_checked: true,
            issue_number: Some(999),
        }];
        let github_issues = vec![GitHubIssue {
            number: 123,
            title: "Different issue".to_string(),
            state: IssueState::Open,
        }];

        let operations = calculate_github_operations(&todo_items, &github_issues);

        assert_eq!(operations.len(), 0);
    }

    #[test]
    fn test_unchecked_with_existing_issue_no_operation() {
        let todo_items = vec![TodoItem {
            text: "Unchecked with issue".to_string(),
            is_checked: false,
            issue_number: Some(456),
        }];
        let github_issues = vec![GitHubIssue {
            number: 456,
            title: "Existing issue".to_string(),
            state: IssueState::Open,
        }];

        let operations = calculate_github_operations(&todo_items, &github_issues);

        assert_eq!(operations.len(), 0);
    }
    #[test]
    fn test_checked_without_issue_no_operation() {
        let todo_items = vec![TodoItem {
            text: "Checked but no issue".to_string(),
            is_checked: true,
            issue_number: None,
        }];
        let github_issues = vec![];

        let operations = calculate_github_operations(&todo_items, &github_issues);

        assert_eq!(operations.len(), 0);
    }

    #[test]
    fn test_create_issue_operation_calls_creator() {
        let todo_item = TodoItem {
            text: "New task".to_string(),
            is_checked: false,
            issue_number: None,
        };
        let github_operations = vec![(
            todo_item.clone(),
            GitHubOperation::CreateIssue {
                title: "New task".to_string(),
            },
        )];

        let mock_creator = |title: &str| -> Result<u64> {
            assert_eq!(title, "New task");
            Ok(789)
        };
        let mock_closer = |_number: u64| -> Result<()> { Ok(()) };

        let updates =
            calculate_todo_updates(&github_operations, mock_creator, mock_closer).unwrap();

        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].0.text, "New task");
        assert_eq!(updates[0].1, Some(789));
    }

    #[test]
    fn test_close_issue_operation_calls_closer() {
        let todo_item = TodoItem {
            text: "Completed task".to_string(),
            is_checked: true,
            issue_number: Some(123),
        };
        let github_operations = vec![(
            todo_item.clone(),
            GitHubOperation::CloseIssue { number: 123 },
        )];

        let mock_creator = |_title: &str| -> Result<u64> { Ok(0) };
        let mock_closer = |number: u64| -> Result<()> {
            assert_eq!(number, 123);
            Ok(())
        };

        let updates =
            calculate_todo_updates(&github_operations, mock_creator, mock_closer).unwrap();

        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].0.text, "Completed task");
        assert_eq!(updates[0].1, None);
    }
}
