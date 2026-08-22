//! `GET /projects/{slug}/openapi.yml` — serve the YAML export as an
//! attachment download.

use crate::domain::project::ProjectSlug;
use crate::state::AppState;
use axum::extract::{Extension, Path};
use axum::http::StatusCode;
use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::{IntoResponse, Response};

/// `GET /projects/{slug}/openapi.yml` — render the project's OpenAPI
/// document and return it as a `Content-Disposition: attachment` YAML
/// download.
pub async fn yaml(Extension(state): Extension<AppState>, Path(slug): Path<String>) -> Response {
    let Ok(slug) = ProjectSlug::parse(&slug) else {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    };
    match crate::domain::use_cases::export_openapi::execute(state.repos(), slug.clone()).await {
        Ok(r) => {
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/yaml"));
            let disposition = format!("attachment; filename=\"{slug}-openapi.yml\"");
            headers
                .insert(header::CONTENT_DISPOSITION, HeaderValue::from_str(&disposition).unwrap());
            (headers, r.yaml).into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, "project not found").into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // No test here — covered by domain::use_cases::export_openapi tests.

    #[allow(dead_code)]
    fn _path_marker(_: Path<String>) {}
}
