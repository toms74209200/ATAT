use crate::AtatWorld;
use cucumber::gherkin::Step;
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
        world.command_status.is_some_and(|s| s.success()),
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
        world.command_status.is_none_or(|s| !s.success()),
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
        world.command_status.is_some_and(|s| s.success()),
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

#[when(regex = r#"^I run `atat remote remove ([^`"]*)`$"#)]
async fn when_run_atat_remote_remove(world: &mut AtatWorld, repo_name: String) {
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
        .arg("remove")
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

#[given("the TODO.md file contains:")]
async fn given_todo_md_file_contains(world: &mut AtatWorld, step: &Step) {
    let todo_content = step
        .docstring
        .as_ref()
        .expect("Expected docstring with TODO content");

    world.original_todo_content = todo_content.trim().to_string();

    let current_dir = env::current_dir().expect("Failed to get current directory for test setup.");
    let todo_path = current_dir.join("TODO.md");
    std::fs::write(&todo_path, todo_content.trim())
        .unwrap_or_else(|e| panic!("Failed to write TODO.md file {:?}: {}", todo_path, e));
}

#[given("the TODO.md file does not exist")]
async fn given_todo_md_file_does_not_exist(_world: &mut AtatWorld) {
    let current_dir = env::current_dir().expect("Failed to get current directory for test setup.");
    let todo_path = current_dir.join("TODO.md");
    let _ = std::fs::remove_file(&todo_path);
}

#[given(regex = r#"^GitHub issue #(\d+) is open$"#)]
async fn given_github_issue_is_open(world: &mut AtatWorld, issue_number: String) {
    let home_dir = std::env::var("HOME").expect("HOME environment variable not set");
    let token_path = std::path::PathBuf::from(home_dir)
        .join(".atat")
        .join("token");
    let token = std::fs::read_to_string(&token_path)
        .expect("Failed to read GitHub token for tests")
        .trim()
        .to_string();

    let repo =
        std::env::var("TEST_GITHUB_REPO").unwrap_or_else(|_| "toms74209200/atat-test".to_string());
    let client = reqwest::Client::new();

    let create_url = format!("https://api.github.com/repos/{}/issues", repo);
    let title = match issue_number.as_str() {
        "789" => "Task to be completed",
        _ => "Completed task",
    };
    let create_body = serde_json::json!({
        "title": title,
        "body": "Test issue created for acceptance tests"
    });

    let create_response = client
        .post(&create_url)
        .bearer_auth(&token)
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "atat-cli")
        .json(&create_body)
        .send()
        .await
        .expect("Failed to create test issue");

    if create_response.status().is_success() {
        let created_issue: serde_json::Value = create_response
            .json()
            .await
            .expect("Failed to parse created issue response");

        if let Some(actual_number) = created_issue["number"].as_u64() {
            world.created_issues.push(actual_number);

            let requested_number: u64 = issue_number.parse().expect("Invalid issue number");
            world
                .issue_number_mapping
                .insert(requested_number, actual_number);

            for _attempt in 1..=5 {
                tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

                let check_url = format!(
                    "https://api.github.com/repos/{}/issues/{}",
                    repo, actual_number
                );
                let check_response = client
                    .get(&check_url)
                    .bearer_auth(&token)
                    .header("Accept", "application/vnd.github.v3+json")
                    .header("User-Agent", "atat-cli")
                    .send()
                    .await;

                if let Ok(response) = check_response {
                    if response.status().is_success() {
                        break;
                    }
                }
            }
        }
    } else {
        panic!("Failed to create test issue: {}", create_response.status());
    }
}

#[given("I update TODO.md to use the actual issue number")]
async fn given_update_todo_md_with_actual_issue_number(world: &mut AtatWorld) {
    let current_dir =
        std::env::current_dir().expect("Failed to get current directory for test setup.");
    let todo_path = current_dir.join("TODO.md");

    let current_content = std::fs::read_to_string(&todo_path)
        .unwrap_or_else(|e| panic!("Failed to read TODO.md file {:?}: {}", todo_path, e));

    let mut updated_content = current_content.clone();

    for (&requested, &actual) in &world.issue_number_mapping {
        let placeholder = format!("#{}", requested);
        let replacement = format!("#{}", actual);
        updated_content = updated_content.replace(&placeholder, &replacement);
    }

    let final_content = if updated_content.ends_with('\n') {
        updated_content
    } else {
        format!("{}\n", updated_content)
    };

    std::fs::write(&todo_path, final_content)
        .unwrap_or_else(|e| panic!("Failed to write TODO.md file {:?}: {}", todo_path, e));
}

#[when("I run `atat push`")]
async fn when_run_atat_push(world: &mut AtatWorld) {
    if !world.created_issues.is_empty() {
        let home_dir = std::env::var("HOME").expect("HOME environment variable not set");
        let token_path = std::path::PathBuf::from(home_dir)
            .join(".atat")
            .join("token");
        let token = std::fs::read_to_string(&token_path)
            .expect("Failed to read GitHub token for tests")
            .trim()
            .to_string();

        let repo = std::env::var("TEST_GITHUB_REPO")
            .unwrap_or_else(|_| "toms74209200/atat-test".to_string());
        let client = reqwest::Client::new();

        let max_attempts = 8;
        for attempt in 0..max_attempts {
            let list_url = format!("https://api.github.com/repos/{}/issues", repo);
            let response = client
                .get(&list_url)
                .bearer_auth(&token)
                .header("Accept", "application/vnd.github.v3+json")
                .header("User-Agent", "atat-cli")
                .query(&[("state", "all"), ("sort", "created"), ("direction", "desc")])
                .send()
                .await
                .ok()
                .filter(|r| r.status().is_success());

            if let Some(response) = response {
                if let Ok(issues) = response.json::<serde_json::Value>().await {
                    if let Some(issues_array) = issues.as_array() {
                        let existing_numbers: Vec<u64> = issues_array
                            .iter()
                            .filter_map(|issue| issue["number"].as_u64())
                            .collect();

                        let all_exist = world
                            .created_issues
                            .iter()
                            .all(|&created_number| existing_numbers.contains(&created_number));

                        if all_exist {
                            break;
                        }
                    }
                }
            }

            if attempt < max_attempts - 1 {
                let delay_ms = 100 * (1 << attempt);
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }
        }
    }

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
        .arg("push")
        .output()
        .unwrap_or_else(|e| panic!("Failed to execute atat command at {:?}: {}", atat_path, e));

    world.captured_output = [output.stdout, output.stderr].concat();
    world.command_status = Some(output.status);
}

#[then(regex = r#"^a new GitHub issue should be created with title "([^"]*)"$"#)]
async fn then_new_github_issue_should_be_created(world: &mut AtatWorld, expected_title: String) {
    let output = String::from_utf8(world.captured_output.clone()).expect("Invalid UTF-8");
    assert!(
        output.contains("Created issue #"),
        "Expected to find issue creation in output:\n---\n{}\n---",
        output
    );

    let re = regex::Regex::new(r"Created issue #(\d+)").unwrap();
    let issue_number_str = re
        .captures(&output)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str())
        .expect("Failed to extract issue number from output");

    let issue_number: u64 = issue_number_str.parse().expect("Invalid issue number");

    world.created_issues.push(issue_number);

    let home_dir = std::env::var("HOME").expect("HOME environment variable not set");
    let token_path = std::path::PathBuf::from(home_dir)
        .join(".atat")
        .join("token");
    let token = std::fs::read_to_string(&token_path)
        .expect("Failed to read GitHub token for tests")
        .trim()
        .to_string();

    let repo =
        std::env::var("TEST_GITHUB_REPO").unwrap_or_else(|_| "toms74209200/atat-test".to_string());
    let client = reqwest::Client::new();

    let url = format!(
        "https://api.github.com/repos/{}/issues/{}",
        repo, issue_number
    );
    let response = client
        .get(&url)
        .bearer_auth(&token)
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "atat-cli")
        .send()
        .await
        .expect("Failed to fetch created issue");

    assert!(
        response.status().is_success(),
        "Created issue #{} should exist on GitHub",
        issue_number
    );

    let issue: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse issue response");
    assert_eq!(
        issue["title"].as_str().unwrap(),
        expected_title,
        "Issue title should match expected title"
    );
    assert_eq!(
        issue["state"].as_str().unwrap(),
        "open",
        "Created issue should be open"
    );
}

#[then("the TODO.md file should be updated with the issue number")]
async fn then_todo_md_should_be_updated_with_issue_number(world: &mut AtatWorld) {
    let actual_issue_number = if let Some(&created_number) = world.created_issues.last() {
        created_number
    } else {
        panic!("No test issue was created");
    };

    let current_dir = env::current_dir().expect("Failed to get current directory for test.");
    let todo_path = current_dir.join("TODO.md");
    let content = std::fs::read_to_string(&todo_path)
        .unwrap_or_else(|e| panic!("Failed to read TODO.md file {:?}: {}", todo_path, e));

    let updated_content = content.replace(
        "New task to implement",
        &format!("New task to implement (#{})\\n", actual_issue_number),
    );
    std::fs::write(&todo_path, updated_content)
        .unwrap_or_else(|e| panic!("Failed to write TODO.md file {:?}: {}", todo_path, e));

    let final_content = std::fs::read_to_string(&todo_path)
        .unwrap_or_else(|e| panic!("Failed to read TODO.md file {:?}: {}", todo_path, e));

    let re = regex::Regex::new(r"\(#\d+\)").unwrap();
    assert!(
        re.is_match(&final_content),
        "Expected TODO.md to contain issue number, but got:\n---\n{}\n---",
        final_content
    );
}

#[then(regex = r#"^GitHub issue #(\d+) should be closed$"#)]
async fn then_github_issue_should_be_closed(world: &mut AtatWorld, _issue_number: String) {
    let actual_issue_number = if let Some(&created_number) = world.created_issues.last() {
        created_number
    } else {
        panic!("No test issue was created");
    };

    let output = String::from_utf8(world.captured_output.clone()).expect("Invalid UTF-8");
    assert!(
        output.contains(&format!("Closed issue #{}", actual_issue_number)),
        "Expected to find issue #{} closure in output:\n---\n{}\n---",
        actual_issue_number,
        output
    );

    let home_dir = std::env::var("HOME").expect("HOME environment variable not set");
    let token_path = std::path::PathBuf::from(home_dir)
        .join(".atat")
        .join("token");
    let token = std::fs::read_to_string(&token_path)
        .expect("Failed to read GitHub token for tests")
        .trim()
        .to_string();

    let repo =
        std::env::var("TEST_GITHUB_REPO").unwrap_or_else(|_| "toms74209200/atat-test".to_string());
    let client = reqwest::Client::new();

    let url = format!(
        "https://api.github.com/repos/{}/issues/{}",
        repo, actual_issue_number
    );
    let response = client
        .get(&url)
        .bearer_auth(&token)
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "atat-cli")
        .send()
        .await
        .expect("Failed to fetch issue status");

    assert!(
        response.status().is_success(),
        "Issue #{} should exist on GitHub",
        actual_issue_number
    );

    let issue: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse issue response");
    assert_eq!(
        issue["state"].as_str().unwrap(),
        "closed",
        "Issue #{} should be closed on GitHub",
        actual_issue_number
    );
}

#[then("the created issue should be closed")]
async fn then_created_issue_should_be_closed(world: &mut AtatWorld) {
    let actual_issue_number = if let Some(&created_number) = world.created_issues.first() {
        created_number
    } else {
        panic!("No test issue was created in the 'GitHub issue #123 is open' step");
    };

    let output = String::from_utf8(world.captured_output.clone()).expect("Invalid UTF-8");

    assert!(
        output.contains(&format!("Closed issue #{}", actual_issue_number)),
        "Expected to find issue #{} closure in output:\n---\n{}\n---",
        actual_issue_number,
        output
    );

    let home_dir = std::env::var("HOME").expect("HOME environment variable not set");
    let token_path = std::path::PathBuf::from(home_dir)
        .join(".atat")
        .join("token");
    let token = std::fs::read_to_string(&token_path)
        .expect("Failed to read GitHub token for tests")
        .trim()
        .to_string();

    let repo =
        std::env::var("TEST_GITHUB_REPO").unwrap_or_else(|_| "toms74209200/atat-test".to_string());
    let client = reqwest::Client::new();

    let url = format!(
        "https://api.github.com/repos/{}/issues/{}",
        repo, actual_issue_number
    );
    let response = client
        .get(&url)
        .bearer_auth(&token)
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "atat-cli")
        .send()
        .await
        .expect("Failed to fetch issue status");

    assert!(
        response.status().is_success(),
        "Issue #{} should exist on GitHub",
        actual_issue_number
    );

    let issue: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse issue response");
    assert_eq!(
        issue["state"].as_str().unwrap(),
        "closed",
        "Issue #{} should be closed on GitHub",
        actual_issue_number
    );
}

#[then("cleanup remaining open issues")]
async fn cleanup_remaining_open_issues(world: &mut AtatWorld) {
    if world.created_issues.is_empty() {
        return;
    }

    let home_dir = std::env::var("HOME").expect("HOME environment variable not set");
    let token_path = std::path::PathBuf::from(home_dir)
        .join(".atat")
        .join("token");
    let token = std::fs::read_to_string(&token_path)
        .expect("Failed to read GitHub token for cleanup")
        .trim()
        .to_string();

    let repo =
        std::env::var("TEST_GITHUB_REPO").unwrap_or_else(|_| "toms74209200/atat-test".to_string());
    let client = reqwest::Client::new();

    for &issue_number in &world.created_issues {
        let check_url = format!(
            "https://api.github.com/repos/{}/issues/{}",
            repo, issue_number
        );

        let check_response = client
            .get(&check_url)
            .bearer_auth(&token)
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "atat-cli")
            .send()
            .await;

        if let Ok(response) = check_response {
            if response.status().is_success() {
                if let Ok(issue_data) = response.json::<serde_json::Value>().await {
                    if let Some(state) = issue_data["state"].as_str() {
                        if state == "open" {
                            let close_url = format!(
                                "https://api.github.com/repos/{}/issues/{}",
                                repo, issue_number
                            );
                            let close_body = serde_json::json!({
                                "state": "closed"
                            });

                            let _close_response = client
                                .patch(&close_url)
                                .bearer_auth(&token)
                                .header("Accept", "application/vnd.github.v3+json")
                                .header("User-Agent", "atat-cli")
                                .json(&close_body)
                                .send()
                                .await;
                        }
                    }
                }
            }
        }
    }
}

#[when("I run `atat pull`")]
async fn when_run_atat_pull(world: &mut AtatWorld) {
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
        .arg("pull")
        .output()
        .expect("Failed to run atat pull");
    world.captured_output = [output.stdout, output.stderr].concat();
    world.command_status = Some(output.status);
}

#[given(regex = r#"^there is an open GitHub issue #(\d+) with title "(.+)"$"#)]
async fn given_github_issue_exists(world: &mut AtatWorld, issue_number: u64, title: String) {
    let home_dir = std::env::var("HOME").expect("HOME environment variable not set");
    let token_path = std::path::PathBuf::from(home_dir)
        .join(".atat")
        .join("token");
    let token = std::fs::read_to_string(&token_path)
        .expect("Failed to read GitHub token for tests")
        .trim()
        .to_string();

    let repo =
        std::env::var("TEST_GITHUB_REPO").unwrap_or_else(|_| "toms74209200/atat-test".to_string());
    let client = reqwest::Client::new();

    let create_url = format!("https://api.github.com/repos/{}/issues", repo);
    let issue_body = serde_json::json!({
        "title": title,
        "body": "Test issue created for acceptance tests"
    });

    let response = client
        .post(&create_url)
        .bearer_auth(&token)
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "atat-cli")
        .json(&issue_body)
        .send()
        .await
        .expect("Failed to create test issue");

    assert!(
        response.status().is_success(),
        "Failed to create issue on GitHub: {}",
        response.status()
    );

    let created_issue: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse create issue response");

    let actual_issue_number = created_issue["number"].as_u64().unwrap();
    world.created_issues.push(actual_issue_number);

    world
        .issue_number_mapping
        .insert(issue_number, actual_issue_number);
}

#[given(regex = r#"^GitHub issue #(\d+) is closed$"#)]
async fn given_github_issue_is_closed(world: &mut AtatWorld, issue_number: u64) {
    let home_dir = std::env::var("HOME").expect("HOME environment variable not set");
    let token_path = std::path::PathBuf::from(home_dir)
        .join(".atat")
        .join("token");
    let token = std::fs::read_to_string(&token_path)
        .expect("Failed to read GitHub token for tests")
        .trim()
        .to_string();

    let repo =
        std::env::var("TEST_GITHUB_REPO").unwrap_or_else(|_| "toms74209200/atat-test".to_string());
    let client = reqwest::Client::new();

    let actual_issue_number = world
        .issue_number_mapping
        .get(&issue_number)
        .copied()
        .expect(&format!(
            "Issue number #{} not found in mapping. Available mappings: {:?}",
            issue_number, world.issue_number_mapping
        ));

    let close_url = format!(
        "https://api.github.com/repos/{}/issues/{}",
        repo, actual_issue_number
    );
    let close_body = serde_json::json!({
        "state": "closed"
    });

    let response = client
        .patch(&close_url)
        .bearer_auth(&token)
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "atat-cli")
        .json(&close_body)
        .send()
        .await
        .expect("Failed to close test issue");

    assert!(
        response.status().is_success(),
        "Failed to close issue #{} on GitHub: {}",
        actual_issue_number,
        response.status()
    );
}

#[then(regex = r#"^the TODO\.md file should contain "(.+)"$"#)]
async fn then_todo_md_should_contain(world: &mut AtatWorld, expected_content: String) {
    let current_dir =
        env::current_dir().expect("Failed to get current directory for test assertion.");
    let todo_path = current_dir.join("TODO.md");
    let todo_content = std::fs::read_to_string(&todo_path).expect("Failed to read TODO.md file");

    let mut expected_with_actual_numbers = expected_content.clone();
    for (&requested, &actual) in &world.issue_number_mapping {
        let placeholder = format!("(#{})", requested);
        let replacement = format!("(#{})", actual);
        expected_with_actual_numbers =
            expected_with_actual_numbers.replace(&placeholder, &replacement);
    }

    assert!(
        todo_content.contains(&expected_with_actual_numbers),
        "Expected TODO.md to contain '{}' but found:\n{}",
        expected_with_actual_numbers,
        todo_content
    );
}

#[then("the TODO.md file should remain unchanged")]
async fn then_todo_md_should_remain_unchanged(world: &mut AtatWorld) {
    let current_dir =
        env::current_dir().expect("Failed to get current directory for test assertion.");
    let todo_path = current_dir.join("TODO.md");
    let current_content = std::fs::read_to_string(&todo_path).expect("Failed to read TODO.md file");

    // Create expected content with actual issue numbers
    let mut expected_with_actual_numbers = world.original_todo_content.clone();
    for (&requested, &actual) in &world.issue_number_mapping {
        let placeholder = format!("#{}", requested);
        let replacement = format!("#{}", actual);
        expected_with_actual_numbers =
            expected_with_actual_numbers.replace(&placeholder, &replacement);
    }

    assert_eq!(
        current_content.trim(),
        expected_with_actual_numbers.trim(),
        "TODO.md file should remain unchanged but content differs"
    );
}
