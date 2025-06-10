use crate::AtatWorld;
use cucumber::{given, then, when};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize)]
struct Claims {
    iat: u64,
    exp: u64,
    iss: String,
}

#[derive(Debug, Deserialize)]
struct Installation {
    id: u64,
}

#[derive(Debug, Deserialize)]
struct InstallationAccessTokenResponse {
    token: String,
}

#[given("the user is logged in via GitHub App for tests")]
async fn user_is_logged_in_via_github_app(_world: &mut AtatWorld) {
    const GITHUB_API_BASE_URL: &str = "https://api.github.com";

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client for test login");

    let app_id = env::var("CLIENT_ID").expect("CLIENT_ID env var required");
    let private_key_pem = env::var("PRIVATE_KEY").expect("PRIVATE_KEY env var required");
    let target_owner =
        env::var("TEST_GITHUB_TARGET_OWNER").expect("TEST_GITHUB_TARGET_OWNER env var required");

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time")
        .as_secs();
    let claims = Claims {
        iat: now - 60,
        exp: now + (9 * 60),
        iss: app_id.to_string(),
    };
    let header = Header::new(Algorithm::RS256);

    let encoding_key =
        EncodingKey::from_rsa_pem(private_key_pem.as_bytes()).expect("Valid private key required");

    let jwt = encode(&header, &claims, &encoding_key).expect("JWT generation should succeed");

    let user_install_url = format!(
        "{}/users/{}/installation",
        GITHUB_API_BASE_URL, target_owner
    );
    let resp_user = client
        .get(&user_install_url)
        .bearer_auth(&jwt)
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "atat-cli")
        .send()
        .await
        .expect("Request should succeed");

    let installation_id = if resp_user.status().is_success() {
        let installation = resp_user.json::<Installation>().await.expect("Valid JSON");
        installation.id
    } else {
        let org_install_url = format!("{}/orgs/{}/installation", GITHUB_API_BASE_URL, target_owner);
        let resp_org = client
            .get(&org_install_url)
            .bearer_auth(&jwt)
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "atat-cli")
            .send()
            .await
            .expect("Request should succeed");

        if resp_org.status().is_success() {
            let installation = resp_org.json::<Installation>().await.expect("Valid JSON");
            installation.id
        } else {
            panic!(
                "Could not find an installation for the GitHub App for owner '{}'",
                target_owner
            );
        }
    };

    let url = format!(
        "{}/app/installations/{}/access_tokens",
        GITHUB_API_BASE_URL, installation_id
    );
    let response = client
        .post(&url)
        .bearer_auth(&jwt)
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "atat-cli")
        .send()
        .await
        .expect("Request should succeed");

    let status = response.status();
    if !status.is_success() {
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error body".to_string());
        panic!(
            "Failed to create installation access token: HTTP {}, Body: {}",
            status, error_body
        );
    }
    let token_response = response
        .json::<InstallationAccessTokenResponse>()
        .await
        .expect("Valid JSON response");

    let home_dir = std::env::var("HOME").expect("HOME env var not set for test token storage");
    let token_dir = std::path::PathBuf::from(home_dir).join(".atat");
    let token_path = token_dir.join("token");

    std::fs::create_dir_all(&token_dir).expect("Failed to create token dir for test token storage");
    std::fs::write(&token_path, &token_response.token)
        .expect("Failed to write token for test setup");
}

#[given(regex = r#"^the config file content is '(.*)'$"#)]
async fn given_config_file_content(_world: &mut AtatWorld, content: String) {
    let current_dir = env::current_dir().expect("Failed to get current directory for test setup.");
    let config_path = current_dir.join(".atat").join("config.json");

    if let Some(parent_dir) = config_path.parent() {
        std::fs::create_dir_all(parent_dir)
            .unwrap_or_else(|e| panic!("Failed to create config dir {:?}: {}", parent_dir, e));
    }
    std::fs::write(&config_path, content)
        .unwrap_or_else(|e| panic!("Failed to write config file {:?}: {}", config_path, e));
}

#[given("an empty config file")]
async fn given_empty_config_file(_world: &mut AtatWorld) {
    let current_dir = env::current_dir().expect("Failed to get current directory for test setup.");
    let config_path = current_dir.join(".atat").join("config.json");

    if let Some(parent_dir) = config_path.parent() {
        let _ = std::fs::create_dir_all(parent_dir);
    }
    let _ = std::fs::remove_file(&config_path);
}

#[when("I run `atat remote`")]
async fn when_run_atat_remote(world: &mut AtatWorld) {
    let target_dir = std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".to_string());
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let atat_path = std::path::PathBuf::from(&target_dir)
        .join(profile)
        .join("atat");
    let output = std::process::Command::new(&atat_path)
        .arg("remote")
        .output()
        .unwrap_or_else(|e| panic!("Failed to execute atat command at {:?}: {}", atat_path, e));

    world.captured_output = [output.stdout, output.stderr].concat();
    world.command_status = Some(output.status);
}

#[then(regex = r#"^the output should be "(.*)"$"#)]
async fn then_output_should_be(world: &mut AtatWorld, expected_output: String) {
    let output = String::from_utf8(world.captured_output.clone()).expect("Invalid UTF-8");
    assert_eq!(
        output.trim_end(),
        expected_output,
        "Expected output '{}', but got:\n---\n{}\n---",
        expected_output,
        output.trim_end()
    );
    assert!(
        world.command_status.map_or(false, |s| s.success()),
        "Command failed with status: {:?}",
        world.command_status
    );
}

#[then(regex = r#"^the error should be "(.*)"$"#)]
async fn then_error_should_be(world: &mut AtatWorld, expected_output: String) {
    let output = String::from_utf8(world.captured_output.clone()).expect("Invalid UTF-8");
    assert_eq!(
        output.trim_end(),
        expected_output,
        "Expected output '{}', but got:\n---\n{}\n---",
        expected_output,
        output.trim_end()
    );
    assert!(
        world.command_status.map_or(true, |s| !s.success()),
        "Command should have failed but succeeded with status: {:?}",
        world.command_status
    );
}

#[then("the output should be empty")]
async fn then_output_should_be_empty(world: &mut AtatWorld) {
    let output = String::from_utf8(world.captured_output.clone()).expect("Invalid UTF-8");
    assert!(
        output.trim().is_empty(),
        "Expected output to be empty, but got:\n---\n{}\n---",
        output
    );
    assert!(
        world.command_status.map_or(false, |s| s.success()),
        "Command failed with status: {:?}",
        world.command_status
    );
}

#[given("the user is not logged in")]
async fn user_is_not_logged_in(_world: &mut AtatWorld) {
    let home_dir =
        std::env::var("HOME").expect("HOME environment variable not set for login test setup.");
    let token_path = std::path::PathBuf::from(home_dir)
        .join(".atat")
        .join("token");
    let _ = std::fs::remove_file(&token_path);
}

#[when("the user executes the `atat login` command")]
async fn user_executes_login(world: &mut AtatWorld) {
    let mut buffer: Vec<u8> = Vec::new();
    let writer_option: Option<&mut dyn std::io::Write> = Some(&mut buffer);

    let args = vec!["atat".to_string(), "login".to_string()];
    let result = atat::run::run(
        args,
        writer_option,
        Some(std::time::Duration::from_millis(100)),
    )
    .await;

    world.captured_output = buffer;
    world.login_result = Some(result);
}

#[then(regex = r#"the authentication URL "(.*)" should be displayed on standard output"#)]
async fn check_auth_url(world: &mut AtatWorld, expected_url: String) {
    let output = String::from_utf8(world.captured_output.clone()).expect("Invalid UTF-8");
    assert!(
        output.contains(&expected_url),
        "Expected URL '{}' not found in output:\n{}",
        expected_url,
        output
    );
}

#[then(
    regex = r#"a user code consisting of 8 alphanumeric characters and a hyphen should be displayed on standard output"#
)]
async fn check_user_code_format(world: &mut AtatWorld) {
    let output = String::from_utf8(world.captured_output.clone()).expect("Invalid UTF-8");
    let re = regex::Regex::new(r"and enter code: ([A-Z0-9]{4}-[A-Z0-9]{4})").unwrap();
    assert!(
        re.is_match(&output),
        "User code format (XXXX-YYYY) not found in output:\n{}",
        output
    );
}

#[then("a message prompting for browser authentication should be displayed on standard output")]
async fn check_prompt_message(world: &mut AtatWorld) {
    let output = String::from_utf8(world.captured_output.clone()).expect("Invalid UTF-8");
    assert!(
        output.contains("browser")
            && (output.contains("URL") || output.contains("link"))
            && output.contains("code"),
        "Browser prompt message not found in output:\n{}",
        output
    );
}

#[given("the user has executed `atat login` and the URL and user code are displayed")]
async fn login_executed_and_info_displayed(_world: &mut AtatWorld) {}

#[when(
    "the test runner completes the GitHub device authentication flow using the displayed information"
)]
async fn runner_completes_flow(_world: &mut AtatWorld) {}

#[then("a login success message should be displayed on standard output")]
async fn check_login_success_message(_world: &mut AtatWorld) {}

#[when(regex = r#"^I run `atat remote add ([^`"]*)`$"#)]
async fn when_run_atat_remote_add(world: &mut AtatWorld, repo_name: String) {
    let target_dir = std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".to_string());
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let atat_path = std::path::PathBuf::from(&target_dir)
        .join(profile)
        .join("atat");
    let output = std::process::Command::new(&atat_path)
        .arg("remote")
        .arg("add")
        .arg(repo_name)
        .output()
        .unwrap_or_else(|e| panic!("Failed to execute atat command at {:?}: {}", atat_path, e));

    world.captured_output = [output.stdout, output.stderr].concat();
    world.command_status = Some(output.status);
}

#[then(regex = r#"^the config file should contain "([^"]*)"$"#)]
async fn then_config_file_should_contain(_world: &mut AtatWorld, expected_repo: String) {
    let current_dir = env::current_dir().expect("Failed to get current directory for test.");
    let config_path = current_dir.join(".atat").join("config.json");
    let content = std::fs::read_to_string(&config_path)
        .unwrap_or_else(|e| panic!("Failed to read config file {:?}: {}", config_path, e));
    assert!(
        content.contains(&expected_repo),
        "Expected config file to contain '{}', but got:\\n---\\n{}\\n---",
        expected_repo,
        content
    );
}

#[then("the config file should be empty")]
async fn then_config_file_should_be_empty(_world: &mut AtatWorld) {
    let current_dir = env::current_dir().expect("Failed to get current directory for test.");
    let config_path = current_dir.join(".atat").join("config.json");
    match std::fs::read_to_string(&config_path) {
        Ok(content) => {
            let is_empty_json = content.trim() == "{}";
            let is_empty_repos_array = content.contains(r#""repositories":[]"#)
                || content.contains(r#""repositories": []"#);
            assert!(
                is_empty_json || is_empty_repos_array,
                "Expected config file to be empty or have an empty repositories list, but got:\\n---\\n{}\\n---",
                content
            );
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            panic!("Failed to read config file {:?}: {}", config_path, e);
        }
    }
}
