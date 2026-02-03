//! Emoji image fetching and caching using Twemoji CDN

use anyhow::{Context, Result};
use image::RgbaImage;
use std::path::PathBuf;
use tracing::{debug, info, warn};

const TWEMOJI_CDN: &str = "https://cdn.jsdelivr.net/gh/twitter/twemoji@latest/assets/72x72";

/// Get the emoji cache directory
fn cache_dir() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME environment variable not set")?;
    let cache_path = PathBuf::from(home).join(".config/claude-deck/emoji-cache");
    std::fs::create_dir_all(&cache_path).context("Failed to create emoji cache directory")?;
    Ok(cache_path)
}

/// Convert an emoji string to its Twemoji codepoint format
/// e.g., "😀" -> "1f600", "👍🏻" -> "1f44d-1f3fb"
pub fn emoji_to_codepoint(emoji: &str) -> String {
    emoji
        .chars()
        .filter(|c| *c != '\u{FE0F}') // Remove variation selector
        .map(|c| format!("{:x}", c as u32))
        .collect::<Vec<_>>()
        .join("-")
}

/// Check if a string looks like an emoji (starts with non-ASCII)
pub fn is_emoji(s: &str) -> bool {
    s.chars().next().map(|c| c as u32 > 127).unwrap_or(false)
}

/// Check if a string looks like a codepoint (hex format like "1f600")
pub fn is_codepoint(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

/// Get an emoji image, fetching from CDN if not cached
///
/// `emoji_ref` can be:
/// - An emoji character: "😀"
/// - A codepoint: "1f600"
/// - A legacy image name: "thumbsup" (falls back to assets/emoji/)
pub fn get_emoji_image(emoji_ref: &str) -> Option<RgbaImage> {
    // Determine if this is an emoji, codepoint, or legacy name
    let codepoint = if is_emoji(emoji_ref) {
        emoji_to_codepoint(emoji_ref)
    } else if is_codepoint(emoji_ref) {
        emoji_ref.to_lowercase()
    } else {
        // Legacy: try to load from assets/emoji/{name}.png
        return load_legacy_emoji(emoji_ref);
    };

    // Try to load from cache
    if let Some(img) = load_cached_emoji(&codepoint) {
        return Some(img);
    }

    // Fetch from CDN (blocking - we're in sync context)
    match fetch_and_cache_emoji(&codepoint) {
        Ok(img) => Some(img),
        Err(e) => {
            warn!("Failed to fetch emoji {}: {}", codepoint, e);
            None
        }
    }
}

/// Load emoji from local cache
fn load_cached_emoji(codepoint: &str) -> Option<RgbaImage> {
    let cache_path = cache_dir().ok()?;
    let file_path = cache_path.join(format!("{}.png", codepoint));

    if file_path.exists() {
        debug!("Loading cached emoji: {}", codepoint);
        image::open(&file_path).ok().map(|img| img.to_rgba8())
    } else {
        None
    }
}

/// Fetch emoji from Twemoji CDN and cache it
fn fetch_and_cache_emoji(codepoint: &str) -> Result<RgbaImage> {
    let url = format!("{}/{}.png", TWEMOJI_CDN, codepoint);
    info!("Fetching emoji from CDN: {}", url);

    // Use a simple blocking HTTP request
    let response = ureq::get(&url)
        .call()
        .context("Failed to fetch emoji from CDN")?;

    if response.status() != 200 {
        anyhow::bail!("CDN returned status {}", response.status());
    }

    // Read the image data
    let mut data = Vec::new();
    response.into_reader().read_to_end(&mut data)
        .context("Failed to read emoji data")?;

    // Parse as image
    let img = image::load_from_memory(&data)
        .context("Failed to parse emoji image")?
        .to_rgba8();

    // Cache it
    let cache_path = cache_dir()?;
    let file_path = cache_path.join(format!("{}.png", codepoint));
    img.save(&file_path).context("Failed to cache emoji")?;
    debug!("Cached emoji: {}", codepoint);

    Ok(img)
}

/// Load an image from a base64 data URL (e.g., "data:image/png;base64,...")
pub fn load_base64_image(data_url: &str) -> Option<RgbaImage> {
    // Parse data URL format: data:image/png;base64,<data>
    let parts: Vec<&str> = data_url.splitn(2, ',').collect();
    if parts.len() != 2 {
        warn!("Invalid data URL format");
        return None;
    }

    // Decode base64
    use base64::{engine::general_purpose::STANDARD, Engine};
    let data = match STANDARD.decode(parts[1]) {
        Ok(d) => d,
        Err(e) => {
            warn!("Failed to decode base64 image: {}", e);
            return None;
        }
    };

    // Parse as image
    match image::load_from_memory(&data) {
        Ok(img) => Some(img.to_rgba8()),
        Err(e) => {
            warn!("Failed to parse image from base64: {}", e);
            None
        }
    }
}

/// Convert legacy emoji names to emoji characters for Twemoji fetching
fn legacy_name_to_emoji(name: &str) -> Option<&'static str> {
    match name {
        "thumbsup" => Some("👍"),
        "thumbsdown" => Some("👎"),
        "check" => Some("✅"),
        "eyes" => Some("👀"),
        "tada" => Some("🎉"),
        "heart" => Some("❤️"),
        "joy" => Some("😂"),
        "fire" => Some("🔥"),
        "hundred" => Some("💯"),
        "pray" => Some("🙏"),
        _ => None,
    }
}

/// Load legacy emoji by converting name to emoji and fetching from Twemoji
fn load_legacy_emoji(name: &str) -> Option<RgbaImage> {
    // Convert legacy name to emoji character
    if let Some(emoji) = legacy_name_to_emoji(name) {
        debug!("Converting legacy emoji '{}' to '{}'", name, emoji);
        let codepoint = emoji_to_codepoint(emoji);

        // Try cache first
        if let Some(img) = load_cached_emoji(&codepoint) {
            return Some(img);
        }

        // Fetch from CDN
        match fetch_and_cache_emoji(&codepoint) {
            Ok(img) => return Some(img),
            Err(e) => warn!("Failed to fetch emoji for legacy '{}': {}", name, e),
        }
    }

    warn!("Unknown legacy emoji name: {}", name);
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emoji_to_codepoint() {
        assert_eq!(emoji_to_codepoint("😀"), "1f600");
        assert_eq!(emoji_to_codepoint("👍"), "1f44d");
        assert_eq!(emoji_to_codepoint("❤️"), "2764"); // Variation selector removed
        assert_eq!(emoji_to_codepoint("👍🏻"), "1f44d-1f3fb"); // With skin tone
    }

    #[test]
    fn test_is_emoji() {
        assert!(is_emoji("😀"));
        assert!(is_emoji("👍"));
        assert!(!is_emoji("thumbsup"));
        assert!(!is_emoji("1f600"));
    }

    #[test]
    fn test_is_codepoint() {
        assert!(is_codepoint("1f600"));
        assert!(is_codepoint("1f44d-1f3fb"));
        assert!(!is_codepoint("thumbsup"));
        assert!(!is_codepoint("😀"));
    }
}
