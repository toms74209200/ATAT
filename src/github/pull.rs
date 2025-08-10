use crate::github::issues::{GitHubIssue, IssueState};
use anyhow::Result;

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
