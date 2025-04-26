use cucumber::World;

#[derive(Debug, Default, World)]
pub struct AtatWorld {
    pub captured_output: Vec<u8>,
    pub login_result: Option<Result<(), anyhow::Error>>,
}

#[tokio::main]
async fn main() {
    AtatWorld::run("features").await;
}

mod steps;
