//! Static file serving with embedded assets

use axum::{
    body::Body,
    http::{header, Response, StatusCode},
    response::IntoResponse,
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "assets/web/"]
pub struct WebAssets;

/// Serve embedded static files
pub async fn serve_static(path: &str) -> impl IntoResponse {
    // Default to index.html for root
    let path = if path.is_empty() || path == "/" {
        "index.html"
    } else {
        path.trim_start_matches('/')
    };

    match WebAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path)
                .first_or_octet_stream()
                .to_string();

            let cache_control = if path.ends_with(".html") {
                "no-cache"
            } else {
                // Embedded assets are versioned with the binary
                "public, max-age=86400"
            };

            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime)
                .header(header::CACHE_CONTROL, cache_control)
                .body(Body::from(content.data.to_vec()))
                .unwrap()
        }
        None => {
            // For SPA routing, return index.html for unknown paths
            if !path.contains('.') {
                if let Some(content) = WebAssets::get("index.html") {
                    return Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, "text/html")
                        .header(header::CACHE_CONTROL, "no-cache")
                        .body(Body::from(content.data.to_vec()))
                        .unwrap();
                }
            }

            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Not found"))
                .unwrap()
        }
    }
}
