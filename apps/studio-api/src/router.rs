use studio_core::api::{ApiError, CreateProjectRequest, UpdateProjectRequest};
use studio_core::model::StoreKind;
use studio_core::openapi;
use worker::{Env, Headers, Method, Request, Response, Result};

use crate::error::{Handled, created, failed, internal, no_content, ok, respond};
use crate::{handlers, store};

pub async fn handle(mut request: Request, env: Env) -> Result<Response> {
    let url = request.url()?;
    let segments: Vec<String> = url
        .path_segments()
        .map(|segments| {
            segments
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let path: Vec<&str> = segments.iter().map(String::as_str).collect();
    let method = request.method();

    // The contract and the capability report are public: a client has to be
    // able to ask what this deployment is before it can authenticate to it.
    match (&method, path.as_slice()) {
        (Method::Get, ["openapi.json"]) => return respond(openapi::openapi_document()),
        (Method::Get, ["reference"]) => {
            let headers = Headers::new();
            headers.set("content-type", "text/html; charset=utf-8")?;
            return Ok(
                Response::ok(openapi::reference_html("/openapi.json"))?.with_headers(headers)
            );
        }
        (Method::Get, ["v1", "capabilities"]) => return ok(handlers::capabilities()),
        _ => {}
    }

    let owner = match authorize(&request, &env) {
        Ok(owner) => owner,
        Err(error) => return failed(&error),
    };
    let db = match store::database(&env) {
        Ok(db) => db,
        Err(error) => return failed(&error),
    };
    let now = now_rfc3339();

    match (&method, path.as_slice()) {
        (Method::Get, ["v1", "projects"]) => respond(handlers::list_projects(&db, &owner).await),
        (Method::Post, ["v1", "projects"]) => {
            let body: CreateProjectRequest = match parse_body(&mut request).await {
                Ok(body) => body,
                Err(error) => return failed(&error),
            };
            match handlers::create_project(&db, &owner, body, &now).await {
                Ok(project) => created(project),
                Err(error) => failed(&error),
            }
        }

        (Method::Get, ["v1", "projects", id]) => {
            respond(handlers::get_project(&db, &owner, id).await)
        }
        (Method::Patch, ["v1", "projects", id]) => {
            let body: UpdateProjectRequest = match parse_body(&mut request).await {
                Ok(body) => body,
                Err(error) => return failed(&error),
            };
            respond(handlers::update_project(&db, &owner, id, body, &now).await)
        }
        (Method::Delete, ["v1", "projects", id]) => {
            match handlers::delete_project(&db, &owner, id).await {
                Ok(()) => no_content(),
                Err(error) => failed(&error),
            }
        }

        (Method::Get, ["v1", "projects", id, "stores"]) => {
            respond(handlers::list_stores(&db, &env, &owner, id).await)
        }
        (Method::Get, ["v1", "projects", id, "stores", store, "apps"]) => {
            match StoreKind::parse(store) {
                Some(store) => {
                    respond(handlers::list_store_apps(&db, &owner, id, Some(store)).await)
                }
                None => failed(&ApiError::not_found(
                    "STORE_NOT_FOUND",
                    format!("`{store}` is not a known store"),
                )),
            }
        }
        (Method::Get, ["v1", "projects", id, "store-apps"]) => {
            respond(handlers::list_store_apps(&db, &owner, id, None).await)
        }
        (Method::Get, ["v1", "projects", id, "store-apps", store_app_id]) => {
            respond(handlers::get_store_app(&db, &owner, id, store_app_id).await)
        }

        (_, ["v1", "projects", _, "packaging", _]) => failed(&ApiError::capability_unavailable(
            "Packaging files require a local project checkout",
        )),

        // Catalogs live in R2 and nothing writes them here yet. Answering 501
        // rather than 404 says the endpoint is coming, so the client reports it
        // instead of hiding the feature.
        (_, ["v1", "projects", _, "store-apps", _, "catalog", ..]) => failed(&ApiError::new(
            501,
            "CATALOG_UNSUPPORTED",
            "Catalogs are not served by the hosted API yet",
        )),

        (_, ["v1", "runs", ..]) => failed(&ApiError::not_found("RUN_NOT_FOUND", "No such run")),

        // Local-only surfaces. There is no filesystem to browse here, and the
        // client already hides the picker because `capabilities.localFs` is
        // false — so this only answers a request that should not have happened.
        (_, ["v1", "fs", ..]) => failed(&ApiError::capability_unavailable(
            "The hosted API has no local filesystem to browse",
        )),

        _ => failed(&ApiError::not_found(
            "NOT_FOUND",
            format!("No route for {} {}", method, url.path()),
        )),
    }
}

async fn parse_body<T: serde::de::DeserializeOwned>(request: &mut Request) -> Handled<T> {
    request.json::<T>().await.map_err(|error| {
        ApiError::bad_request("INVALID_REQUEST", format!("Malformed body: {error}"))
    })
}

/// Bearer token authentication.
///
/// One shared token for now, held as a Worker secret. It fails closed: without
/// the secret configured the API answers 500 rather than letting requests
/// through, because "auth is not set up" must never read as "auth passed".
fn authorize(request: &Request, env: &Env) -> Handled<String> {
    let expected = env
        .secret("STUDIO_API_TOKEN")
        .map_err(|error| internal("STUDIO_API_TOKEN is not configured", error))?
        .to_string();

    let provided = request
        .headers()
        .get("authorization")
        .ok()
        .flatten()
        .unwrap_or_default();

    let Some(token) = provided.strip_prefix("Bearer ") else {
        return Err(ApiError::unauthorized(
            "Send `Authorization: Bearer <token>`",
        ));
    };
    if !constant_time_eq(token.as_bytes(), expected.as_bytes()) {
        return Err(ApiError::unauthorized("That token is not valid"));
    }

    // One token, one owner, until sign-in exists. The column is already there,
    // so adding real accounts does not reshape any of the queries.
    Ok(env
        .var("STUDIO_OWNER_ID")
        .map(|owner| owner.to_string())
        .unwrap_or_else(|_| "default".to_owned()))
}

/// Compares without an early exit, so a wrong token cannot be narrowed down by
/// timing how long the rejection took.
fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

/// RFC 3339, to the second — the format every timestamp on the wire uses.
fn now_rfc3339() -> String {
    let iso = worker::Date::now().to_string();
    // `Date::to_string` yields a full ISO 8601 string with milliseconds; the
    // contract's timestamps are second-resolution.
    match (iso.find('.'), iso.ends_with('Z')) {
        (Some(dot), true) => format!("{}Z", &iso[..dot]),
        _ => iso,
    }
}

#[cfg(test)]
mod tests {
    use super::constant_time_eq;

    #[test]
    fn tokens_compare_by_value_and_length() {
        assert!(constant_time_eq(b"secret", b"secret"));
        assert!(!constant_time_eq(b"secret", b"secreT"));
        assert!(!constant_time_eq(b"secret", b"secret2"));
        assert!(!constant_time_eq(b"", b"x"));
        assert!(constant_time_eq(b"", b""));
    }
}
