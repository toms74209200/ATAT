use crate::github::issues::{GitHubIssue, IssueState};
use crate::todo::TodoItem;
use anyhow::Result;
use std::collections::HashMap;

pub fn parse_github_issues(issues_json: &[serde_json::Value]) -> Vec<GitHubIssue> {
    issues_json
        .iter()
        .filter_map(|issue| {
            if let (Some(number), Some(title), Some(state)) = (
                issue["number"].as_u64(),
                issue["title"].as_str(),
                issue["state"].as_str(),
            ) {
                if issue["pull_request"].is_null() {
                    let state = match state {
                        "open" => IssueState::Open,
                        "closed" => IssueState::Closed,
                        _ => return None,
                    };

                    Some(GitHubIssue {
                        number,
                        title: title.to_string(),
                        state,
                    })
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

pub fn fetch_github_issues<F>(repo: &str, token: &str, issue_fetcher: F) -> Result<Vec<GitHubIssue>>
where
    F: Fn(&str, &str, u32, u32) -> Result<Vec<serde_json::Value>>,
{
    let mut all_issues = Vec::new();
    let mut page = 1;
    let per_page = 100;

    loop {
        let issues_json = issue_fetcher(repo, token, page, per_page)?;

        if issues_json.is_empty() {
            break;
        }

        let parsed_issues = parse_github_issues(&issues_json);
        all_issues.extend(parsed_issues);
        page += 1;
    }

    Ok(all_issues)
}

pub fn synchronize_with_github_issues(
    todo_items: &[TodoItem],
    github_issues: &[GitHubIssue],
) -> Vec<TodoItem> {
    let github_issues_map: HashMap<u64, &GitHubIssue> = github_issues
        .iter()
        .map(|issue| (issue.number, issue))
        .collect();

    let updated_items: Vec<TodoItem> = todo_items
        .iter()
        .map(|todo_item| {
            todo_item
                .issue_number
                .and_then(|issue_number| github_issues_map.get(&issue_number))
                .filter(|github_issue| matches!(github_issue.state, IssueState::Closed))
                .filter(|_| !todo_item.is_checked)
                .map_or_else(
                    || todo_item.clone(),
                    |_| TodoItem {
                        text: todo_item.text.clone(),
                        is_checked: true,
                        issue_number: todo_item.issue_number,
                    },
                )
        })
        .collect();

    let new_items: Vec<TodoItem> = github_issues
        .iter()
        .filter(|github_issue| matches!(github_issue.state, IssueState::Open))
        .filter(|github_issue| {
            !updated_items.iter().any(|todo_item| {
                todo_item.issue_number == Some(github_issue.number)
                    || todo_item.text.trim() == github_issue.title.trim()
            })
        })
        .map(|github_issue| TodoItem {
            text: github_issue.title.clone(),
            is_checked: false,
            issue_number: Some(github_issue.number),
        })
        .collect();

    updated_items.into_iter().chain(new_items).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_github_issues_with_valid_issues() {
        let issues_json = vec![
            serde_json::json!({
                "number": 123,
                "title": "Test issue",
                "state": "open",
                "pull_request": null
            }),
            serde_json::json!({
                "number": 456,
                "title": "Closed issue",
                "state": "closed",
                "pull_request": null
            }),
        ];

        let issues = parse_github_issues(&issues_json);

        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].number, 123);
        assert_eq!(issues[0].title, "Test issue");
        assert_eq!(issues[0].state, IssueState::Open);
        assert_eq!(issues[1].number, 456);
        assert_eq!(issues[1].title, "Closed issue");
        assert_eq!(issues[1].state, IssueState::Closed);
    }

    #[test]
    fn test_parse_github_issues_filters_pull_requests() {
        let issues_json = vec![
            serde_json::json!({
                "number": 123,
                "title": "Regular issue",
                "state": "open",
                "pull_request": null
            }),
            serde_json::json!({
                "number": 456,
                "title": "Pull request",
                "state": "open",
                "pull_request": {"url": "https://api.github.com/repos/user/repo/pulls/456"}
            }),
        ];

        let issues = parse_github_issues(&issues_json);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].number, 123);
        assert_eq!(issues[0].title, "Regular issue");
    }

    #[test]
    fn test_parse_github_issues_ignores_invalid_state() {
        let issues_json = vec![
            serde_json::json!({
                "number": 123,
                "title": "Valid issue",
                "state": "open",
                "pull_request": null
            }),
            serde_json::json!({
                "number": 456,
                "title": "Invalid state",
                "state": "unknown",
                "pull_request": null
            }),
        ];

        let issues = parse_github_issues(&issues_json);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].number, 123);
    }

    #[test]
    fn test_parse_github_issues_ignores_missing_fields() {
        let issues_json = vec![
            serde_json::json!({
                "number": 123,
                "title": "Valid issue",
                "state": "open",
                "pull_request": null
            }),
            serde_json::json!({
                "title": "Missing number",
                "state": "open",
                "pull_request": null
            }),
            serde_json::json!({
                "number": 456,
                "state": "open",
                "pull_request": null
            }),
        ];

        let issues = parse_github_issues(&issues_json);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].number, 123);
    }

    #[test]
    fn test_fetch_github_issues_single_page() {
        let mock_fetcher = |_repo: &str,
                            _token: &str,
                            page: u32,
                            _per_page: u32|
         -> Result<Vec<serde_json::Value>> {
            match page {
                1 => Ok(vec![serde_json::json!({
                    "number": 123,
                    "title": "Test issue",
                    "state": "open",
                    "pull_request": null
                })]),
                _ => Ok(vec![]),
            }
        };

        let result = fetch_github_issues("user/repo", "token", mock_fetcher);

        assert!(result.is_ok());
        let issues = result.unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].number, 123);
    }

    #[test]
    fn test_fetch_github_issues_multiple_pages() {
        let mock_fetcher = |_repo: &str,
                            _token: &str,
                            page: u32,
                            _per_page: u32|
         -> Result<Vec<serde_json::Value>> {
            match page {
                1 => Ok(vec![serde_json::json!({
                    "number": 123,
                    "title": "First issue",
                    "state": "open",
                    "pull_request": null
                })]),
                2 => Ok(vec![serde_json::json!({
                    "number": 456,
                    "title": "Second issue",
                    "state": "closed",
                    "pull_request": null
                })]),
                _ => Ok(vec![]),
            }
        };

        let result = fetch_github_issues("user/repo", "token", mock_fetcher);

        assert!(result.is_ok());
        let issues = result.unwrap();
        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].number, 123);
        assert_eq!(issues[1].number, 456);
    }

    #[test]
    fn test_fetch_github_issues_empty_response() {
        let mock_fetcher = |_repo: &str,
                            _token: &str,
                            _page: u32,
                            _per_page: u32|
         -> Result<Vec<serde_json::Value>> { Ok(vec![]) };

        let result = fetch_github_issues("user/repo", "token", mock_fetcher);

        assert!(result.is_ok());
        let issues = result.unwrap();
        assert_eq!(issues.len(), 0);
    }

    #[test]
    fn test_fetch_github_issues_error_handling() {
        let mock_fetcher = |_repo: &str,
                            _token: &str,
                            _page: u32,
                            _per_page: u32|
         -> Result<Vec<serde_json::Value>> {
            Err(anyhow::anyhow!("Network error"))
        };

        let result = fetch_github_issues("user/repo", "token", mock_fetcher);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Network error"));
    }

    #[test]
    fn test_parse_github_issues_empty_array() {
        let issues_json = vec![];
        let issues = parse_github_issues(&issues_json);
        assert_eq!(issues.len(), 0);
    }

    #[test]
    fn test_parse_github_issues_all_invalid() {
        let issues_json = vec![
            serde_json::json!({}),
            serde_json::json!({"invalid": "data"}),
            serde_json::json!({"number": "not_a_number"}),
        ];
        let issues = parse_github_issues(&issues_json);
        assert_eq!(issues.len(), 0);
    }

    #[test]
    fn test_parse_github_issues_partial_valid() {
        let issues_json = vec![
            serde_json::json!({
                "number": 123,
                "title": "Valid issue",
                "state": "open",
                "pull_request": null
            }),
            serde_json::json!({
                "number": 456,
                "title": "Invalid state issue",
                "state": "invalid",
                "pull_request": null
            }),
            serde_json::json!({
                "number": 789,
                "title": "Another valid issue",
                "state": "closed",
                "pull_request": null
            }),
        ];

        let issues = parse_github_issues(&issues_json);

        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].number, 123);
        assert_eq!(issues[0].state, IssueState::Open);
        assert_eq!(issues[1].number, 789);
        assert_eq!(issues[1].state, IssueState::Closed);
    }

    #[test]
    fn test_synchronize_with_github_issues_updates_closed_issues() {
        let todo_items = vec![
            TodoItem {
                text: "Fix bug".to_string(),
                is_checked: false,
                issue_number: Some(123),
            },
            TodoItem {
                text: "Add feature".to_string(),
                is_checked: false,
                issue_number: Some(456),
            },
        ];
        let github_issues = vec![
            GitHubIssue {
                number: 123,
                title: "Fix bug".to_string(),
                state: IssueState::Closed,
            },
            GitHubIssue {
                number: 456,
                title: "Add feature".to_string(),
                state: IssueState::Open,
            },
        ];

        let result = synchronize_with_github_issues(&todo_items, &github_issues);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].text, "Fix bug");
        assert_eq!(result[0].is_checked, true);
        assert_eq!(result[0].issue_number, Some(123));
        assert_eq!(result[1].text, "Add feature");
        assert_eq!(result[1].is_checked, false);
        assert_eq!(result[1].issue_number, Some(456));
    }

    #[test]
    fn test_synchronize_with_github_issues_adds_new_open_issues() {
        let todo_items = vec![TodoItem {
            text: "Existing task".to_string(),
            is_checked: false,
            issue_number: Some(123),
        }];
        let github_issues = vec![
            GitHubIssue {
                number: 123,
                title: "Existing task".to_string(),
                state: IssueState::Open,
            },
            GitHubIssue {
                number: 456,
                title: "New task".to_string(),
                state: IssueState::Open,
            },
        ];

        let result = synchronize_with_github_issues(&todo_items, &github_issues);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].text, "Existing task");
        assert_eq!(result[0].issue_number, Some(123));
        assert_eq!(result[1].text, "New task");
        assert_eq!(result[1].is_checked, false);
        assert_eq!(result[1].issue_number, Some(456));
    }

    #[test]
    fn test_synchronize_with_github_issues_skips_already_checked() {
        let todo_items = vec![TodoItem {
            text: "Completed task".to_string(),
            is_checked: true,
            issue_number: Some(123),
        }];
        let github_issues = vec![GitHubIssue {
            number: 123,
            title: "Completed task".to_string(),
            state: IssueState::Closed,
        }];

        let result = synchronize_with_github_issues(&todo_items, &github_issues);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].text, "Completed task");
        assert_eq!(result[0].is_checked, true);
        assert_eq!(result[0].issue_number, Some(123));
    }

    #[test]
    fn test_synchronize_with_github_issues_ignores_closed_issues_for_new_todos() {
        let todo_items = vec![];
        let github_issues = vec![
            GitHubIssue {
                number: 123,
                title: "Closed issue".to_string(),
                state: IssueState::Closed,
            },
            GitHubIssue {
                number: 456,
                title: "Open issue".to_string(),
                state: IssueState::Open,
            },
        ];

        let result = synchronize_with_github_issues(&todo_items, &github_issues);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].text, "Open issue");
        assert_eq!(result[0].is_checked, false);
        assert_eq!(result[0].issue_number, Some(456));
    }

    #[test]
    fn test_synchronize_with_github_issues_preserves_todo_without_issue_number() {
        let todo_items = vec![
            TodoItem {
                text: "Local task".to_string(),
                is_checked: false,
                issue_number: None,
            },
            TodoItem {
                text: "Task with issue".to_string(),
                is_checked: false,
                issue_number: Some(123),
            },
        ];
        let github_issues = vec![GitHubIssue {
            number: 123,
            title: "Task with issue".to_string(),
            state: IssueState::Closed,
        }];

        let result = synchronize_with_github_issues(&todo_items, &github_issues);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].text, "Local task");
        assert_eq!(result[0].is_checked, false);
        assert_eq!(result[0].issue_number, None);
        assert_eq!(result[1].text, "Task with issue");
        assert_eq!(result[1].is_checked, true);
        assert_eq!(result[1].issue_number, Some(123));
    }

    #[test]
    fn test_synchronize_with_github_issues_avoids_duplicate_by_title() {
        let todo_items = vec![TodoItem {
            text: "Same title task".to_string(),
            is_checked: false,
            issue_number: None,
        }];
        let github_issues = vec![GitHubIssue {
            number: 123,
            title: "Same title task".to_string(),
            state: IssueState::Open,
        }];

        let result = synchronize_with_github_issues(&todo_items, &github_issues);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].text, "Same title task");
        assert_eq!(result[0].issue_number, None);
    }

    #[test]
    fn test_synchronize_with_github_issues_avoids_duplicate_by_title_with_trim() {
        let todo_items = vec![TodoItem {
            text: "  Task with spaces  ".to_string(),
            is_checked: false,
            issue_number: None,
        }];
        let github_issues = vec![GitHubIssue {
            number: 123,
            title: "Task with spaces".to_string(),
            state: IssueState::Open,
        }];

        let result = synchronize_with_github_issues(&todo_items, &github_issues);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].text, "  Task with spaces  ");
        assert_eq!(result[0].issue_number, None);
    }

    #[test]
    fn test_synchronize_with_github_issues_no_matching_issue() {
        let todo_items = vec![TodoItem {
            text: "Task without matching issue".to_string(),
            is_checked: false,
            issue_number: Some(999),
        }];
        let github_issues = vec![GitHubIssue {
            number: 123,
            title: "Different issue".to_string(),
            state: IssueState::Closed,
        }];

        let result = synchronize_with_github_issues(&todo_items, &github_issues);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].text, "Task without matching issue");
        assert_eq!(result[0].is_checked, false);
        assert_eq!(result[0].issue_number, Some(999));
    }

    #[test]
    fn test_synchronize_with_github_issues_empty_inputs() {
        let todo_items = vec![];
        let github_issues = vec![];

        let result = synchronize_with_github_issues(&todo_items, &github_issues);

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_synchronize_with_github_issues_complex_scenario() {
        let todo_items = vec![
            TodoItem {
                text: "To be closed".to_string(),
                is_checked: false,
                issue_number: Some(100),
            },
            TodoItem {
                text: "Already closed".to_string(),
                is_checked: true,
                issue_number: Some(200),
            },
            TodoItem {
                text: "Local only task".to_string(),
                is_checked: false,
                issue_number: None,
            },
        ];
        let github_issues = vec![
            GitHubIssue {
                number: 100,
                title: "To be closed".to_string(),
                state: IssueState::Closed,
            },
            GitHubIssue {
                number: 200,
                title: "Already closed".to_string(),
                state: IssueState::Closed,
            },
            GitHubIssue {
                number: 300,
                title: "New open issue".to_string(),
                state: IssueState::Open,
            },
            GitHubIssue {
                number: 400,
                title: "Closed new issue".to_string(),
                state: IssueState::Closed,
            },
        ];

        let result = synchronize_with_github_issues(&todo_items, &github_issues);

        assert_eq!(result.len(), 4);
        assert_eq!(result[0].text, "To be closed");
        assert_eq!(result[0].is_checked, true);
        assert_eq!(result[1].text, "Already closed");
        assert_eq!(result[1].is_checked, true);
        assert_eq!(result[2].text, "Local only task");
        assert_eq!(result[2].is_checked, false);
        assert_eq!(result[2].issue_number, None);
        assert_eq!(result[3].text, "New open issue");
        assert_eq!(result[3].is_checked, false);
        assert_eq!(result[3].issue_number, Some(300));
    }

    #[test]
    fn test_parse_github_issues_number_type_variants() {
        let issues_json = vec![
            serde_json::json!({
                "number": 123,
                "title": "Valid u64 number",
                "state": "open",
                "pull_request": null
            }),
            serde_json::json!({
                "number": "456",
                "title": "String number should be ignored",
                "state": "open",
                "pull_request": null
            }),
            serde_json::json!({
                "number": 789.5,
                "title": "Float number should be ignored",
                "state": "open",
                "pull_request": null
            }),
        ];

        let issues = parse_github_issues(&issues_json);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].number, 123);
    }
}
