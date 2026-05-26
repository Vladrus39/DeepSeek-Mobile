//! Inject workspace `.env` API key into debug builds for on-device testing.
//!
//! Release builds must not embed secrets; only `PROFILE=debug` reads `.env`.

use std::path::PathBuf;

fn main() {
    let profile = std::env::var("PROFILE").unwrap_or_default();
    if profile != "debug" {
        return;
    }

    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let env_path = manifest.join("../../.env");
    println!("cargo:rerun-if-changed={}", env_path.display());

    let Ok(content) = std::fs::read_to_string(&env_path) else {
        return;
    };

    for line in content.lines() {
        let line = line.trim();
        if let Some(key) = line.strip_prefix("DEEPSEEK_API_KEY=") {
            let key = key.trim().trim_matches('"');
            if key.starts_with("sk-") {
                println!("cargo:rustc-env=DEEPSEEK_API_KEY={key}");
            }
            break;
        }
    }
}
