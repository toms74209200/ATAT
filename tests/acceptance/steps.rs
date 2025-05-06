use crate::AtatWorld;
use cucumber::{given, then, when};
use std::env;

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
        .join(".config")
        .join("atat")
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
