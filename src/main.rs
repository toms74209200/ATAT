/// GitHub API endpoints
mod endpoints {
    pub const DEVICE_CODE: &str = "https://github.com/login/device/code";
    pub const ACCESS_TOKEN: &str = "https://github.com/login/oauth/access_token";
    pub const USER: &str = "https://api.github.com/user";
}

const CLIENT_ID: &str = std::env!("GITHUB_CLIENT_ID");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    match atat::cli::parser::parse_args(&args) {
        atat::cli::parser::Command::Whoami => {
            let storage = atat::storage::FileTokenStorage::new();
            match atat::storage::TokenStorage::load(&storage)? {
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
                        match atat::whoami::extract_login_from_user_response(&text) {
                            Ok(login) => println!("{}", login),
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
        atat::cli::parser::Command::Login => {
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

            println!("Please visit: {}", device_code_res.verification_uri);
            println!("and enter code: {}", device_code_res.user_code);

            let access_token = anyhow::Context::context(
                poll_for_token(&client, &device_code_res).await,
                "Failed to poll for access token",
            )?;

            let storage = atat::storage::FileTokenStorage::new();
            anyhow::Context::context(
                atat::storage::TokenStorage::save(&storage, &access_token),
                "Failed to save token",
            )?;
            println!("✓ Authentication complete");
        }
        _ => {
            return Err(anyhow::Error::msg(format!("Unknown command {}", args[1])));
        }
    }
    Ok(())
}

async fn request_device_code(
    client: &reqwest::Client,
    client_id: &str,
) -> anyhow::Result<atat::auth::DeviceCodeResponse> {
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

    let device_code_response = response.json::<atat::auth::DeviceCodeResponse>().await?;
    Ok(device_code_response)
}

async fn poll_for_token(
    client: &reqwest::Client,
    device_code: &atat::auth::DeviceCodeResponse,
) -> anyhow::Result<String> {
    let mut interval = std::time::Duration::from_secs(device_code.interval);

    loop {
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
            let token_response = response.json::<atat::auth::AccessTokenResponse>().await?;
            match atat::auth::handle_polling_response(&token_response) {
                atat::auth::PollingResult::Success(token) => return Ok(token),
                atat::auth::PollingResult::Wait(Some(new_interval)) => {
                    interval = std::time::Duration::from_secs(new_interval);
                }
                atat::auth::PollingResult::Wait(None) => (),
                atat::auth::PollingResult::Error(err) => return Err(anyhow::anyhow!(err)),
            }
        } else {
            return Err(anyhow::anyhow!("API request error: {}", response.status()));
        }

        tokio::time::sleep(interval).await;
    }
}
