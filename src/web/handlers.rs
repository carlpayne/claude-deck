//! API endpoint handlers

use axum::{
    extract::{Path, Query, State},
    Json,
};
use std::sync::{Arc, RwLock as StdRwLock};
use tokio::sync::{mpsc, RwLock as TokioRwLock};
use tracing::{info, warn};

use crate::config::Config;
use crate::profiles::store::ButtonConfigEntry;
use crate::profiles::{generate_default_profiles, ProfileManager};

use super::types::{
    get_action_types, get_available_keys, get_builtin_actions, get_color_presets,
    get_modifier_keys, ActionsResponse, ApiResponse, AppsResponse, ColorsResponse,
    ConfigChangeEvent, CreateProfileRequest, GiphyGif, GiphySearchQuery, GiphySearchResponse,
    HasDefaultsResponse, InstalledApp, ProfileResponse, ProfileSummary, UpdateButtonRequest,
    UpdateProfileRequest,
};

/// Shared application state for web handlers
pub struct AppState {
    pub config: Arc<TokioRwLock<Config>>,
    pub profile_manager: Arc<StdRwLock<ProfileManager>>,
    pub change_tx: mpsc::Sender<ConfigChangeEvent>,
    pub device_state: Arc<TokioRwLock<crate::state::AppState>>,
}

/// GET /api/profiles - List all profiles
pub async fn list_profiles(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<Vec<ProfileSummary>>> {
    let manager = state.profile_manager.read().unwrap();
    let profiles: Vec<ProfileSummary> = manager
        .get_profiles()
        .iter()
        .map(ProfileSummary::from)
        .collect();

    Json(ApiResponse::ok(profiles))
}

/// GET /api/profiles/:name - Get a profile with all buttons
pub async fn get_profile(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Json<ApiResponse<ProfileResponse>> {
    let manager = state.profile_manager.read().unwrap();

    match manager.get_profile(&name) {
        Some(profile) => Json(ApiResponse::ok(ProfileResponse::from(profile))),
        None => Json(ApiResponse::error(format!("Profile '{}' not found", name))),
    }
}

/// PUT /api/profiles/:name - Update a profile
pub async fn update_profile(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(request): Json<UpdateProfileRequest>,
) -> Json<ApiResponse<ProfileResponse>> {
    let response = {
        let mut manager = state.profile_manager.write().unwrap();

        match manager.get_profile_mut(&name) {
            Some(profile) => {
                // Update fields if provided
                if let Some(match_apps) = request.match_apps {
                    profile.match_apps = match_apps;
                }
                if let Some(buttons) = request.buttons {
                    profile.buttons = buttons;
                }

                Some(ProfileResponse::from(&*profile))
            }
            None => None,
        }
    };

    match response {
        Some(response) => {
            // Notify of change
            if let Err(e) = state
                .change_tx
                .send(ConfigChangeEvent::ProfileUpdated(name.clone()))
                .await
            {
                warn!("Failed to send config change event: {}", e);
            }

            // Save config
            save_config(&state).await;

            Json(ApiResponse::ok(response))
        }
        None => Json(ApiResponse::error(format!("Profile '{}' not found", name))),
    }
}

/// PUT /api/profiles/:name/buttons/:position - Update a single button
pub async fn update_button(
    State(state): State<Arc<AppState>>,
    Path((name, position)): Path<(String, u8)>,
    Json(request): Json<UpdateButtonRequest>,
) -> Json<ApiResponse<ButtonConfigEntry>> {
    let result = {
        let mut manager = state.profile_manager.write().unwrap();

        match manager.get_profile_mut(&name) {
            Some(profile) => {
                // Find the button entry
                let button = profile.buttons.iter_mut().find(|b| b.position == position);

                match button {
                    Some(button) => {
                        // Update fields if provided
                        if let Some(label) = request.label {
                            button.label = label;
                        }
                        if let Some(color) = request.color {
                            button.color = color;
                        }
                        if let Some(bright_color) = request.bright_color {
                            button.bright_color = bright_color;
                        }
                        if let Some(action) = request.action {
                            button.action = action;
                        }
                        if let Some(emoji_image) = request.emoji_image {
                            button.emoji_image = if emoji_image.is_empty() {
                                None
                            } else {
                                Some(emoji_image)
                            };
                        }
                        if let Some(custom_image) = request.custom_image {
                            button.custom_image = if custom_image.is_empty() {
                                None
                            } else {
                                Some(custom_image)
                            };
                        }
                        if let Some(gif_url) = request.gif_url {
                            button.gif_url = if gif_url.is_empty() {
                                None
                            } else {
                                Some(gif_url)
                            };
                        }

                        Ok(button.clone())
                    }
                    None => Err(format!(
                        "Button at position {} not found in profile '{}'",
                        position, name
                    )),
                }
            }
            None => Err(format!("Profile '{}' not found", name)),
        }
    };

    match result {
        Ok(response) => {
            // Notify of change
            if let Err(e) = state
                .change_tx
                .send(ConfigChangeEvent::ButtonUpdated {
                    profile: name.clone(),
                    position,
                })
                .await
            {
                warn!("Failed to send config change event: {}", e);
            }

            // Save config
            save_config(&state).await;

            Json(ApiResponse::ok(response))
        }
        Err(e) => Json(ApiResponse::error(e)),
    }
}

/// POST /api/reload - Hot-reload config
pub async fn reload_config(State(state): State<Arc<AppState>>) -> Json<ApiResponse<String>> {
    info!("Config reload requested via web UI");

    // Reload config from disk
    match Config::load() {
        Ok(new_config) => {
            let profiles = if new_config.profiles.is_empty() {
                generate_default_profiles()
            } else {
                new_config.profiles.clone()
            };

            // Update state
            {
                let mut config = state.config.write().await;
                *config = new_config;
            }
            {
                let mut manager = state.profile_manager.write().unwrap();
                manager.set_profiles(profiles);
            }

            // Notify of change
            if let Err(e) = state.change_tx.send(ConfigChangeEvent::Reload).await {
                warn!("Failed to send config change event: {}", e);
            }

            Json(ApiResponse::ok("Config reloaded".to_string()))
        }
        Err(e) => Json(ApiResponse::error(format!("Failed to reload config: {}", e))),
    }
}

/// GET /api/colors - Get available color presets
pub async fn get_colors() -> Json<ApiResponse<ColorsResponse>> {
    Json(ApiResponse::ok(ColorsResponse {
        presets: get_color_presets(),
    }))
}

/// GET /api/actions - Get available action types and keys
pub async fn get_actions() -> Json<ApiResponse<ActionsResponse>> {
    Json(ApiResponse::ok(ActionsResponse {
        action_types: get_action_types(),
        available_keys: get_available_keys(),
        modifier_keys: get_modifier_keys(),
        builtin_actions: get_builtin_actions(),
    }))
}

/// Save current config to disk
async fn save_config(state: &AppState) {
    let config = state.config.read().await;
    let manager = state.profile_manager.read().unwrap();

    // Create new config with updated profiles
    let mut new_config = config.clone();
    new_config.profiles = manager.get_profiles().to_vec();

    if let Err(e) = new_config.save() {
        warn!("Failed to save config: {}", e);
    } else {
        info!("Config saved to disk");
    }
}

/// Built-in profile names that have known default configurations
const BUILTIN_PROFILES: &[&str] = &["claude", "slack"];

/// GET /api/profiles/:name/has-defaults - Check if profile has known defaults
pub async fn has_profile_defaults(Path(name): Path<String>) -> Json<ApiResponse<HasDefaultsResponse>> {
    let has_defaults = BUILTIN_PROFILES.contains(&name.to_lowercase().as_str());
    Json(ApiResponse::ok(HasDefaultsResponse { has_defaults }))
}

/// POST /api/profiles/:name/reset - Reset profile to default button configuration
pub async fn reset_profile(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Json<ApiResponse<ProfileResponse>> {
    let name_lower = name.to_lowercase();

    if !BUILTIN_PROFILES.contains(&name_lower.as_str()) {
        return Json(ApiResponse::error(format!(
            "Profile '{}' does not have known defaults",
            name
        )));
    }

    // Generate default profiles and find the matching one
    let default_profiles = generate_default_profiles();
    let default_profile = match default_profiles.iter().find(|p| p.name == name_lower) {
        Some(p) => p.clone(),
        None => {
            return Json(ApiResponse::error(format!(
                "Could not find default config for '{}'",
                name
            )))
        }
    };

    // Update the profile in the manager
    let response = {
        let mut manager = state.profile_manager.write().unwrap();
        match manager.get_profile_mut(&name_lower) {
            Some(profile) => {
                profile.buttons = default_profile.buttons;
                Some(ProfileResponse::from(&*profile))
            }
            None => None,
        }
    };

    match response {
        Some(response) => {
            // Notify of change
            if let Err(e) = state
                .change_tx
                .send(ConfigChangeEvent::ProfileUpdated(name_lower.clone()))
                .await
            {
                warn!("Failed to send config change event: {}", e);
            }

            // Save config
            save_config(&state).await;

            info!("Reset profile '{}' to defaults", name_lower);
            Json(ApiResponse::ok(response))
        }
        None => Json(ApiResponse::error(format!("Profile '{}' not found", name))),
    }
}

/// GET /api/apps - List installed macOS applications
pub async fn list_apps() -> Json<ApiResponse<AppsResponse>> {
    let apps_dir = std::path::Path::new("/Applications");

    let mut apps: Vec<InstalledApp> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(apps_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "app") {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown")
                    .to_string();

                // Try to read bundle ID from Info.plist
                let bundle_id = read_bundle_id(&path);

                apps.push(InstalledApp { name, bundle_id });
            }
        }
    }

    // Sort alphabetically
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Json(ApiResponse::ok(AppsResponse { apps }))
}

/// Read bundle ID from an app's Info.plist
fn read_bundle_id(app_path: &std::path::Path) -> Option<String> {
    let plist_path = app_path.join("Contents/Info.plist");
    if !plist_path.exists() {
        return None;
    }

    // Read the plist file and look for CFBundleIdentifier
    // Using simple string matching since we don't want to add a plist dependency
    if let Ok(content) = std::fs::read_to_string(&plist_path) {
        // Find CFBundleIdentifier key and extract the following string value
        if let Some(key_pos) = content.find("<key>CFBundleIdentifier</key>") {
            let after_key = &content[key_pos..];
            if let Some(string_start) = after_key.find("<string>") {
                let value_start = string_start + 8;
                if let Some(string_end) = after_key[value_start..].find("</string>") {
                    return Some(after_key[value_start..value_start + string_end].to_string());
                }
            }
        }
    }
    None
}

/// POST /api/profiles - Create a new profile
pub async fn create_profile(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateProfileRequest>,
) -> Json<ApiResponse<ProfileResponse>> {
    let name = request.name.to_lowercase().replace(' ', "-");

    // Validate name
    if name.is_empty() {
        return Json(ApiResponse::error("Profile name cannot be empty"));
    }

    // Check if profile already exists
    {
        let manager = state.profile_manager.read().unwrap();
        if manager.get_profile(&name).is_some() {
            return Json(ApiResponse::error(format!(
                "Profile '{}' already exists",
                name
            )));
        }
    }

    // Get buttons - either copy from existing profile or use empty default
    let buttons = if let Some(copy_from) = &request.copy_from {
        let manager = state.profile_manager.read().unwrap();
        match manager.get_profile(copy_from) {
            Some(source) => source.buttons.clone(),
            None => {
                return Json(ApiResponse::error(format!(
                    "Source profile '{}' not found",
                    copy_from
                )))
            }
        }
    } else {
        // Create empty/default buttons with no action
        use crate::profiles::store::{ActionConfig, ButtonConfigEntry};
        (0..10)
            .map(|pos| ButtonConfigEntry {
                position: pos,
                label: "---".to_string(),
                color: "#505560".to_string(),
                bright_color: "#6E737D".to_string(),
                action: ActionConfig::Custom {
                    value: "".to_string(),  // Empty = no action
                },
                emoji_image: None,
                custom_image: None,
                gif_url: None,
            })
            .collect()
    };

    // Create new profile
    let new_profile = crate::profiles::store::ProfileConfig {
        name: name.clone(),
        match_apps: request.match_apps,
        buttons,
    };

    let response = ProfileResponse::from(&new_profile);

    // Add to manager
    {
        let mut manager = state.profile_manager.write().unwrap();
        let mut profiles = manager.get_profiles().to_vec();
        profiles.push(new_profile);
        manager.set_profiles(profiles);
    }

    // Save config
    save_config(&state).await;

    info!("Created new profile '{}'", name);
    Json(ApiResponse::ok(response))
}

/// DELETE /api/profiles/:name - Delete a user-created profile
pub async fn delete_profile(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Json<ApiResponse<String>> {
    let name_lower = name.to_lowercase();

    // Prevent deletion of built-in profiles
    if BUILTIN_PROFILES.contains(&name_lower.as_str()) {
        return Json(ApiResponse::error(format!(
            "Cannot delete built-in profile '{}'",
            name
        )));
    }

    // Remove from manager
    let removed = {
        let mut manager = state.profile_manager.write().unwrap();
        let profiles = manager.get_profiles().to_vec();
        let filtered: Vec<_> = profiles
            .into_iter()
            .filter(|p| p.name.to_lowercase() != name_lower)
            .collect();

        let was_removed = filtered.len() < manager.get_profiles().len();
        if was_removed {
            manager.set_profiles(filtered);
        }
        was_removed
    };

    if !removed {
        return Json(ApiResponse::error(format!("Profile '{}' not found", name)));
    }

    // Notify of change
    if let Err(e) = state.change_tx.send(ConfigChangeEvent::Reload).await {
        warn!("Failed to send config change event: {}", e);
    }

    // Save config
    save_config(&state).await;

    info!("Deleted profile '{}'", name);
    Json(ApiResponse::ok(format!("Profile '{}' deleted", name)))
}

/// DELETE /api/profiles/:name/buttons/:position - Reset a single button to default
pub async fn reset_button(
    State(state): State<Arc<AppState>>,
    Path((name, position)): Path<(String, u8)>,
) -> Json<ApiResponse<ButtonConfigEntry>> {
    use crate::profiles::store::{ActionConfig, ButtonConfigEntry};

    let result = {
        let mut manager = state.profile_manager.write().unwrap();

        match manager.get_profile_mut(&name) {
            Some(profile) => {
                // Create default empty button
                let default_button = ButtonConfigEntry {
                    position,
                    label: "---".to_string(),
                    color: "#505560".to_string(),
                    bright_color: "#6E737D".to_string(),
                    action: ActionConfig::Custom {
                        value: "".to_string(),
                    },
                    emoji_image: None,
                    custom_image: None,
                    gif_url: None,
                };

                // Find and replace the button
                if let Some(button) = profile.buttons.iter_mut().find(|b| b.position == position) {
                    *button = default_button.clone();
                    Ok(default_button)
                } else {
                    Err(format!(
                        "Button at position {} not found in profile '{}'",
                        position, name
                    ))
                }
            }
            None => Err(format!("Profile '{}' not found", name)),
        }
    };

    match result {
        Ok(response) => {
            // Notify of change
            if let Err(e) = state
                .change_tx
                .send(ConfigChangeEvent::ButtonUpdated {
                    profile: name.clone(),
                    position,
                })
                .await
            {
                warn!("Failed to send config change event: {}", e);
            }

            // Save config
            save_config(&state).await;

            info!("Reset button {} in profile '{}'", position, name);
            Json(ApiResponse::ok(response))
        }
        Err(e) => Json(ApiResponse::error(e)),
    }
}

/// POST /api/profiles/:name/buttons/swap - Swap two buttons
pub async fn swap_buttons(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(request): Json<super::types::SwapButtonsRequest>,
) -> Json<ApiResponse<String>> {
    let pos1 = request.position1;
    let pos2 = request.position2;

    if pos1 == pos2 {
        return Json(ApiResponse::error("Cannot swap a button with itself"));
    }

    if pos1 >= 10 || pos2 >= 10 {
        return Json(ApiResponse::error("Button positions must be 0-9"));
    }

    let result = {
        let mut manager = state.profile_manager.write().unwrap();

        match manager.get_profile_mut(&name) {
            Some(profile) => {
                // Find indices of both buttons
                let idx1 = profile.buttons.iter().position(|b| b.position == pos1);
                let idx2 = profile.buttons.iter().position(|b| b.position == pos2);

                match (idx1, idx2) {
                    (Some(i1), Some(i2)) => {
                        // Swap the button configs but keep positions
                        profile.buttons.swap(i1, i2);
                        // Update position fields after swap
                        profile.buttons[i1].position = pos1;
                        profile.buttons[i2].position = pos2;
                        Ok(())
                    }
                    _ => Err(format!(
                        "Button positions {} or {} not found in profile '{}'",
                        pos1, pos2, name
                    )),
                }
            }
            None => Err(format!("Profile '{}' not found", name)),
        }
    };

    match result {
        Ok(()) => {
            // Notify of change for both buttons
            let _ = state
                .change_tx
                .send(ConfigChangeEvent::ButtonUpdated {
                    profile: name.clone(),
                    position: pos1,
                })
                .await;
            let _ = state
                .change_tx
                .send(ConfigChangeEvent::ButtonUpdated {
                    profile: name.clone(),
                    position: pos2,
                })
                .await;

            // Save config
            save_config(&state).await;

            info!("Swapped buttons {} and {} in profile '{}'", pos1, pos2, name);
            Json(ApiResponse::ok("Buttons swapped".to_string()))
        }
        Err(e) => Json(ApiResponse::error(e)),
    }
}

/// GET /api/giphy/search - Search for GIFs
pub async fn search_giphy(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GiphySearchQuery>,
) -> Json<ApiResponse<GiphySearchResponse>> {
    let api_key = {
        let config = state.config.read().await;
        config.giphy.api_key.clone()
    };

    if api_key.is_empty() {
        return Json(ApiResponse::error(
            "Giphy API key not configured. This shouldn't happen - try restarting the app.",
        ));
    }

    let url = format!(
        "https://api.giphy.com/v1/gifs/search?api_key={}&q={}&limit={}&rating=g",
        api_key,
        urlencoding::encode(&query.q),
        query.limit
    );

    let client = reqwest::Client::new();
    match client.get(&url).send().await {
        Ok(response) => {
            if !response.status().is_success() {
                return Json(ApiResponse::error(format!(
                    "Giphy API error: {}",
                    response.status()
                )));
            }

            match response.json::<serde_json::Value>().await {
                Ok(json) => {
                    let gifs = parse_giphy_response(&json);
                    Json(ApiResponse::ok(GiphySearchResponse { gifs }))
                }
                Err(e) => Json(ApiResponse::error(format!("Failed to parse Giphy response: {}", e))),
            }
        }
        Err(e) => Json(ApiResponse::error(format!("Failed to fetch from Giphy: {}", e))),
    }
}

/// GET /api/status - Get current Claude status from state file + live device state
pub async fn get_status(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let state_path = std::path::PathBuf::from(home).join(".claude-deck/state.json");

    let mut status = match std::fs::read_to_string(&state_path) {
        Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(state) => state,
            Err(_) => serde_json::json!({
                "task": "READY",
                "tool_detail": null,
                "waiting_for_input": false,
                "model": "unknown",
                "connected": false
            }),
        },
        Err(_) => {
            serde_json::json!({
                "task": "READY",
                "tool_detail": null,
                "waiting_for_input": false,
                "model": "unknown",
                "connected": false
            })
        }
    };

    // Augment with live device state (volume, connected status)
    let device = state.device_state.read().await;
    if let Some(obj) = status.as_object_mut() {
        obj.insert("volume".to_string(), serde_json::json!(device.volume));
        obj.insert("volume_display_active".to_string(), serde_json::json!(device.is_volume_display_active()));
        obj.insert("connected".to_string(), serde_json::json!(device.connected));
    }

    Json(ApiResponse::ok(status))
}

/// Parse Giphy API response into our GiphyGif format
fn parse_giphy_response(json: &serde_json::Value) -> Vec<GiphyGif> {
    let mut gifs = Vec::new();

    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
        for item in data {
            let id = item.get("id").and_then(|v| v.as_str()).unwrap_or_default();
            let title = item.get("title").and_then(|v| v.as_str()).unwrap_or_default();

            // Get the fixed_width version for consistent sizing
            let images = item.get("images");

            // Preview: use fixed_width_small for grid display
            let preview = images
                .and_then(|i| i.get("fixed_width_small"))
                .or_else(|| images.and_then(|i| i.get("fixed_width")));

            // Full: use fixed_width for button display (200px width)
            let full = images.and_then(|i| i.get("fixed_width"));

            if let (Some(preview), Some(full)) = (preview, full) {
                let preview_url = preview.get("url").and_then(|v| v.as_str()).unwrap_or_default();
                let url = full.get("url").and_then(|v| v.as_str()).unwrap_or_default();
                let width: u32 = full
                    .get("width")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(200);
                let height: u32 = full
                    .get("height")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(200);

                if !url.is_empty() {
                    gifs.push(GiphyGif {
                        id: id.to_string(),
                        title: title.to_string(),
                        preview_url: preview_url.to_string(),
                        url: url.to_string(),
                        width,
                        height,
                    });
                }
            }
        }
    }

    gifs
}
