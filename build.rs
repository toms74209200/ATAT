use std::env;

fn main() {
    let client_id = env::var("GITHUB_CLIENT_ID").unwrap_or_else(|_| "YOUR_CLIENT_ID".to_string());
    println!("cargo:rustc-env=GITHUB_CLIENT_ID={}", client_id);
}
