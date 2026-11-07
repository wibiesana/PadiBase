use axum::{
    body::Body,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "src/ui/static/"]
struct Asset;

fn build_embedded_response(
    resolved: &str,
    content: rust_embed::EmbeddedFile,
    headers: &HeaderMap,
) -> Response {
    let etag = format!("\"{}\"", hex::encode(content.metadata.sha256_hash()));
    if let Some(inm) = headers.get(header::IF_NONE_MATCH).and_then(|v| v.to_str().ok()) {
        if inm.trim() == etag {
            return Response::builder()
                .status(StatusCode::NOT_MODIFIED)
                .header(
                    header::ETAG,
                    HeaderValue::from_str(&etag).unwrap_or(HeaderValue::from_static("")),
                )
                .body(Body::empty())
                .unwrap_or_else(|_| StatusCode::NOT_MODIFIED.into_response());
        }
    }

    let mime = mime_guess::from_path(resolved).first_or_octet_stream();
    let cache_control = if resolved == "index.html"
        || resolved.ends_with(".js")
        || resolved.ends_with(".css")
    {
        "public, max-age=0, must-revalidate"
    } else {
        "public, max-age=86400"
    };
    let mut builder = Response::builder()
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_str(mime.as_ref())
                .unwrap_or(HeaderValue::from_static("text/html")),
        )
        .header(
            header::CACHE_CONTROL,
            HeaderValue::from_static(cache_control),
        );
    if let Ok(etag_val) = HeaderValue::from_str(&etag) {
        builder = builder.header(header::ETAG, etag_val);
    }
    builder
        .body(Body::from(content.data))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

pub async fn serve_admin_ui(
    headers: HeaderMap,
    path: axum::extract::Path<Option<String>>,
) -> impl IntoResponse {
    let file_path = path.0.unwrap_or_else(|| "index.html".to_string());
    let target = if file_path.is_empty() || file_path == "/" {
        "index.html"
    } else {
        &file_path
    };

    // 1. In local development, prioritize live files from disk so UI updates are instant
    let disk_file = std::path::Path::new("src/ui/static").join(target);
    if disk_file.is_file() {
        if let Ok(bytes) = std::fs::read(&disk_file) {
            let mime = mime_guess::from_path(target).first_or_octet_stream();
            let builder = Response::builder()
                .header(
                    header::CONTENT_TYPE,
                    HeaderValue::from_str(mime.as_ref())
                        .unwrap_or(HeaderValue::from_static("text/html")),
                )
                .header(
                    header::CACHE_CONTROL,
                    HeaderValue::from_static("no-cache, no-store, must-revalidate"),
                );
            return builder
                .body(Body::from(bytes))
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }
    }

    // 2. Check embedded assets (for production binary)
    if let Some(content) = Asset::get(target) {
        return build_embedded_response(target, content, &headers);
    }

    // 3. If target has a static asset file extension (e.g. .js, .css, .png, .svg) and wasn't found, return 404
    let is_asset_extension = target.contains('.')
        && !target.ends_with(".html")
        && !target.ends_with(".htm");

    if is_asset_extension {
        return StatusCode::NOT_FOUND.into_response();
    }

    // 4. SPA Fallback: for routes like /tables, /tables/:name, /monitor, /storage, /settings, serve index.html
    // Prioritize live disk file if present
    let disk_index = std::path::Path::new("src/ui/static/index.html");
    if disk_index.is_file() {
        if let Ok(bytes) = std::fs::read(&disk_index) {
            let builder = Response::builder()
                .header(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("text/html; charset=utf-8"),
                )
                .header(
                    header::CACHE_CONTROL,
                    HeaderValue::from_static("no-cache, no-store, must-revalidate"),
                );
            return builder
                .body(Body::from(bytes))
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }
    }

    // Fallback to embedded index.html
    if let Some(content) = Asset::get("index.html") {
        return build_embedded_response("index.html", content, &headers);
    }

    StatusCode::NOT_FOUND.into_response()
}
