use std::env;
use std::fs;
use std::path::Path;

fn main() {
	println!("cargo:rerun-if-changed=.env");
	println!("cargo:rerun-if-env-changed=LOG_LEVEL");

	let mut level = None;

	// Try .env file if present
	if Path::new(".env").exists() {
		if let Ok(content) = fs::read_to_string(".env") {
			for line in content.lines() {
				let line = line.trim();
				if line.is_empty() || line.starts_with('#') { continue; }
				if let Some(rest) = line.strip_prefix("LOG_LEVEL=") {
					level = Some(rest.trim().to_string());
					break;
				}
			}
		}
	}

	// Fallback to environment variables at build time
	if level.is_none() {
		if let Ok(val) = env::var("LOG_LEVEL") { level = Some(val); }
	}

	let level = level.unwrap_or_else(|| "debug".to_string());
	println!("cargo:rustc-env=LOG_LEVEL={}", level);
}
