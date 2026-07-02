use crate::github::issues::{GitHubIssue, IssueState};
use crate::todo::TodoItem;
use anyhow::Result;
use std::collections::HashMap;

pub(crate) async fn collect_past_titles<F, Fut>(
    todo_items: &[TodoItem],
    github_issues: &[GitHubIssue],
    events_fetcher: F,
) -> Result<HashMap<u64, Vec<String>>>
where
    F: Fn(u64) -> Fut,
    Fut: std::future::Future<Output = Result<Vec<serde_json::Value>>>,
{
    let mut past_titles = HashMap::new();
    for issue_number in find_title_mismatches(todo_items, github_issues) {
        let events = events_fetcher(issue_number).await?;
        past_titles.insert(issue_number, parse_past_titles(&events));
    }
    Ok(past_titles)
}

pub(crate) fn matches_past_title(
    past_titles: &HashMap<u64, Vec<String>>,
    issue_number: u64,
    text: &str,
) -> bool {
    past_titles
        .get(&issue_number)
        .is_some_and(|titles| titles.iter().any(|title| title.trim() == text.trim()))
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_matches_past_title_with_trim() {
        let past_titles = HashMap::from([(123u64, vec!["Old title".to_string()])]);

        assert!(matches_past_title(&past_titles, 123, "  Old title  "));
        assert!(!matches_past_title(&past_titles, 123, "Other title"));
        assert!(!matches_past_title(&past_titles, 456, "Old title"));
    }

    #[tokio::test]
    async fn test_collect_past_titles_fetches_only_mismatched_issues() {
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
        let events_fetcher = |issue_number: u64| async move {
            match issue_number {
                123 => Ok(vec![serde_json::json!({
                    "event": "renamed",
                    "rename": {"from": "Old title", "to": "New title"}
                })]),
                _ => Err(anyhow::anyhow!(
                    "history should not be fetched for issue #{issue_number}"
                )),
            }
        };

        let result = collect_past_titles(&todo_items, &github_issues, events_fetcher).await;

        assert!(result.is_ok());
        let past_titles = result.unwrap();
        assert_eq!(
            past_titles,
            HashMap::from([(123u64, vec!["Old title".to_string()])])
        );
    }

    #[tokio::test]
    async fn test_collect_past_titles_propagates_fetcher_error() {
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

        let result = collect_past_titles(&todo_items, &github_issues, events_fetcher).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Network error"));
    }
}
