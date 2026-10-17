// Copyright (c) 2026 Nicolás Correa
// SPDX-License-Identifier: GPL-3.0-or-later

use std::process::Command;

const LATEST_URL: &str = "https://api.github.com/repos/ncorrea-13/bw-tui/releases/latest";

fn parse(v: &str) -> Option<Vec<u64>> {
    v.trim_start_matches('v')
        .split('.')
        .map(|p| p.parse().ok())
        .collect()
}

fn is_newer(latest: &str, current: &str) -> bool {
    matches!((parse(latest), parse(current)), (Some(l), Some(c)) if l > c)
}

pub fn check() -> Option<String> {
    let out = Command::new("curl")
        .args(["-fsS", "--max-time", "5", LATEST_URL])
        .output()
        .ok()?;
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let tag = json["tag_name"].as_str()?;
    is_newer(tag, env!("CARGO_PKG_VERSION")).then(|| tag.to_string())
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn compares_numerically() {
        assert!(is_newer("v0.2.7", "0.2.6"));
        assert!(is_newer("v0.10.0", "0.9.9"));
        assert!(!is_newer("v0.2.6", "0.2.6"));
        assert!(!is_newer("v0.2.5", "0.2.6"));
        assert!(!is_newer("nightly", "0.2.6"));
    }
}
