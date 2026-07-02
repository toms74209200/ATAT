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

#[derive(Debug, Clone, PartialEq)]
pub struct TitleSynchronization {
    pub items: Vec<TodoItem>,
    pub locally_edited_issues: Vec<u64>,
}

pub async fn synchronize_titles_with_history<F, Fut>(
    todo_items: &[TodoItem],
    github_issues: &[GitHubIssue],
    events_fetcher: F,
) -> Result<TitleSynchronization>
where
    F: Fn(u64) -> Fut,
    Fut: std::future::Future<Output = Result<Vec<serde_json::Value>>>,
{
    let mut past_titles = HashMap::new();
    for issue_number in find_title_mismatches(todo_items, github_issues) {
        let events = events_fetcher(issue_number).await?;
        past_titles.insert(issue_number, parse_past_titles(&events));
    }

    let (items, locally_edited_issues) =
        synchronize_titles(todo_items, github_issues, &past_titles);

    Ok(TitleSynchronization {
        items,
        locally_edited_issues,
    })
}

fn parse_past_titles(events_json: &[serde_json::Value]) -> Vec<String> {
    events_json
        .iter()
        .filter(|event| event["event"].as_str() == Some("renamed"))
        .filter_map(|event| event["rename"]["from"].as_str())
        .map(str::to_string)
        .collect()
}

fn find_title_mismatches(todo_items: &[TodoItem], github_issues: &[GitHubIssue]) -> Vec<u64> {
    let github_issues_map: HashMap<u64, &GitHubIssue> = github_issues
        .iter()
        .map(|issue| (issue.number, issue))
        .collect();

    todo_items
        .iter()
        .filter_map(|todo_item| {
            todo_item
                .issue_number
                .and_then(|issue_number| github_issues_map.get(&issue_number))
                .filter(|github_issue| matches!(github_issue.state, IssueState::Open))
                .filter(|github_issue| todo_item.text.trim() != github_issue.title.trim())
                .map(|github_issue| github_issue.number)
        })
        .collect()
}

fn synchronize_titles(
    todo_items: &[TodoItem],
    github_issues: &[GitHubIssue],
    past_titles: &HashMap<u64, Vec<String>>,
) -> (Vec<TodoItem>, Vec<u64>) {
    let github_issues_map: HashMap<u64, &GitHubIssue> = github_issues
        .iter()
        .map(|issue| (issue.number, issue))
        .collect();

    let mut local_edits = Vec::new();

    let updated_items = todo_items
        .iter()
        .map(|todo_item| {
            let renamed_issue = todo_item
                .issue_number
                .and_then(|issue_number| github_issues_map.get(&issue_number))
                .filter(|github_issue| matches!(github_issue.state, IssueState::Open))
                .filter(|github_issue| todo_item.text.trim() != github_issue.title.trim());

            match renamed_issue {
                None => todo_item.clone(),
                Some(github_issue) => {
                    let is_stale_local_text =
                        past_titles.get(&github_issue.number).is_some_and(|titles| {
                            titles
                                .iter()
                                .any(|title| title.trim() == todo_item.text.trim())
                        });

                    if is_stale_local_text {
                        TodoItem {
                            text: github_issue.title.clone(),
                            is_checked: todo_item.is_checked,
                            issue_number: todo_item.issue_number,
                        }
                    } else {
                        local_edits.push(github_issue.number);
                        todo_item.clone()
                    }
                }
            }
        })
        .collect();

    (updated_items, local_edits)
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
    fn test_parse_past_titles_extracts_renamed_events() {
        let events_json = vec![
            serde_json::json!({
                "event": "renamed",
                "rename": {"from": "First title", "to": "Second title"}
            }),
            serde_json::json!({
                "event": "labeled",
                "label": {"name": "bug"}
            }),
            serde_json::json!({
                "event": "renamed",
                "rename": {"from": "Second title", "to": "Third title"}
            }),
        ];

        let past_titles = parse_past_titles(&events_json);

        assert_eq!(past_titles, vec!["First title", "Second title"]);
    }

    #[test]
    fn test_parse_past_titles_empty_events() {
        let past_titles = parse_past_titles(&[]);
        assert!(past_titles.is_empty());
    }

    #[test]
    fn test_parse_past_titles_ignores_malformed_rename() {
        let events_json = vec![
            serde_json::json!({"event": "renamed"}),
            serde_json::json!({"event": "renamed", "rename": {"to": "No from"}}),
        ];

        let past_titles = parse_past_titles(&events_json);

        assert!(past_titles.is_empty());
    }

    #[test]
    fn test_find_title_mismatches_detects_open_issue_with_different_title() {
        let todo_items = vec![
            TodoItem {
                text: "Old title".to_string(),
                is_checked: false,
                issue_number: Some(123),
            },
            TodoItem {
                text: "Same title".to_string(),
                is_checked: false,
                issue_number: Some(456),
            },
        ];
        let github_issues = vec![
            GitHubIssue {
                number: 123,
                title: "New title".to_string(),
                state: IssueState::Open,
            },
            GitHubIssue {
                number: 456,
                title: "Same title".to_string(),
                state: IssueState::Open,
            },
        ];

        let mismatches = find_title_mismatches(&todo_items, &github_issues);

        assert_eq!(mismatches, vec![123]);
    }

    #[test]
    fn test_find_title_mismatches_ignores_closed_issues() {
        let todo_items = vec![TodoItem {
            text: "Old title".to_string(),
            is_checked: false,
            issue_number: Some(123),
        }];
        let github_issues = vec![GitHubIssue {
            number: 123,
            title: "New title".to_string(),
            state: IssueState::Closed,
        }];

        let mismatches = find_title_mismatches(&todo_items, &github_issues);

        assert!(mismatches.is_empty());
    }

    #[test]
    fn test_find_title_mismatches_ignores_items_without_issue_number() {
        let todo_items = vec![TodoItem {
            text: "Local task".to_string(),
            is_checked: false,
            issue_number: None,
        }];
        let github_issues = vec![GitHubIssue {
            number: 123,
            title: "Unrelated issue".to_string(),
            state: IssueState::Open,
        }];

        let mismatches = find_title_mismatches(&todo_items, &github_issues);

        assert!(mismatches.is_empty());
    }

    #[test]
    fn test_find_title_mismatches_compares_trimmed() {
        let todo_items = vec![TodoItem {
            text: "  Same title  ".to_string(),
            is_checked: false,
            issue_number: Some(123),
        }];
        let github_issues = vec![GitHubIssue {
            number: 123,
            title: "Same title".to_string(),
            state: IssueState::Open,
        }];

        let mismatches = find_title_mismatches(&todo_items, &github_issues);

        assert!(mismatches.is_empty());
    }

    #[test]
    fn test_synchronize_titles_updates_text_when_remote_renamed() {
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

        let (updated_items, local_edits) =
            synchronize_titles(&todo_items, &github_issues, &past_titles);

        assert_eq!(updated_items.len(), 1);
        assert_eq!(updated_items[0].text, "New title");
        assert_eq!(updated_items[0].issue_number, Some(123));
        assert!(local_edits.is_empty());
    }

    #[test]
    fn test_synchronize_titles_keeps_local_edit_and_reports_it() {
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

        let (updated_items, local_edits) =
            synchronize_titles(&todo_items, &github_issues, &past_titles);

        assert_eq!(updated_items.len(), 1);
        assert_eq!(updated_items[0].text, "Locally edited title");
        assert_eq!(local_edits, vec![123]);
    }

    #[test]
    fn test_synchronize_titles_ignores_closed_issues() {
        let todo_items = vec![TodoItem {
            text: "Old title".to_string(),
            is_checked: true,
            issue_number: Some(123),
        }];
        let github_issues = vec![GitHubIssue {
            number: 123,
            title: "New title".to_string(),
            state: IssueState::Closed,
        }];
        let past_titles = HashMap::from([(123u64, vec!["Old title".to_string()])]);

        let (updated_items, local_edits) =
            synchronize_titles(&todo_items, &github_issues, &past_titles);

        assert_eq!(updated_items[0].text, "Old title");
        assert!(local_edits.is_empty());
    }

    #[test]
    fn test_synchronize_titles_keeps_items_in_sync_untouched() {
        let todo_items = vec![
            TodoItem {
                text: "Same title".to_string(),
                is_checked: false,
                issue_number: Some(123),
            },
            TodoItem {
                text: "Local task".to_string(),
                is_checked: false,
                issue_number: None,
            },
        ];
        let github_issues = vec![GitHubIssue {
            number: 123,
            title: "Same title".to_string(),
            state: IssueState::Open,
        }];
        let past_titles = HashMap::new();

        let (updated_items, local_edits) =
            synchronize_titles(&todo_items, &github_issues, &past_titles);

        assert_eq!(updated_items[0].text, "Same title");
        assert_eq!(updated_items[1].text, "Local task");
        assert!(local_edits.is_empty());
    }

    #[test]
    fn test_synchronize_titles_matches_past_title_with_trim() {
        let todo_items = vec![TodoItem {
            text: "  Old title  ".to_string(),
            is_checked: false,
            issue_number: Some(123),
        }];
        let github_issues = vec![GitHubIssue {
            number: 123,
            title: "New title".to_string(),
            state: IssueState::Open,
        }];
        let past_titles = HashMap::from([(123u64, vec!["Old title".to_string()])]);

        let (updated_items, _) = synchronize_titles(&todo_items, &github_issues, &past_titles);

        assert_eq!(updated_items[0].text, "New title");
    }

    #[tokio::test]
    async fn test_synchronize_titles_with_history_updates_renamed_and_reports_local_edits() {
        let todo_items = vec![
            TodoItem {
                text: "Old title".to_string(),
                is_checked: false,
                issue_number: Some(123),
            },
            TodoItem {
                text: "Locally edited title".to_string(),
                is_checked: false,
                issue_number: Some(456),
            },
            TodoItem {
                text: "Same title".to_string(),
                is_checked: false,
                issue_number: Some(789),
            },
        ];
        let github_issues = vec![
            GitHubIssue {
                number: 123,
                title: "New title".to_string(),
                state: IssueState::Open,
            },
            GitHubIssue {
                number: 456,
                title: "Original title".to_string(),
                state: IssueState::Open,
            },
            GitHubIssue {
                number: 789,
                title: "Same title".to_string(),
                state: IssueState::Open,
            },
        ];
        let events_fetcher = |issue_number: u64| async move {
            match issue_number {
                123 => Ok(vec![serde_json::json!({
                    "event": "renamed",
                    "rename": {"from": "Old title", "to": "New title"}
                })]),
                456 => Ok(vec![]),
                _ => Err(anyhow::anyhow!(
                    "history should not be fetched for issue #{issue_number}"
                )),
            }
        };

        let result =
            synchronize_titles_with_history(&todo_items, &github_issues, events_fetcher).await;

        assert!(result.is_ok());
        let synchronization = result.unwrap();
        assert_eq!(
            synchronization,
            TitleSynchronization {
                items: vec![
                    TodoItem {
                        text: "New title".to_string(),
                        is_checked: false,
                        issue_number: Some(123),
                    },
                    TodoItem {
                        text: "Locally edited title".to_string(),
                        is_checked: false,
                        issue_number: Some(456),
                    },
                    TodoItem {
                        text: "Same title".to_string(),
                        is_checked: false,
                        issue_number: Some(789),
                    },
                ],
                locally_edited_issues: vec![456],
            }
        );
    }

    #[tokio::test]
    async fn test_synchronize_titles_with_history_no_mismatches_never_fetches() {
        let todo_items = vec![TodoItem {
            text: "Same title".to_string(),
            is_checked: false,
            issue_number: Some(123),
        }];
        let github_issues = vec![GitHubIssue {
            number: 123,
            title: "Same title".to_string(),
            state: IssueState::Open,
        }];
        let events_fetcher =
            |_: u64| async move { Err(anyhow::anyhow!("history should not be fetched")) };

        let result =
            synchronize_titles_with_history(&todo_items, &github_issues, events_fetcher).await;

        assert!(result.is_ok());
        let synchronization = result.unwrap();
        assert_eq!(synchronization.items, todo_items);
        assert!(synchronization.locally_edited_issues.is_empty());
    }

    #[tokio::test]
    async fn test_synchronize_titles_with_history_propagates_fetcher_error() {
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
        let events_fetcher = |_: u64| async move { Err(anyhow::anyhow!("Network error")) };

        let result =
            synchronize_titles_with_history(&todo_items, &github_issues, events_fetcher).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Network error"));
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
