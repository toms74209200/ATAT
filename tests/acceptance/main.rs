use cucumber::World;
use std::process::ExitStatus;

#[derive(Debug, Default, World)]
pub struct AtatWorld {
    pub captured_output: Vec<u8>,
    pub login_result: Option<Result<(), anyhow::Error>>,
    pub command_status: Option<ExitStatus>,
}

#[tokio::main]
async fn main() {
    AtatWorld::run("features").await;
}

mod steps;
