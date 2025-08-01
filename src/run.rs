use anyhow::anyhow;

use crate::auth;
use crate::cli;
use crate::config;
use crate::output;
use crate::storage;
use crate::whoami;

mod endpoints {
    pub const DEVICE_CODE: &str = "https://github.com/login/device/code";
    pub const ACCESS_TOKEN: &str = "https://github.com/login/oauth/access_token";
    pub const USER: &str = "https://api.github.com/user";
    pub const REPO_DETAILS: &str = "https://api.github.com/repos";
}

const CLIENT_ID: &str = std::env!("CLIENT_ID");
const DEFAULT_POLL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5 * 60);

pub async fn run(
    args: Vec<String>,
    mut stdout_additional: Option<&mut dyn std::io::Write>,
    poll_timeout: Option<std::time::Duration>,
) -> anyhow::Result<()> {
    match cli::parser::parse_args(&args) {
        cli::parser::Command::Whoami => {
            let storage = storage::FileTokenStorage::new();
            match storage::TokenStorage::load(&storage)? {
                Some(token) => {
                    let client = reqwest::Client::builder()
                        .timeout(std::time::Duration::from_secs(30))
                        .build()?;
                    let response = client
                        .get(endpoints::USER)
                        .bearer_auth(token)
                        .header("Accept", "application/json")
                        .header("User-Agent", "atat-cli")
                        .send()
                        .await?;
                    if response.status().is_success() {
                        let text = response.text().await?;
                        match whoami::extract_login_from_user_response(&text) {
                            Ok(login) => output::println(&login, &mut stdout_additional)?,
                            Err(err) => eprintln!("{}", err),
                        }
                    } else if response.status() == reqwest::StatusCode::UNAUTHORIZED {
                        eprintln!("Token invalid or expired. Please run `login` again.");
                    } else {
                        eprintln!("API request error: {}", response.status());
                    }
                }
                None => eprintln!("No token found. Please run `login` first."),
            }
        }
        cli::parser::Command::Login => {
            let client = anyhow::Context::context(
                reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(30))
                    .build(),
                "Failed to create HTTP client",
            )?;

            let device_code_res = anyhow::Context::context(
                request_device_code(&client, CLIENT_ID).await,
                "Failed to get device code",
            )?;

            output::println(
                &format!("Please visit: {}", device_code_res.verification_uri),
                &mut stdout_additional,
            )?;
            output::println(
                &format!("and enter code: {}", device_code_res.user_code),
                &mut stdout_additional,
            )?;

            let timeout = poll_timeout.unwrap_or(DEFAULT_POLL_TIMEOUT);

            let access_token = anyhow::Context::context(
                poll_for_token(&client, &device_code_res, timeout).await,
                "Failed to poll for access token",
            )?;

            let storage = storage::FileTokenStorage::new();
            anyhow::Context::context(
                storage::TokenStorage::save(&storage, &access_token),
                "Failed to save token",
            )?;
            output::println("✓ Authentication complete", &mut stdout_additional)?;
        }
        cli::parser::Command::RemoteList => {
            let config_storage = anyhow::Context::context(
                storage::LocalConfigStorage::new(),
                "Failed to read project configuration",
            )?;

            match storage::ConfigStorage::load_config(&config_storage) {
                Ok(config_map) => {
                    if let Some(serde_json::Value::Array(repos)) =
                        config_map.get(&config::ConfigKey::Repositories)
                    {
                        for repo_val in repos {
                            if let serde_json::Value::String(repo_str) = repo_val {
                                output::println(repo_str, &mut stdout_additional)?;
                            }
                        }
                    }
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Error loading project config: {}", e));
                }
            }
        }
        cli::parser::Command::RemoteAdd { repo } => {
            let config_storage = match storage::LocalConfigStorage::new() {
                Ok(storage) => storage,
                Err(e) => {
                    return Err(anyhow::anyhow!("Error initializing config storage: {}", e));
                }
            };

            let mut config_map =
                storage::ConfigStorage::load_config(&config_storage).unwrap_or_default();

            let repo_list_val = config_map
                .entry(config::ConfigKey::Repositories)
                .or_insert_with(|| serde_json::json!([]));

            if let Some(repos_array) = repo_list_val.as_array_mut() {
                let new_repo_val = serde_json::json!(repo.clone());
                if !repos_array.contains(&new_repo_val) {
                    let client = reqwest::Client::builder()
                        .timeout(std::time::Duration::from_secs(30))
                        .build()?;

                    let token_storage = storage::FileTokenStorage::new();
                    let token = storage::TokenStorage::load(&token_storage).unwrap_or(None);

                    match check_repo_exists(&client, &repo, token.as_deref()).await {
                        Ok(true) => {
                            repos_array.push(new_repo_val);
                            storage::ConfigStorage::save_config(&config_storage, &config_map)
                                .map_err(|e| {
                                    anyhow::anyhow!("Error saving project config: {}", e)
                                })?;
                        }
                        Ok(false) => {
                            return Err(anyhow!(
                                "Repository {} not found or not accessible.",
                                repo
                            ));
                        }
                        Err(e) => {
                            return Err(anyhow!("Failed to check repository {}: {}", repo, e));
                        }
                    }
                }
            } else {
                return Err(anyhow::anyhow!(
                    "'repositories' key in config is not an array. Cannot add repository."
                ));
            }
        }
        cli::parser::Command::RemoteRemove { repo } => {
            let config_storage = match storage::LocalConfigStorage::new() {
                Ok(storage) => storage,
                Err(e) => {
                    return Err(anyhow::anyhow!("Error initializing config storage: {}", e));
                }
            };

            let config_map =
                storage::ConfigStorage::load_config(&config_storage).unwrap_or_default();

            let new_config = if let Some(serde_json::Value::Array(repos)) =
                config_map.get(&config::ConfigKey::Repositories)
            {
                let repo_json = serde_json::json!(repo.clone());
                let filtered_repos: Vec<serde_json::Value> =
                    repos.iter().filter(|&r| r != &repo_json).cloned().collect();

                if filtered_repos.is_empty() {
                    std::collections::HashMap::new()
                } else {
                    let mut updates = std::collections::HashMap::new();
                    updates.insert(
                        config::ConfigKey::Repositories,
                        serde_json::json!(filtered_repos),
                    );
                    config::update_config(&config_map, &updates)
                }
            } else {
                config_map
            };

            storage::ConfigStorage::save_config(&config_storage, &new_config)
                .map_err(|e| anyhow::anyhow!("Error saving project config: {}", e))?;
        }
        cli::parser::Command::Unknown(message) => return Err(anyhow!(message)),
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid command or arguments. Use --help for usage."
            ));
        }
    }
    Ok(())
}

async fn request_device_code(
    client: &reqwest::Client,
    client_id: &str,
) -> anyhow::Result<auth::DeviceCodeResponse> {
    let response = client
        .post(endpoints::DEVICE_CODE)
        .query(&[("client_id", client_id)])
        .header("Accept", "application/json")
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to get device code: HTTP {}",
            response.status()
        ));
    }

    let device_code_response = response.json::<auth::DeviceCodeResponse>().await?;
    Ok(device_code_response)
}

async fn poll_for_token(
    client: &reqwest::Client,
    device_code: &auth::DeviceCodeResponse,
    timeout: std::time::Duration,
) -> anyhow::Result<String> {
    let start_time = std::time::Instant::now();
    let mut interval = std::time::Duration::from_secs(device_code.interval);

    loop {
        if start_time.elapsed() > timeout {
            return Err(anyhow::anyhow!(
                "Authentication timed out after {} seconds. Please try `login` again.",
                timeout.as_secs()
            ));
        }

        let response = client
            .post(endpoints::ACCESS_TOKEN)
            .header("Accept", "application/json")
            .query(&[
                ("client_id", CLIENT_ID),
                ("device_code", &device_code.device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await?;

        if response.status().is_success() {
            let token_response = response.json::<auth::AccessTokenResponse>().await?;
            match auth::handle_polling_response(&token_response) {
                auth::PollingResult::Success(token) => return Ok(token),
                auth::PollingResult::Wait(Some(new_interval)) => {
                    interval = std::time::Duration::from_secs(new_interval);
                }
                auth::PollingResult::Wait(None) => (),
                auth::PollingResult::Error(err) => return Err(anyhow::anyhow!(err)),
            }
        } else {
            return Err(anyhow::anyhow!("API request error: {}", response.status()));
        }

        tokio::time::sleep(interval).await;
    }
}

async fn check_repo_exists(
    client: &reqwest::Client,
    repo_name: &str,
    token: Option<&str>,
) -> anyhow::Result<bool> {
    let url = format!("{}/{}", endpoints::REPO_DETAILS, repo_name);
    let mut request_builder = client.get(&url).header("User-Agent", "atat-cli");

    if let Some(t) = token {
        request_builder = request_builder.bearer_auth(t);
    }

    let response = request_builder.send().await?;

    match response.status() {
        reqwest::StatusCode::OK => Ok(true),
        reqwest::StatusCode::NOT_FOUND => Ok(false),
        reqwest::StatusCode::FORBIDDEN => Ok(false),
        status => Err(anyhow::anyhow!(
            "Failed to check repository: GitHub API returned HTTP {}",
            status
        )),
    }
}
