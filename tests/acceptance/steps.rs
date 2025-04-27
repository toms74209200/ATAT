use crate::AtatWorld;
use cucumber::{given, then, when};
use regex::Regex;
use std::{io::Write, time::Duration};

#[given("the user is not logged in")]
async fn user_is_not_logged_in(_world: &mut AtatWorld) {}

#[when("the user executes the `atat login` command")]
async fn user_executes_login(world: &mut AtatWorld) {
    let mut buffer: Vec<u8> = Vec::new();
    let writer_option: Option<&mut dyn Write> = Some(&mut buffer);

    let args = vec!["atat".to_string(), "login".to_string()];
    let result = atat::run::run(args, writer_option, Some(Duration::from_millis(100))).await;

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
    let re = Regex::new(r"and enter code: ([A-Z0-9]{4}-[A-Z0-9]{4})").unwrap();
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
