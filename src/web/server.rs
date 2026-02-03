//! Axum web server setup

use axum::{
    extract::Request,
    routing::{delete, get, post, put},
    Router,
};
use std::net::SocketAddr;
use std::sync::{Arc, RwLock as StdRwLock};
use tokio::sync::{mpsc, RwLock as TokioRwLock};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use crate::config::Config;
use crate::profiles::{generate_default_profiles, ProfileManager};

use super::handlers::{
    self, AppState,
};
use super::static_files::serve_static;
use super::types::ConfigChangeEvent;

/// Start the web server
pub async fn start_server(
    config: Arc<TokioRwLock<Config>>,
    profile_manager: Arc<StdRwLock<ProfileManager>>,
    change_tx: mpsc::Sender<ConfigChangeEvent>,
) -> anyhow::Result<()> {
    let port = config.read().await.web.port;
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let app_state = Arc::new(AppState {
        config,
        profile_manager,
        change_tx,
    });

    // CORS layer for development
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // API routes
    let api_routes = Router::new()
        .route("/profiles", get(handlers::list_profiles))
        .route("/profiles", post(handlers::create_profile))
        .route("/profiles/{name}", get(handlers::get_profile))
        .route("/profiles/{name}", put(handlers::update_profile))
        .route("/profiles/{name}", delete(handlers::delete_profile))
        .route(
            "/profiles/{name}/buttons/{position}",
            put(handlers::update_button),
        )
        .route(
            "/profiles/{name}/buttons/{position}",
            delete(handlers::reset_button),
        )
        .route(
            "/profiles/{name}/buttons/swap",
            post(handlers::swap_buttons),
        )
        .route(
            "/profiles/{name}/has-defaults",
            get(handlers::has_profile_defaults),
        )
        .route("/profiles/{name}/reset", post(handlers::reset_profile))
        .route("/apps", get(handlers::list_apps))
        .route("/reload", post(handlers::reload_config))
        .route("/colors", get(handlers::get_colors))
        .route("/actions", get(handlers::get_actions))
        .route("/giphy/search", get(handlers::search_giphy))
        .with_state(app_state);

    // Static file fallback handler
    let static_handler = |req: Request| async move {
        let path = req.uri().path();
        serve_static(path).await
    };

    // Combine routes
    let app = Router::new()
        .nest("/api", api_routes)
        .fallback(static_handler)
        .layer(cors);

    info!("Web UI available at http://localhost:{}", port);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Initialize profile manager with profiles from config or defaults
pub fn init_profile_manager(config: &Config) -> ProfileManager {
    let profiles = if config.profiles.is_empty() {
        generate_default_profiles()
    } else {
        config.profiles.clone()
    };

    ProfileManager::new(profiles)
}
