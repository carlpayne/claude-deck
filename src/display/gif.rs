//! GIF animation support for button displays

use image::{imageops::FilterType, RgbaImage};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{debug, warn};

/// Target size for pre-resized GIF frames (buttons are 112x112, image area is 90x90)
const FRAME_SIZE: u32 = 90;

/// A single frame from a GIF with its display duration
/// Frames are pre-resized to FRAME_SIZE and wrapped in Arc for zero-copy sharing
#[derive(Clone)]
pub struct GifFrame {
    pub image: Arc<RgbaImage>,
    pub delay: Duration,
}

/// Cached GIF with all frames pre-resized for display
#[derive(Clone)]
pub struct CachedGif {
    pub frames: Vec<GifFrame>,
    pub total_duration: Duration,
}

/// Animation state for a single button
struct ButtonAnimation {
    gif_url: String,
    current_frame: usize,
    last_frame_time: Instant,
    /// Whether we've rendered at least one frame (for initial load detection)
    has_rendered: bool,
}

/// Result of a tick - button ID and its current frame image (Arc for zero-copy)
pub struct TickResult {
    pub button_id: u8,
    pub frame: Arc<RgbaImage>,
}

/// Manages GIF animations for buttons
pub struct GifAnimator {
    /// Cache of loaded GIFs by URL
    gif_cache: HashMap<String, Option<CachedGif>>,
    /// Current animation state per button
    animations: HashMap<u8, ButtonAnimation>,
    /// URLs currently being loaded in background
    loading: HashSet<String>,
}

impl GifAnimator {
    pub fn new() -> Self {
        Self {
            gif_cache: HashMap::new(),
            animations: HashMap::new(),
            loading: HashSet::new(),
        }
    }

    /// Check if a GIF is cached (loaded or failed)
    pub fn is_cached(&self, url: &str) -> bool {
        self.gif_cache.contains_key(url)
    }

    /// Check if a GIF is currently loading
    pub fn is_loading(&self, url: &str) -> bool {
        self.loading.contains(url)
    }

    /// Mark a URL as loading (called before spawning background task)
    pub fn mark_loading(&mut self, url: &str) {
        self.loading.insert(url.to_string());
    }

    /// Store a loaded GIF in the cache (called from background task)
    pub fn store_loaded_gif(&mut self, url: String, gif: Option<CachedGif>) {
        self.loading.remove(&url);
        self.gif_cache.insert(url, gif);
    }

    /// Get URLs that need to be loaded for current animations
    pub fn get_pending_urls(&self) -> Vec<String> {
        let mut urls = Vec::new();
        for anim in self.animations.values() {
            if !self.gif_cache.contains_key(&anim.gif_url) && !self.loading.contains(&anim.gif_url)
            {
                urls.push(anim.gif_url.clone());
            }
        }
        urls
    }

    /// Set up animation for a button with a GIF URL (non-blocking)
    /// GIF will be loaded in background - button renders without GIF until loaded
    pub fn set_button_gif(&mut self, button_id: u8, gif_url: &str) {
        // Just register the animation - don't load synchronously
        self.animations.insert(
            button_id,
            ButtonAnimation {
                gif_url: gif_url.to_string(),
                current_frame: 0,
                last_frame_time: Instant::now(),
                has_rendered: false,
            },
        );
    }

    /// Remove animation from a button
    pub fn clear_button(&mut self, button_id: u8) {
        self.animations.remove(&button_id);
    }

    /// Clear all button animations (called on profile/app change)
    pub fn clear_all(&mut self) {
        self.animations.clear();
    }

    /// Update all animations and return buttons that need redraw with their frame images
    /// This avoids the need to lock the animator again during rendering
    /// Uses Arc for zero-copy frame sharing (only increments refcount, no 32KB copy)
    /// Also detects newly loaded GIFs and marks those buttons for initial render
    pub fn tick(&mut self) -> Vec<TickResult> {
        let mut results = Vec::new();
        let now = Instant::now();

        for (&button_id, anim) in self.animations.iter_mut() {
            // Get the cached GIF
            let cached = match self.gif_cache.get(&anim.gif_url).and_then(|o| o.as_ref()) {
                Some(c) => c,
                None => continue,
            };

            if cached.frames.is_empty() {
                continue;
            }

            // Check if this is a newly loaded GIF that hasn't rendered yet
            if !anim.has_rendered {
                anim.has_rendered = true;
                anim.last_frame_time = now;
                let frame = Arc::clone(&cached.frames[0].image);
                results.push(TickResult { button_id, frame });
                continue;
            }

            // Check if it's time to advance to the next frame
            let current_delay = cached.frames[anim.current_frame].delay;
            if now.duration_since(anim.last_frame_time) >= current_delay {
                // Advance to next frame
                anim.current_frame = (anim.current_frame + 1) % cached.frames.len();
                anim.last_frame_time = now;

                // Arc::clone is cheap - just increments refcount, no image data copy
                let frame = Arc::clone(&cached.frames[anim.current_frame].image);
                results.push(TickResult { button_id, frame });
            }
        }

        results
    }

    /// Get the current frame for a button's GIF animation
    pub fn get_current_frame(&self, button_id: u8) -> Option<&RgbaImage> {
        let anim = self.animations.get(&button_id)?;
        let cached = self.gif_cache.get(&anim.gif_url)?.as_ref()?;
        cached.frames.get(anim.current_frame).map(|f| f.image.as_ref())
    }

    /// Check if a button has an active animation
    pub fn has_animation(&self, button_id: u8) -> bool {
        self.animations.contains_key(&button_id)
    }
}

/// Fetch a GIF from URL and decode all frames, pre-resizing to button size
/// This is a blocking operation - call from a background thread/task
pub fn fetch_and_decode_gif(url: &str) -> Option<CachedGif> {
    debug!("Fetching GIF: {}", url);

    // Fetch the GIF
    let response = ureq::get(url).call().ok()?;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(10_000_000) // 10MB limit
        .read_to_end(&mut bytes)
        .ok()?;

    // Decode GIF frames
    let cursor = std::io::Cursor::new(&bytes);
    let decoder = image::codecs::gif::GifDecoder::new(cursor).ok()?;

    use image::AnimationDecoder;
    let frames_iter = decoder.into_frames();

    let mut frames = Vec::new();
    let mut total_duration = Duration::ZERO;

    for frame_result in frames_iter {
        match frame_result {
            Ok(frame) => {
                // Get delay (in milliseconds, default to 100ms if not specified)
                let delay_numer = frame.delay().numer_denom_ms().0;
                let delay = Duration::from_millis(delay_numer.max(30) as u64); // Min 30ms (~33 FPS max)

                let raw_image = frame.into_buffer();

                // Pre-resize frame to target size using fast Triangle filter
                // This avoids expensive per-frame resize during animation
                let image = image::imageops::resize(
                    &raw_image,
                    FRAME_SIZE,
                    FRAME_SIZE,
                    FilterType::Triangle, // Fast bilinear - good enough for small GIFs
                );

                total_duration += delay;
                frames.push(GifFrame {
                    image: Arc::new(image),
                    delay,
                });
            }
            Err(e) => {
                warn!("Failed to decode GIF frame: {}", e);
                break;
            }
        }
    }

    if frames.is_empty() {
        // Fall back to loading as static image
        if let Ok(img) = image::load_from_memory(&bytes) {
            let raw_image = img.to_rgba8();
            let image = image::imageops::resize(
                &raw_image,
                FRAME_SIZE,
                FRAME_SIZE,
                FilterType::Triangle,
            );
            frames.push(GifFrame {
                image: Arc::new(image),
                delay: Duration::from_millis(100),
            });
            total_duration = Duration::from_millis(100);
        } else {
            return None;
        }
    }

    debug!(
        "Loaded GIF with {} frames (pre-resized to {}x{}), total duration {:?}",
        frames.len(),
        FRAME_SIZE,
        FRAME_SIZE,
        total_duration
    );

    Some(CachedGif {
        frames,
        total_duration,
    })
}

/// Global GIF animator instance (thread-safe)
static GIF_ANIMATOR: std::sync::OnceLock<Arc<Mutex<GifAnimator>>> = std::sync::OnceLock::new();

/// Get the global GIF animator
pub fn animator() -> Arc<Mutex<GifAnimator>> {
    GIF_ANIMATOR
        .get_or_init(|| Arc::new(Mutex::new(GifAnimator::new())))
        .clone()
}
