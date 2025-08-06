use std::env;

fn main() {
    let client_id = env::var("CLIENT_ID").unwrap_or_else(|_| "YOUR_CLIENT_ID".to_string());
    println!("cargo:rustc-env=CLIENT_ID={client_id}");
}
