use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use studio_core::api::{ApiError, Capabilities, StudioMode, capabilities_for};
use tokio::sync::RwLock;

use crate::local::paths;
use crate::local::registry::{RegisteredProject, Registry};

/// Everything the handlers share.
///
/// The registry is cached; packaging writes share a lock. No project *contents*
/// are cached: stores, catalogs and platforms are read from the checkout on
/// every request, so Studio can never disagree with what `fastforge` would see.
#[derive(Clone)]
pub struct AppState {
    inner: Arc<Inner>,
}

struct Inner {
    registry_path: PathBuf,
    registry: RwLock<Registry>,
    packaging_write: tokio::sync::Mutex<()>,
    capabilities: Capabilities,
    /// The root of the browsable area for the directory picker.
    home: PathBuf,
}

impl AppState {
    pub fn new(registry_path: PathBuf, home: PathBuf) -> Result<Self> {
        let registry = Registry::load(&registry_path)?;
        Ok(Self {
            inner: Arc::new(Inner {
                registry_path,
                registry: RwLock::new(registry),
                packaging_write: tokio::sync::Mutex::new(()),
                capabilities: Capabilities {
                    // Local mode is where catalog sync belongs, but the store
                    // clients are not wired up yet — so this build says so
                    // rather than offering a button that answers 501. Flip it
                    // on with `routes::catalog::pull`.
                    catalog_sync: false,
                    ..capabilities_for(StudioMode::Local)
                },
                home,
            }),
        })
    }

    pub fn capabilities(&self) -> Capabilities {
        self.inner.capabilities
    }

    pub async fn packaging_write_lock(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.inner.packaging_write.lock().await
    }

    pub fn home(&self) -> &Path {
        &self.inner.home
    }

    pub async fn registry(&self) -> tokio::sync::RwLockReadGuard<'_, Registry> {
        self.inner.registry.read().await
    }

    /// Applies a change and persists it.
    ///
    /// The write lock is held across the save so a concurrent request cannot
    /// observe — or overwrite — a registry that is only half committed.
    pub async fn update_registry<T, F>(&self, change: F) -> Result<T, ApiError>
    where
        F: FnOnce(&mut Registry) -> Result<T, ApiError>,
    {
        let mut registry = self.inner.registry.write().await;
        let outcome = change(&mut registry)?;
        registry.save(&self.inner.registry_path).map_err(|error| {
            ApiError::internal(
                "REGISTRY_WRITE_FAILED",
                format!("Failed to save the project registry: {error}"),
            )
        })?;
        Ok(outcome)
    }

    pub async fn project(&self, id: &str) -> Result<RegisteredProject, ApiError> {
        self.registry()
            .await
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::not_found("PROJECT_NOT_FOUND", format!("No project `{id}`")))
    }

    /// Looks up a project and confirms its directory is still there.
    ///
    /// A registered directory that has been moved or deleted is a 404 with its
    /// own code: the project exists in Studio's list, but nothing can be read
    /// from it, and the UI should say so rather than show an empty page.
    pub async fn project_dir(&self, id: &str) -> Result<(RegisteredProject, PathBuf), ApiError> {
        let project = self.project(id).await?;
        if !project.path.is_dir() {
            return Err(ApiError::not_found(
                "PROJECT_PATH_MISSING",
                format!("{} no longer exists", project.path.display()),
            ));
        }
        let path = paths::canonicalize(&project.path)
            .map_err(|error| ApiError::internal("PROJECT_PATH_UNREADABLE", error.to_string()))?;
        Ok((project, path))
    }
}
