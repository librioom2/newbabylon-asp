use structopt::StructOpt;
use reqwest::Client;
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(StructOpt, Debug)]
#[structopt(name = "asp_demo_client", about = "Rust client for ASP Babylon demo server")]
struct Opt {
    /// Text to translate
    #[structopt(short, long)]
    text: String,

    /// Target language code (e.g., ru, de)
    #[structopt(short, long)]
    target: String,

    /// Server URL (default http://127.0.0.1:8080)
    #[structopt(short, long, default_value = "http://127.0.0.1:8080")]
    server: String,
}

#[derive(Deserialize, Debug)]
struct TranslationResponse {
    translation: String,
}

fn load_language_tokens<P: AsRef<Path>>(path: P) -> Result<std::collections::HashMap<String, String>, Box<dyn std::error::Error>> {
    let data = fs::read_to_string(path)?;
    let map: std::collections::HashMap<String, String> = serde_json::from_str(&data)?;
    Ok(map)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();

    // Validate target language against tokens file
    let tokens_path = "language_tokens.json"; // relative to current dir (demo/client)
    let tokens = load_language_tokens(tokens_path)?;
    if !tokens.contains_key(&opt.target) {
        eprintln!("Unsupported target language: {}", opt.target);
        eprintln!("Supported languages: {}", tokens.keys().cloned().collect::<Vec<_>>().join(", "));
        std::process::exit(1);
    }

    let client = Client::new();
    let request_url = format!("{}/translate", opt.server.trim_end_matches('/'));
    let resp = client
        .get(&request_url)
        .query(&[ ("text", opt.text), ("target", opt.target) ])
        .send()
        .await?;

    if !resp.status().is_success() {
        eprintln!("Server returned error: {}", resp.status());
        let body = resp.text().await?;
        eprintln!("Response body: {}", body);
        std::process::exit(1);
    }

    let translation: TranslationResponse = resp.json().await?;
    println!("Translation: {}", translation.translation);
    Ok(())
}
