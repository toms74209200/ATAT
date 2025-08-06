use serde::Deserialize;
use serde_json;

/// Struct representing the GitHub `/user` API response.
#[derive(Deserialize, Debug, PartialEq)]
pub struct UserResponse {
    /// The login username of the authenticated user.
    pub login: String,
    /// The unique ID of the authenticated user.
    pub id: u64,
}

/// Extracts the `login` field from a GitHub `/user` API JSON response string.
///
/// # Arguments
///
/// * `json` - A JSON string returned by the GitHub `/user` API.
///
/// # Returns
///
/// * `Ok(login)` if parsing succeeds.
/// * `Err(error_message)` if parsing fails.
pub fn extract_login_from_user_response(json: &str) -> Result<String, String> {
    serde_json::from_str::<UserResponse>(json)
        .map(|user| user.login)
        .map_err(|e| format!("Failed to parse user response: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_login_success() {
        let json = r#"{"login":"octocat","id":1}"#;
        let result = extract_login_from_user_response(json);
        assert_eq!(result, Ok("octocat".to_string()));
    }

    #[test]
    fn test_extract_login_invalid_json() {
        let json = "{ invalid json }";
        let result = extract_login_from_user_response(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_login_missing_field() {
        let json = r#"{"id":1}"#;
        let result = extract_login_from_user_response(json);
        assert!(result.is_err());
    }
}
