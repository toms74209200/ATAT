use crate::github::issues::{GitHubIssue, IssueState};
use crate::todo::TodoItem;
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum GitHubOperation {
    CreateIssue { title: String },
    CloseIssue { number: u64 },
    RenameIssue { number: u64, title: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TitleUpdates {
    pub operations: Vec<(TodoItem, GitHubOperation)>,
    pub stale_issues: Vec<u64>,
}

pub async fn calculate_title_updates_with_history<F, Fut>(
    todo_items: &[TodoItem],
    github_issues: &[GitHubIssue],
    events_fetcher: F,
) -> Result<TitleUpdates>
where
    F: Fn(u64) -> Fut,
    Fut: std::future::Future<Output = Result<Vec<serde_json::Value>>>,
{
    let past_titles =
        crate::github::title::collect_past_titles(todo_items, github_issues, events_fetcher)
            .await?;

    Ok(calculate_title_updates(
        todo_items,
        github_issues,
        &past_titles,
    ))
}

fn calculate_title_updates(
    todo_items: &[TodoItem],
    github_issues: &[GitHubIssue],
    past_titles: &HashMap<u64, Vec<String>>,
) -> TitleUpdates {
    let github_issues_map: HashMap<u64, &GitHubIssue> = github_issues
        .iter()
        .map(|issue| (issue.number, issue))
        .collect();

    let mut operations = Vec::new();
    let mut stale_issues = Vec::new();

    for todo_item in todo_items {
        let renamed_issue = todo_item
            .issue_number
            .and_then(|issue_number| github_issues_map.get(&issue_number))
            .filter(|github_issue| matches!(github_issue.state, IssueState::Open))
            .filter(|github_issue| todo_item.text.trim() != github_issue.title.trim());

        if let Some(github_issue) = renamed_issue {
            if crate::github::title::matches_past_title(
                past_titles,
                github_issue.number,
                &todo_item.text,
            ) {
                stale_issues.push(github_issue.number);
            } else {
                operations.push((
                    todo_item.clone(),
                    GitHubOperation::RenameIssue {
                        number: github_issue.number,
                        title: todo_item.text.trim().to_string(),
                    },
                ));
            }
        }
    }

    TitleUpdates {
        operations,
        stale_issues,
    }
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
            GitHubOperation::RenameIssue { .. } => Ok((todo_item.clone(), None)),
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
    fn test_calculate_title_updates_renames_locally_edited_title() {
        let todo_items = vec![TodoItem {
            text: "Locally edited title".to_string(),
            is_checked: false,
            issue_number: Some(123),
        }];
        let github_issues = vec![GitHubIssue {
            number: 123,
            title: "Original title".to_string(),
            state: IssueState::Open,
        }];
        let past_titles = HashMap::from([(123u64, vec![])]);

        let updates = calculate_title_updates(&todo_items, &github_issues, &past_titles);

        assert_eq!(
            updates,
            TitleUpdates {
                operations: vec![(
                    todo_items[0].clone(),
                    GitHubOperation::RenameIssue {
                        number: 123,
                        title: "Locally edited title".to_string(),
                    },
                )],
                stale_issues: vec![],
            }
        );
    }

    #[test]
    fn test_calculate_title_updates_skips_stale_local_text() {
        let todo_items = vec![TodoItem {
            text: "Old title".to_string(),
            is_checked: false,
            issue_number: Some(123),
        }];
        let github_issues = vec![GitHubIssue {
            number: 123,
            title: "New title".to_string(),
            state: IssueState::Open,
        }];
        let past_titles = HashMap::from([(123u64, vec!["Old title".to_string()])]);

        let updates = calculate_title_updates(&todo_items, &github_issues, &past_titles);

        assert_eq!(
            updates,
            TitleUpdates {
                operations: vec![],
                stale_issues: vec![123],
            }
        );
    }

    #[test]
    fn test_calculate_title_updates_ignores_synced_closed_and_unnumbered_items() {
        let todo_items = vec![
            TodoItem {
                text: "Same title".to_string(),
                is_checked: false,
                issue_number: Some(123),
            },
            TodoItem {
                text: "Edited closed title".to_string(),
                is_checked: true,
                issue_number: Some(456),
            },
            TodoItem {
                text: "Local task".to_string(),
                is_checked: false,
                issue_number: None,
            },
        ];
        let github_issues = vec![
            GitHubIssue {
                number: 123,
                title: "Same title".to_string(),
                state: IssueState::Open,
            },
            GitHubIssue {
                number: 456,
                title: "Closed title".to_string(),
                state: IssueState::Closed,
            },
        ];
        let past_titles = HashMap::new();

        let updates = calculate_title_updates(&todo_items, &github_issues, &past_titles);

        assert_eq!(
            updates,
            TitleUpdates {
                operations: vec![],
                stale_issues: vec![],
            }
        );
    }

    #[test]
    fn test_calculate_title_updates_trims_title_for_rename() {
        let todo_items = vec![TodoItem {
            text: "  Edited title  ".to_string(),
            is_checked: false,
            issue_number: Some(123),
        }];
        let github_issues = vec![GitHubIssue {
            number: 123,
            title: "Original title".to_string(),
            state: IssueState::Open,
        }];
        let past_titles = HashMap::new();

        let updates = calculate_title_updates(&todo_items, &github_issues, &past_titles);

        assert_eq!(
            updates.operations,
            vec![(
                todo_items[0].clone(),
                GitHubOperation::RenameIssue {
                    number: 123,
                    title: "Edited title".to_string(),
                },
            )]
        );
    }

    #[tokio::test]
    async fn test_calculate_title_updates_with_history_resolves_by_rename_history() {
        let todo_items = vec![
            TodoItem {
                text: "Locally edited title".to_string(),
                is_checked: false,
                issue_number: Some(123),
            },
            TodoItem {
                text: "Old title".to_string(),
                is_checked: false,
                issue_number: Some(456),
            },
        ];
        let github_issues = vec![
            GitHubIssue {
                number: 123,
                title: "Original title".to_string(),
                state: IssueState::Open,
            },
            GitHubIssue {
                number: 456,
                title: "New title".to_string(),
                state: IssueState::Open,
            },
        ];
        let events_fetcher = |issue_number: u64| async move {
            match issue_number {
                123 => Ok(vec![]),
                456 => Ok(vec![serde_json::json!({
                    "event": "renamed",
                    "rename": {"from": "Old title", "to": "New title"}
                })]),
                _ => Err(anyhow::anyhow!(
                    "history should not be fetched for issue #{issue_number}"
                )),
            }
        };

        let result =
            calculate_title_updates_with_history(&todo_items, &github_issues, events_fetcher).await;

        assert!(result.is_ok());
        let updates = result.unwrap();
        assert_eq!(
            updates.operations,
            vec![(
                todo_items[0].clone(),
                GitHubOperation::RenameIssue {
                    number: 123,
                    title: "Locally edited title".to_string(),
                },
            )]
        );
        assert_eq!(updates.stale_issues, vec![456]);
    }

    #[test]
    fn test_rename_issue_operation_updates_nothing_in_todo() {
        let todo_item = TodoItem {
            text: "Edited title".to_string(),
            is_checked: false,
            issue_number: Some(123),
        };
        let github_operations = vec![(
            todo_item.clone(),
            GitHubOperation::RenameIssue {
                number: 123,
                title: "Edited title".to_string(),
            },
        )];

        let mock_creator = |_title: &str| -> Result<u64> { panic!("creator should not be called") };
        let mock_closer = |_number: u64| -> Result<()> { panic!("closer should not be called") };

        let updates =
            calculate_todo_updates(&github_operations, mock_creator, mock_closer).unwrap();

        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].1, None);
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
