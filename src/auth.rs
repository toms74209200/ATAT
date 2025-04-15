use serde::{Deserialize, Serialize};

/// Request parameters for device code
#[derive(Serialize, Debug)]
pub struct DeviceCodeRequest {
    client_id: String,
}

/// Response from device code request
#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

/// Request parameters for access token
#[derive(Serialize, Debug)]
pub struct AccessTokenRequest {
    client_id: String,
    device_code: String,
    grant_type: String,
}

/// Response from access token request
#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct AccessTokenResponse {
    pub access_token: Option<String>,
    pub token_type: Option<String>,
    pub scope: Option<String>,
    pub error: Option<String>,
}

/// Generate request parameters for access token
#[allow(dead_code)]
pub fn create_access_token_request(client_id: &str, device_code: &str) -> AccessTokenRequest {
    AccessTokenRequest {
        client_id: client_id.to_string(),
        device_code: device_code.to_string(),
        grant_type: "urn:ietf:params:oauth:grant-type:device_code".to_string(),
    }
}

/// Handle polling state for access token acquisition
///
/// This function determines the next action based on the previous polling result
/// Returns the token if successful, wait time if more waiting is needed,
/// or an error message if an error occurred
#[allow(dead_code)]
pub fn handle_polling_response(response: &AccessTokenResponse) -> PollingResult {
    if let Some(token) = &response.access_token {
        return PollingResult::Success(token.clone());
    }

    if let Some(error) = &response.error {
        match error.as_str() {
            "authorization_pending" => {
                // User has not entered the code yet. Wait and continue polling
                PollingResult::Wait(None)
            }
            "slow_down" => {
                // Polling too fast. Increase interval and wait
                PollingResult::Wait(Some(5)) // Additional 5 seconds wait
            }
            "expired_token" => PollingResult::Error(
                "The device code has expired. Please run `login` again.".to_string(),
            ),
            "access_denied" => PollingResult::Error("Login cancelled by user.".to_string()),
            _ => PollingResult::Error(format!("Unknown error: {}", error)),
        }
    } else {
        // No error and no access token (this case should not normally occur)
        PollingResult::Error("Invalid response from GitHub API".to_string())
    }
}

/// Enum representing polling result
#[derive(Debug, PartialEq)]
pub enum PollingResult {
    /// Authentication successful, contains access token
    Success(String),
    /// Waiting required, Some contains additional wait time (seconds)
    Wait(Option<u64>),
    /// Error occurred, contains error message
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_access_token_request() {
        let client_id = "test_client_id";
        let device_code = "test_device_code";
        let request = create_access_token_request(client_id, device_code);

        assert_eq!(request.client_id, client_id);
        assert_eq!(request.device_code, device_code);
        assert_eq!(
            request.grant_type,
            "urn:ietf:params:oauth:grant-type:device_code"
        );
    }

    #[test]
    fn test_handle_polling_response_success() {
        let response = AccessTokenResponse {
            access_token: Some("test_token".to_string()),
            token_type: Some("bearer".to_string()),
            scope: Some("".to_string()),
            error: None,
        };

        let result = handle_polling_response(&response);
        assert_eq!(result, PollingResult::Success("test_token".to_string()));
    }

    #[test]
    fn test_handle_polling_response_authorization_pending() {
        let response = AccessTokenResponse {
            access_token: None,
            token_type: None,
            scope: None,
            error: Some("authorization_pending".to_string()),
        };

        let result = handle_polling_response(&response);
        assert_eq!(result, PollingResult::Wait(None));
    }

    #[test]
    fn test_handle_polling_response_slow_down() {
        let response = AccessTokenResponse {
            access_token: None,
            token_type: None,
            scope: None,
            error: Some("slow_down".to_string()),
        };

        let result = handle_polling_response(&response);
        assert_eq!(result, PollingResult::Wait(Some(5)));
    }

    #[test]
    fn test_handle_polling_response_expired_token() {
        let response = AccessTokenResponse {
            access_token: None,
            token_type: None,
            scope: None,
            error: Some("expired_token".to_string()),
        };

        let result = handle_polling_response(&response);
        assert_eq!(
            result,
            PollingResult::Error(
                "The device code has expired. Please run `login` again.".to_string()
            )
        );
    }

    #[test]
    fn test_handle_polling_response_access_denied() {
        let response = AccessTokenResponse {
            access_token: None,
            token_type: None,
            scope: None,
            error: Some("access_denied".to_string()),
        };

        let result = handle_polling_response(&response);
        assert_eq!(
            result,
            PollingResult::Error("Login cancelled by user.".to_string())
        );
    }

    #[test]
    fn test_handle_polling_response_unknown_error() {
        let response = AccessTokenResponse {
            access_token: None,
            token_type: None,
            scope: None,
            error: Some("unknown_error".to_string()),
        };

        let result = handle_polling_response(&response);
        assert_eq!(
            result,
            PollingResult::Error("Unknown error: unknown_error".to_string())
        );
    }
}
