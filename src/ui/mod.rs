use axum::{
    body::Body,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "src/ui/static/"]
struct Asset;

pub async fn serve_admin_ui(path: axum::extract::Path<Option<String>>) -> impl IntoResponse {
    let file_path = path.0.unwrap_or_else(|| "index.html".to_string());
    let target = if file_path.is_empty() || file_path == "/" {
        "index.html"
    } else {
        &file_path
    };

    match Asset::get(target).or_else(|| Asset::get("index.html")) {
        Some(content) => {
            let mime = mime_guess::from_path(target).first_or_octet_stream();
            let cache_control = if target == "index.html" || target.ends_with(".js") || target.ends_with(".css") {
                "no-cache, no-store, must-revalidate"
            } else {
                "public, max-age=3600"
            };
            Response::builder()
                .header(
                    header::CONTENT_TYPE,
                    HeaderValue::from_str(mime.as_ref())
                        .unwrap_or(HeaderValue::from_static("text/html")),
                )
                .header(
                    header::CACHE_CONTROL,
                    HeaderValue::from_static(cache_control),
                )
                .body(Body::from(content.data))
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
