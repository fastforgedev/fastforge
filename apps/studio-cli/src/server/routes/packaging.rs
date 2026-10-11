use axum::Json;
use axum::extract::{Path, State};
use studio_core::api::{ApiError, PutPackagingFileRequest};
use studio_core::model::PackagingFormat;

use crate::local::packaging;
use crate::server::error::{Result, ok};
use crate::server::state::AppState;

pub async fn read(
    State(state): State<AppState>,
    Path((id, format)): Path<(String, String)>,
) -> Result {
    let (_, project) = state.project_dir(&id).await?;
    Ok(ok(packaging::read(&project, parse_format(&format)?)?))
}

pub async fn write(
    State(state): State<AppState>,
    Path((id, format)): Path<(String, String)>,
    Json(request): Json<PutPackagingFileRequest>,
) -> Result {
    let (_, project) = state.project_dir(&id).await?;
    let _guard = state.packaging_write_lock().await;
    Ok(ok(packaging::write(
        &project,
        parse_format(&format)?,
        &request.content,
        &request.etag,
    )?))
}

fn parse_format(value: &str) -> std::result::Result<PackagingFormat, ApiError> {
    match value {
        "deb" => Ok(PackagingFormat::Deb),
        "pacman" => Ok(PackagingFormat::Pacman),
        "desktop" => Ok(PackagingFormat::Desktop),
        _ => Err(ApiError::bad_request(
            "INVALID_PACKAGING_FORMAT",
            "Expected deb, pacman or desktop",
        )),
    }
}
