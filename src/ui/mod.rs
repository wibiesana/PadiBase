use axum::{
    body::Body,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "src/ui/static/"]
struct Asset;

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

    let (resolved, content) = match Asset::get(target) {
        Some(c) => (target, c),
        None => match Asset::get("index.html") {
            Some(c) => ("index.html", c),
            None => return StatusCode::NOT_FOUND.into_response(),
        },
    };

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
