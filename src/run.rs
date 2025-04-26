use crate::auth;
use crate::cli;
use crate::output;
use crate::storage;
use crate::whoami;

mod endpoints {
    pub const DEVICE_CODE: &str = "https://github.com/login/device/code";
    pub const ACCESS_TOKEN: &str = "https://github.com/login/oauth/access_token";
    pub const USER: &str = "https://api.github.com/user";
}

const CLIENT_ID: &str = std::env!("GITHUB_CLIENT_ID");
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
        _ => {
            output::println(
                "Invalid command or arguments. Use --help for usage.",
                &mut stdout_additional,
            )?;
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
