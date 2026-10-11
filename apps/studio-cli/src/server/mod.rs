pub mod error;
mod routes;
mod security;
pub mod state;
mod web;

use std::path::PathBuf;

use axum::Router;
use axum::routing::{get, post};

pub use state::AppState;

/// The whole HTTP surface, in the order the contract lists it.
///
/// `web_root` is the built web client. When it is absent — a source checkout
/// that has not run `pnpm studio:build` — the API still serves, and the fallback page
/// says what to do instead of returning a bare 404.
pub fn router(state: AppState, web_root: Option<PathBuf>) -> Router {
    let api = Router::new()
        .route("/v1/capabilities", get(routes::meta::capabilities))
        .route(
            "/v1/projects/{projectId}/packaging/{format}",
            get(routes::packaging::read).put(routes::packaging::write),
        )
        .route(
            "/v1/projects",
            get(routes::projects::list).post(routes::projects::create),
        )
        .route(
            "/v1/projects/{projectId}",
            get(routes::projects::get)
                .patch(routes::projects::update)
                .delete(routes::projects::delete),
        )
        .route("/v1/projects/{projectId}/stores", get(routes::stores::list))
        .route(
            "/v1/projects/{projectId}/stores/{store}/apps",
            get(routes::stores::list_apps).post(routes::stores::create_app),
        )
        .route(
            "/v1/projects/{projectId}/store-apps",
            get(routes::stores::list_all_apps),
        )
        .route(
            "/v1/projects/{projectId}/store-apps/{storeAppId}",
            get(routes::stores::get_app)
                .patch(routes::stores::update_app)
                .delete(routes::stores::delete_app),
        )
        .route(
            "/v1/projects/{projectId}/store-apps/{storeAppId}/listing",
            get(routes::listing::get),
        )
        .route(
            "/v1/projects/{projectId}/store-apps/{storeAppId}/catalog",
            get(routes::catalog::tree),
        )
        .route(
            "/v1/projects/{projectId}/store-apps/{storeAppId}/catalog/file",
            get(routes::catalog::read_file).put(routes::catalog::write_file),
        )
        .route(
            "/v1/projects/{projectId}/store-apps/{storeAppId}/catalog/raw",
            get(routes::catalog::raw),
        )
        .route(
            "/v1/projects/{projectId}/store-apps/{storeAppId}/catalog/pull",
            post(routes::catalog::pull),
        )
        .route(
            "/v1/projects/{projectId}/store-apps/{storeAppId}/catalog/push",
            post(routes::catalog::push),
        )
        .route("/v1/runs/{runId}", get(routes::runs::get))
        .route("/v1/runs/{runId}/events", get(routes::runs::events))
        .route("/v1/runs/{runId}/cancel", post(routes::runs::cancel))
        .route("/v1/fs/browse", get(routes::fs::browse))
        .route("/openapi.json", get(routes::meta::openapi_document))
        .route("/reference", get(routes::meta::reference));

    web::attach(api, web_root)
        // Applied last so it wraps every route, including the static files.
        .layer(axum::middleware::from_fn(security::guard))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(state)
}
