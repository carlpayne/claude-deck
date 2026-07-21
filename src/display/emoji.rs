//! Emoji image fetching and caching using Twemoji CDN
//!
//! Network fetches never block the render path. On cache miss, a background
//! task downloads the image; callers get `None` until it arrives, then the
//! main loop can redraw via [`take_needs_redraw`].

use anyhow::{Context, Result};
use image::RgbaImage;
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};

const TWEMOJI_CDN: &str = "https://cdn.jsdelivr.net/gh/twitter/twemoji@latest/assets/72x72";

struct EmojiCache {
    /// In-memory images (`None` = permanently failed)
    images: HashMap<String, Option<Arc<RgbaImage>>>,
    /// Codepoints currently being fetched
    loading: HashSet<String>,
    /// Set when a background fetch completes successfully
    needs_redraw: bool,
}

impl EmojiCache {
    fn new() -> Self {
        Self {
            images: HashMap::new(),
            loading: HashSet::new(),
            needs_redraw: false,
        }
    }
}

static EMOJI_CACHE: std::sync::OnceLock<Mutex<EmojiCache>> = std::sync::OnceLock::new();

fn cache() -> &'static Mutex<EmojiCache> {
    EMOJI_CACHE.get_or_init(|| Mutex::new(EmojiCache::new()))
}

/// Returns true (and clears the flag) if any emoji finished loading since last check.
pub fn take_needs_redraw() -> bool {
    let Ok(mut guard) = cache().lock() else {
        return false;
    };
    let needs = guard.needs_redraw;
    guard.needs_redraw = false;
    needs
}

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

/// Get an emoji image without blocking on the network.
///
/// `emoji_ref` can be:
/// - An emoji character: "😀"
/// - A codepoint: "1f600"
/// - A legacy image name: "thumbsup" (falls back to Twemoji via name map)
///
/// Returns `None` while a background fetch is in progress (or on failure).
/// Call [`take_needs_redraw`] from the main loop to refresh buttons when ready.
pub fn get_emoji_image(emoji_ref: &str) -> Option<Arc<RgbaImage>> {
    let codepoint = if is_emoji(emoji_ref) {
        emoji_to_codepoint(emoji_ref)
    } else if is_codepoint(emoji_ref) {
        emoji_ref.to_lowercase()
    } else if let Some(emoji) = legacy_name_to_emoji(emoji_ref) {
        debug!("Converting legacy emoji '{}' to '{}'", emoji_ref, emoji);
        emoji_to_codepoint(emoji)
    } else {
        warn!("Unknown legacy emoji name: {}", emoji_ref);
        return None;
    };

    get_or_fetch_codepoint(&codepoint)
}

fn get_or_fetch_codepoint(codepoint: &str) -> Option<Arc<RgbaImage>> {
    // Fast path: in-memory cache
    {
        let Ok(guard) = cache().lock() else {
            return None;
        };
        if let Some(entry) = guard.images.get(codepoint) {
            return entry.clone();
        }
        if guard.loading.contains(codepoint) {
            return None;
        }
    }

    // Disk cache (local I/O only — no network)
    if let Some(img) = load_cached_emoji(codepoint) {
        let arc = Arc::new(img);
        if let Ok(mut guard) = cache().lock() {
            guard.images.insert(codepoint.to_string(), Some(Arc::clone(&arc)));
        }
        return Some(arc);
    }

    // Kick off background fetch; render path stays non-blocking
    start_background_fetch(codepoint.to_string());
    None
}

fn start_background_fetch(codepoint: String) {
    {
        let Ok(mut guard) = cache().lock() else {
            return;
        };
        if guard.images.contains_key(&codepoint) || guard.loading.contains(&codepoint) {
            return;
        }
        guard.loading.insert(codepoint.clone());
    }

    let fetch = move || {
        let result = fetch_and_cache_emoji(&codepoint);
        let stored = match result {
            Ok(img) => {
                info!("Emoji loaded: {}", codepoint);
                Some(Arc::new(img))
            }
            Err(e) => {
                warn!("Failed to fetch emoji {}: {}", codepoint, e);
                None
            }
        };

        if let Ok(mut guard) = cache().lock() {
            guard.loading.remove(&codepoint);
            let success = stored.is_some();
            guard.images.insert(codepoint, stored);
            if success {
                guard.needs_redraw = true;
            }
        }
    };

    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.spawn(async move {
            let _ = tokio::task::spawn_blocking(fetch).await;
        });
    } else {
        // Tests / no runtime: still keep the call site non-blocking
        std::thread::spawn(fetch);
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

/// Fetch emoji from Twemoji CDN and cache it (blocking — background only)
fn fetch_and_cache_emoji(codepoint: &str) -> Result<RgbaImage> {
    let url = format!("{}/{}.png", TWEMOJI_CDN, codepoint);
    info!("Fetching emoji from CDN: {}", url);

    let response = ureq::get(&url)
        .call()
        .context("Failed to fetch emoji from CDN")?;

    if response.status() != 200 {
        anyhow::bail!("CDN returned status {}", response.status());
    }

    let mut data = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut data)
        .context("Failed to read emoji data")?;

    let img = image::load_from_memory(&data)
        .context("Failed to parse emoji image")?
        .to_rgba8();

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
