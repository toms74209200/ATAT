use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;

mod auth;

/// GitHub API endpoints
mod endpoints {
    pub const DEVICE_CODE: &str = "https://github.com/login/device/code";
}

const CLIENT_ID: &str = std::env!("GITHUB_CLIENT_ID");

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("Failed to create HTTP client")?;

    let device_code_res = request_device_code(&client, CLIENT_ID)
        .await
        .context("Failed to get device code")?;

    println!("Please visit: {}", device_code_res.verification_uri);
    println!("and enter code: {}", device_code_res.user_code);

    Ok(())
}

async fn request_device_code(client: &Client, client_id: &str) -> Result<auth::DeviceCodeResponse> {
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
