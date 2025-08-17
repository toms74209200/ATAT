use cucumber::World;
use std::collections::HashMap;
use std::process::ExitStatus;

#[derive(Debug, Default, World)]
pub struct AtatWorld {
    pub captured_output: Vec<u8>,
    pub captured_error: Vec<u8>,
    pub login_result: Option<Result<(), anyhow::Error>>,
    pub command_status: Option<ExitStatus>,
    pub created_issues: Vec<u64>,
    pub issue_number_mapping: HashMap<u64, u64>,
    pub original_todo_content: String,
}

#[tokio::main]
async fn main() {
    AtatWorld::run("features").await;
}

mod steps;
