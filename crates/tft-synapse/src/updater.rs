//! Startup version check against GitHub Releases API.
//! Runs once in a background thread and sends result via channel.
//!
//! ## Guarantees
//! - Non-fatal: any network or parse failure returns `None`, never panics
//! - Timeout: HTTP request has a 3-second timeout to avoid blocking startup

/// The version of this build.
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// GitHub Releases API endpoint for the latest release.
pub const RELEASES_API: &str = "https://api.github.com/repos/Mattbusel/tft-synapse/releases/latest";

/// Information about a newer available release.
#[derive(Debug, Clone)]
pub struct UpdateInfo {
    /// The latest version string (without leading 'v').
    pub latest_version: String,
    /// True if latest_version differs from CURRENT_VERSION.
    pub update_available: bool,
    /// HTML URL to the release page.
    pub release_url: String,
}

/// Fetch the latest release info from GitHub.
///
/// Returns `None` if the check fails for any reason (network unavailable,
/// timeout, JSON parse error, etc.).  This is intentionally non-fatal.
///
/// # Panics
/// This function never panics.
pub fn check_for_update() -> Option<UpdateInfo> {
    check_for_update_at(RELEASES_API)
}

/// Inner implementation that accepts a custom URL (for testing).
pub fn check_for_update_at(url: &str) -> Option<UpdateInfo> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .ok()?;

    let response = client
        .get(url)
        .header("User-Agent", format!("tft-synapse/{}", CURRENT_VERSION))
        .send()
        .ok()?;

    let json: serde_json::Value = response.json().ok()?;

    let tag_name = json.get("tag_name")?.as_str()?;
    let html_url = json.get("html_url")?.as_str()?;

    // Strip leading 'v' if present
    let latest_version = tag_name.trim_start_matches('v').to_string();
    let update_available = latest_version != CURRENT_VERSION;

    Some(UpdateInfo {
        latest_version,
        update_available,
        release_url: html_url.to_string(),
    })
}

/// Parse version info from a JSON string directly (for testing the parsing logic).
///
/// Returns `None` if the JSON is invalid or missing required fields.
#[cfg_attr(not(test), allow(dead_code))]
pub fn parse_update_info(json_str: &str) -> Option<UpdateInfo> {
    let json: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let tag_name = json.get("tag_name")?.as_str()?;
    let html_url = json.get("html_url")?.as_str()?;
    let latest_version = tag_name.trim_start_matches('v').to_string();
    let update_available = latest_version != CURRENT_VERSION;
    Some(UpdateInfo {
        latest_version,
        update_available,
        release_url: html_url.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_release_json(tag: &str, url: &str) -> String {
        format!(r#"{{"tag_name": "{}", "html_url": "{}"}}"#, tag, url)
    }

    #[test]
    fn test_parse_update_info_with_newer_version() {
        let json = make_release_json("v0.5.0", "https://github.com/releases/v0.5.0");
        let info = parse_update_info(&json).expect("parse failed in test");
        assert_eq!(info.latest_version, "0.5.0");
        assert!(info.update_available);
        assert_eq!(info.release_url, "https://github.com/releases/v0.5.0");
    }

    #[test]
    fn test_parse_update_info_same_version_no_update() {
        let json = make_release_json(
            &format!("v{}", CURRENT_VERSION),
            "https://github.com/releases",
        );
        let info = parse_update_info(&json).expect("parse failed in test");
        assert_eq!(info.latest_version, CURRENT_VERSION);
        assert!(!info.update_available);
    }

    #[test]
    fn test_parse_update_info_strips_leading_v() {
        let json = make_release_json("v1.2.3", "https://example.com");
        let info = parse_update_info(&json).expect("parse failed in test");
        assert_eq!(info.latest_version, "1.2.3");
        assert!(!info.latest_version.starts_with('v'));
    }

    #[test]
    fn test_parse_update_info_no_leading_v() {
        let json = make_release_json("1.0.0", "https://example.com");
        let info = parse_update_info(&json).expect("parse failed in test");
        assert_eq!(info.latest_version, "1.0.0");
    }

    #[test]
    fn test_parse_update_info_invalid_json_returns_none() {
        let result = parse_update_info("{not valid json}");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_update_info_missing_tag_name_returns_none() {
        let json = r#"{"html_url": "https://example.com"}"#;
        let result = parse_update_info(json);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_update_info_missing_html_url_returns_none() {
        let json = r#"{"tag_name": "v0.5.0"}"#;
        let result = parse_update_info(json);
        assert!(result.is_none());
    }

    #[test]
    fn test_check_for_update_bad_url_returns_none() {
        // Network call to a non-existent server should return None, never panic
        let result = check_for_update_at("http://127.0.0.1:1/nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_current_version_constant_is_semver_like() {
        // Sanity-check: CURRENT_VERSION looks like X.Y.Z
        let parts: Vec<&str> = CURRENT_VERSION.split('.').collect();
        assert_eq!(parts.len(), 3, "CURRENT_VERSION should be X.Y.Z format");
        for part in parts {
            assert!(
                part.parse::<u32>().is_ok(),
                "each part should be numeric: {}",
                part
            );
        }
    }

    #[test]
    fn test_update_info_clone() {
        let json = make_release_json("v0.9.0", "https://example.com");
        let info = parse_update_info(&json).expect("parse failed in test");
        let cloned = info.clone();
        assert_eq!(info.latest_version, cloned.latest_version);
        assert_eq!(info.update_available, cloned.update_available);
        assert_eq!(info.release_url, cloned.release_url);
    }
}
