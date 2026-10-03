use crate::AppStoreConnectContext;
use crate::cli::GlobalArgs;
use crate::cli::commands::app::resolve_app;
use anyhow::{Context, Result};
use clap::Args;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

use super::VersionMetadata;
use super::screenshots::{self, ScreenshotManifestEntry};
use super::{APP_INFO_CATEGORY_FIELDS, AppInfoCategories};
use crate::types as asc_types;

/// Helper: fetch first page via typed client then follow `links.next` via raw HTTP.
async fn collect_pages<T>(
    ctx: &AppStoreConnectContext,
    resp: crate::ResponseValue<T>,
) -> Result<Vec<Value>>
where
    T: serde::Serialize + HasLinks,
{
    let inner = resp.into_inner();
    let mut all_data = Vec::new();
    for item in inner.data_iter() {
        all_data.push(serde_json::to_value(item)?);
    }
    let mut next_url = inner.next_link();
    while let Some(url) = next_url {
        let page: Value = ctx
            .http
            .get(&url)
            .send()
            .await
            .context("failed to fetch page")?
            .error_for_status()
            .context("API error")?
            .json()
            .await
            .context("failed to parse page")?;
        if let Some(data) = page.get("data").and_then(Value::as_array) {
            all_data.extend(data.iter().cloned());
        }
        next_url = page
            .get("links")
            .and_then(|l| l.get("next"))
            .and_then(Value::as_str)
            .map(|s| s.to_string());
    }
    Ok(all_data)
}

/// Trait for paginated typed responses.
trait HasLinks {
    type Item: serde::Serialize;
    fn data_iter(&self) -> Box<dyn Iterator<Item = &Self::Item> + '_>;
    fn next_link(&self) -> Option<String>;
}

macro_rules! impl_has_links {
    ($resp:ty, $item:ty) => {
        impl HasLinks for $resp {
            type Item = $item;
            fn data_iter(&self) -> Box<dyn Iterator<Item = &Self::Item> + '_> {
                Box::new(self.data.iter())
            }
            fn next_link(&self) -> Option<String> {
                self.links.next.clone()
            }
        }
    };
}

impl_has_links!(
    asc_types::AppStoreVersionsResponse,
    asc_types::AppStoreVersion
);
impl_has_links!(asc_types::AppInfosResponse, asc_types::AppInfo);
impl_has_links!(
    asc_types::AppInfoLocalizationsResponse,
    asc_types::AppInfoLocalization
);
impl_has_links!(
    asc_types::AppStoreVersionLocalizationsResponse,
    asc_types::AppStoreVersionLocalization
);
impl_has_links!(
    asc_types::AppScreenshotSetsResponse,
    asc_types::AppScreenshotSet
);
impl_has_links!(asc_types::AppScreenshotsResponse, asc_types::AppScreenshot);
impl_has_links!(asc_types::AppPreviewSetsResponse, asc_types::AppPreviewSet);
impl_has_links!(asc_types::AppPreviewsResponse, asc_types::AppPreview);

/// Fetch all app store versions via the generated typed client.
async fn fetch_versions(
    ctx: &AppStoreConnectContext,
    app_id: &str,
    platform: Option<&str>,
    version: Option<&str>,
) -> Result<Vec<Value>> {
    let query = version_query_params(platform, version)?;
    let response: Value = ctx
        .http
        .get(ctx.url(&format!("/v1/apps/{app_id}/appStoreVersions")))
        .query(&query)
        .send()
        .await
        .context("failed to fetch versions")?
        .error_for_status()
        .context("failed to fetch versions")?
        .json()
        .await
        .context("failed to decode versions")?;
    collect_value_pages(ctx, response).await
}

fn version_query_params(
    platform: Option<&str>,
    version: Option<&str>,
) -> Result<Vec<(&'static str, String)>> {
    let platform = platform.unwrap_or("IOS");
    if !matches!(platform, "IOS" | "MAC_OS" | "TV_OS" | "VISION_OS") {
        anyhow::bail!("unsupported App Store platform `{platform}`");
    }

    let mut query = vec![
        (
            "fields[appStoreVersions]",
            "platform,versionString,createdDate,copyright,appStoreState,appVersionState".into(),
        ),
        ("filter[platform]", platform.to_string()),
        ("limit", "200".into()),
    ];
    if let Some(version) = version {
        query.push(("filter[versionString]", version.to_string()));
    }
    Ok(query)
}

async fn collect_value_pages(ctx: &AppStoreConnectContext, mut page: Value) -> Result<Vec<Value>> {
    let mut all_data = Vec::new();
    loop {
        if let Some(data) = page.get("data").and_then(Value::as_array) {
            all_data.extend(data.iter().cloned());
        }
        let Some(next_url) = page
            .get("links")
            .and_then(|links| links.get("next"))
            .and_then(Value::as_str)
        else {
            break;
        };
        page = ctx
            .http
            .get(next_url)
            .send()
            .await
            .context("failed to fetch versions page")?
            .error_for_status()
            .context("failed to fetch versions page")?
            .json()
            .await
            .context("failed to decode versions page")?;
    }
    Ok(all_data)
}

/// Fetch all app infos via the generated typed client.
async fn fetch_app_infos(ctx: &AppStoreConnectContext, app_id: &str) -> Result<Vec<Value>> {
    let resp = ctx
        .client
        .apps_app_infos_get_to_many_related(
            app_id,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(200),
            None,
        )
        .await
        .map_err(|e| anyhow::anyhow!("failed to fetch app infos: {e}"))?;
    collect_pages(ctx, resp).await
}

async fn fetch_app_info_categories(
    ctx: &AppStoreConnectContext,
    app_info_id: &str,
) -> Result<AppInfoCategories> {
    let response: Value = ctx
        .http
        .get(ctx.url(&format!("/v1/appInfos/{app_info_id}")))
        .query(&[
            ("fields[appInfos]", APP_INFO_CATEGORY_FIELDS),
            ("include", APP_INFO_CATEGORY_FIELDS),
        ])
        .send()
        .await
        .context("failed to fetch App Store app info categories")?
        .error_for_status()
        .context("failed to fetch App Store app info categories")?
        .json()
        .await
        .context("failed to decode App Store app info categories")?;
    Ok(AppInfoCategories::from_relationships(
        &response["data"]["relationships"],
    ))
}

/// Fetch all app info localizations via the generated typed client.
async fn fetch_app_info_localizations(
    ctx: &AppStoreConnectContext,
    app_info_id: &str,
) -> Result<Vec<Value>> {
    let resp = ctx
        .client
        .app_infos_app_info_localizations_get_to_many_related(
            app_info_id,
            None,
            None,
            None,
            None,
            Some(200),
        )
        .await
        .map_err(|e| anyhow::anyhow!("failed to fetch app info localizations: {e}"))?;
    collect_pages(ctx, resp).await
}

/// Fetch all version localizations via the generated typed client.
async fn fetch_version_localizations(
    ctx: &AppStoreConnectContext,
    version_id: &str,
) -> Result<Vec<Value>> {
    let resp = ctx
        .client
        .app_store_versions_app_store_version_localizations_get_to_many_related(
            version_id,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(200),
            None,
            None,
            None,
        )
        .await
        .map_err(|e| anyhow::anyhow!("failed to fetch version localizations: {e}"))?;
    collect_pages(ctx, resp).await
}

/// Fetch all screenshot sets via the generated typed client.
async fn fetch_screenshot_sets(ctx: &AppStoreConnectContext, vloc_id: &str) -> Result<Vec<Value>> {
    let resp = ctx
        .client
        .app_store_version_localizations_app_screenshot_sets_get_to_many_related(
            vloc_id,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(200),
            None,
        )
        .await
        .map_err(|e| anyhow::anyhow!("failed to fetch screenshot sets: {e}"))?;
    collect_pages(ctx, resp).await
}

/// Fetch all screenshots in a set via the generated typed client.
async fn fetch_screenshots(ctx: &AppStoreConnectContext, ss_set_id: &str) -> Result<Vec<Value>> {
    let resp = ctx
        .client
        .app_screenshot_sets_app_screenshots_get_to_many_related(
            ss_set_id,
            None,
            None,
            None,
            Some(200),
        )
        .await
        .map_err(|e| anyhow::anyhow!("failed to fetch screenshots: {e}"))?;
    collect_pages(ctx, resp).await
}

/// Fetch all preview sets via the generated typed client.
async fn fetch_preview_sets(ctx: &AppStoreConnectContext, vloc_id: &str) -> Result<Vec<Value>> {
    let resp = ctx
        .client
        .app_store_version_localizations_app_preview_sets_get_to_many_related(
            vloc_id,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(200),
            None,
        )
        .await
        .map_err(|e| anyhow::anyhow!("failed to fetch preview sets: {e}"))?;
    collect_pages(ctx, resp).await
}

/// Fetch all previews in a set via the generated typed client.
async fn fetch_previews(ctx: &AppStoreConnectContext, pv_set_id: &str) -> Result<Vec<Value>> {
    let resp = ctx
        .client
        .app_preview_sets_app_previews_get_to_many_related(pv_set_id, None, None, None, Some(200))
        .await
        .map_err(|e| anyhow::anyhow!("failed to fetch previews: {e}"))?;
    collect_pages(ctx, resp).await
}

/// Resolve an image download URL from imageAsset templateUrl + dimensions.
fn resolve_image_url(attrs: &serde_json::Map<String, Value>) -> Option<String> {
    let image_asset = attrs.get("imageAsset")?;
    let template = image_asset.get("templateUrl")?.as_str()?;
    let width = image_asset.get("width")?.as_i64()?;
    let height = image_asset.get("height")?.as_i64()?;
    let ext = Path::new(attrs.get("fileName")?.as_str()?)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png");
    Some(
        template
            .replace("{w}", &width.to_string())
            .replace("{h}", &height.to_string())
            .replace("{f}", ext),
    )
}

/// Resolve a preview video download URL from previewFrameTimeCode + videoUrl/templateUrl.
fn resolve_video_url(attrs: &serde_json::Map<String, Value>) -> Option<String> {
    // Prefer direct videoUrl
    if let Some(url) = attrs
        .get("videoUrl")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    {
        return Some(url.to_string());
    }
    // Fallback: videoUrl may need a separate fetch with specific fields
    // For now, return None — will log a warning
    None
}

#[derive(Args, Debug)]
pub struct PullArgs {
    /// App bundle ID or app ID
    #[arg(long = "app")]
    pub app: String,

    /// Optional version string filter (e.g. "1.2.3"). If omitted, pulls all versions.
    #[arg(long = "version")]
    pub version: Option<String>,

    /// Platform filter (e.g. "IOS", "MAC_OS"). Defaults to "IOS".
    #[arg(long = "platform")]
    pub platform: Option<String>,

    /// Output directory root. Defaults to `.fastforge/stores/appstore/{bundle_id}/`
    #[arg(long = "output")]
    pub output: Option<String>,
}

/// Download a file from a URL and save it to disk.
async fn download_file(ctx: &AppStoreConnectContext, url: &str, dest: &Path) -> Result<()> {
    let response = ctx
        .http
        .get(url)
        .send()
        .await
        .context("failed to download file")?
        .error_for_status()
        .context("download returned error")?;
    let bytes = response
        .bytes()
        .await
        .context("failed to read response body")?;
    tokio::fs::write(dest, &bytes)
        .await
        .with_context(|| format!("failed to write {}", dest.display()))?;
    Ok(())
}

/// Ensure a directory exists.
fn ensure_dir(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path)
        .with_context(|| format!("failed to create directory {}", path.display()))
}

/// Write a YAML value to a file.
fn write_yaml<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let content = serde_yaml::to_string(value).context("failed to serialize YAML")?;
    std::fs::write(path, &content).with_context(|| format!("failed to write {}", path.display()))
}

pub async fn execute(args: &PullArgs, _global: &GlobalArgs) -> Result<()> {
    let ctx = AppStoreConnectContext::from_env()?;
    execute_with_context(args, &ctx).await
}

/// Pull catalog data using an existing App Store Connect context.
pub async fn execute_with_context(args: &PullArgs, ctx: &AppStoreConnectContext) -> Result<()> {
    let start = std::time::Instant::now();

    // 1. Resolve app
    eprintln!("🔍 Resolving app '{}'...", args.app);
    let app_row = resolve_app(ctx, &args.app).await?;
    let bundle_id = &app_row.bundle_id;

    let output_root = args
        .output
        .as_deref()
        .map(Path::new)
        .unwrap_or_else(|| Path::new(".fastforge/stores/appstore"))
        .join(bundle_id);
    let staged = StagedCatalog::new(&output_root)?;
    let pulled_count = pull_into_directory(args, ctx, &app_row, staged.path())
        .await
        .with_context(|| {
            format!(
                "catalog pull failed; discarded the incomplete snapshot and left {} unchanged",
                output_root.display()
            )
        })?;
    staged.commit()?;

    let elapsed = start.elapsed();
    eprintln!(
        "\n✅ Pull complete: {pulled_count} resources synced to {} in {elapsed:?}",
        output_root.display()
    );
    Ok(())
}

async fn pull_into_directory(
    args: &PullArgs,
    ctx: &AppStoreConnectContext,
    app_row: &crate::cli::commands::app::AppRow,
    output_root: &Path,
) -> Result<u64> {
    let mut pulled_count = 0u64;
    let bundle_id = &app_row.bundle_id;
    let app_id = &app_row.id;

    // Write app.yaml
    let app_yaml = asc_types::AppAttributes {
        bundle_id: Some(bundle_id.clone()),
        name: Some(app_row.name.clone()),
        primary_locale: Some(app_row.locale.clone()),
        sku: Some(app_row.sku.clone()),
        ..Default::default()
    };
    write_yaml(&output_root.join("app.yaml"), &app_yaml)?;
    eprintln!("  ✓ app.yaml");
    pulled_count += 1;

    // 2. Fetch appInfos
    eprintln!("📦 Fetching app info...");
    let app_infos = fetch_app_infos(ctx, app_id).await?;

    if let Some(app_info) = app_infos.first() {
        let info_id = app_info["id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("App Store app info is missing its id"))?;
        let categories = fetch_app_info_categories(ctx, info_id).await?;
        if !categories.is_empty() {
            write_yaml(&output_root.join("app_info.yaml"), &categories)?;
            eprintln!("  ✓ app_info.yaml");
            pulled_count += 1;
        }
    }

    for app_info in &app_infos {
        let info_id = app_info["id"].as_str().unwrap_or_default().to_string();

        // 3. Fetch appInfoLocalizations
        let localizations = fetch_app_info_localizations(ctx, &info_id).await?;
        let localizations_dir = output_root.join("info");
        ensure_dir(&localizations_dir)?;

        for loc in &localizations {
            let locale = loc["attributes"]["locale"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();

            let yaml: asc_types::AppInfoLocalizationAttributes =
                serde_json::from_value(loc["attributes"].clone())
                    .context("failed to parse app info localization attributes")?;
            let loc_path = localizations_dir.join(format!("{locale}.yaml"));
            write_yaml(&loc_path, &yaml)?;
            eprintln!("  ✓ info/{locale}.yaml");
            pulled_count += 1;
        }
    }

    // 4. Fetch appStoreVersions
    eprintln!("📱 Fetching app store versions...");
    let mut versions = fetch_versions(
        ctx,
        app_id,
        args.platform.as_deref(),
        args.version.as_deref(),
    )
    .await?;
    sort_versions_for_asset_dedup(&mut versions);

    let mut media_signatures: HashMap<(String, String, &'static str, String), Value> =
        HashMap::new();
    let mut version_localization_attrs: HashMap<
        (String, String),
        asc_types::AppStoreVersionLocalizationAttributes,
    > = HashMap::new();
    let mut version_copyrights: HashMap<String, String> = HashMap::new();
    let mut review_detail_attrs: HashMap<String, asc_types::AppStoreReviewDetailAttributes> =
        HashMap::new();

    for version in &versions {
        let version_id = version["id"].as_str().unwrap_or_default().to_string();
        let platform = version["attributes"]["platform"]
            .as_str()
            .unwrap_or("IOS")
            .to_string();
        let version_string = version["attributes"]["versionString"]
            .as_str()
            .unwrap_or("0.0.0")
            .to_string();
        let state = version["attributes"]["appStoreState"]
            .as_str()
            .or_else(|| version["attributes"]["appVersionState"].as_str())
            .map(str::to_owned);
        let version_dir = output_root
            .join("versions")
            .join(&platform)
            .join(&version_string);
        ensure_dir(&version_dir)?;

        let copyright = version["attributes"]["copyright"].as_str();
        write_version_file(
            &version_dir,
            &version_id,
            &platform,
            &version_string,
            state,
            copyright,
            version_copyrights.get(&platform),
        )?;
        eprintln!("  ✓ versions/{platform}/{version_string}/version.yaml");
        pulled_count += 1;
        if let Some(copyright) = copyright {
            version_copyrights.insert(platform.clone(), copyright.to_string());
        }

        // 5. Fetch version localizations
        let version_locs = fetch_version_localizations(ctx, &version_id).await?;

        for vloc in &version_locs {
            let locale = vloc["attributes"]["locale"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();
            let vloc_dir = version_dir.join(&locale);

            let vloc_attrs: asc_types::AppStoreVersionLocalizationAttributes =
                serde_json::from_value(vloc["attributes"].clone())
                    .context("failed to parse version localization attributes")?;
            let vloc_path = vloc_dir.join("localization.yaml");
            let legacy_vloc_path = vloc_dir.join("version.yaml");
            let previous_attrs = version_localization_attrs
                .get(&(platform.clone(), locale.clone()))
                .cloned();
            let writable_attrs =
                diff_version_localization_attrs(&vloc_attrs, previous_attrs.as_ref());
            version_localization_attrs.insert((platform.clone(), locale.clone()), vloc_attrs);
            if has_version_localization_changes(&writable_attrs) {
                ensure_dir(&vloc_dir)?;
                write_yaml(&vloc_path, &writable_attrs)?;
                eprintln!("  ✓ versions/{platform}/{version_string}/{locale}/localization.yaml");
                pulled_count += 1;
            }
            if legacy_vloc_path.exists() {
                std::fs::remove_file(&legacy_vloc_path).with_context(|| {
                    format!(
                        "failed to remove legacy file {}",
                        legacy_vloc_path.display()
                    )
                })?;
            }

            // 6. Fetch screenshot sets
            let vloc_id = vloc["id"].as_str().unwrap_or_default().to_string();
            let screenshot_sets = fetch_screenshot_sets(ctx, &vloc_id).await?;

            for ss_set in &screenshot_sets {
                let ss_set_id = ss_set["id"].as_str().unwrap_or_default().to_string();
                let display_type = ss_set["attributes"]["screenshotDisplayType"]
                    .as_str()
                    .unwrap_or("UNKNOWN")
                    .to_string();
                let screenshots = fetch_screenshots(ctx, &ss_set_id).await?;
                let signature = media_set_signature(&ss_set["attributes"], &screenshots);
                let signature_key = (
                    platform.clone(),
                    locale.clone(),
                    "screenshots",
                    display_type.clone(),
                );
                if media_signatures.get(&signature_key) == Some(&signature) {
                    eprintln!("  • skipped unchanged screenshots/{display_type}");
                    continue;
                }
                media_signatures.insert(signature_key, signature);

                let screenshots_dir = vloc_dir.join("screenshots").join(&display_type);
                screenshots::prepare_screenshot_directory(&screenshots_dir)?;
                let mut manifest_entries = Vec::new();

                // 7. Fetch screenshots
                for (idx, screenshot) in screenshots.iter().enumerate() {
                    let ss_id = screenshot["id"].as_str().unwrap_or_default().to_string();
                    let ss_attrs = screenshot["attributes"]
                        .as_object()
                        .cloned()
                        .unwrap_or_default();

                    let seq = idx + 1;
                    let file_name = ss_attrs
                        .get("fileName")
                        .and_then(Value::as_str)
                        .unwrap_or("screenshot.png");
                    let extension = Path::new(file_name)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("png")
                        .to_ascii_lowercase();
                    let screenshot_filename =
                        screenshots::pulled_screenshot_file_name(seq, &extension);
                    let screenshot_path = screenshots_dir.join(&screenshot_filename);

                    if let Some(image_url) = resolve_image_url(&ss_attrs) {
                        match download_file(ctx, &image_url, &screenshot_path).await {
                            Ok(_) => {
                                manifest_entries.push(ScreenshotManifestEntry {
                                    file_name: screenshot_filename.clone(),
                                    remote_id: ss_id.clone(),
                                    checksum: screenshots::screenshot_checksum(&screenshot_path)?,
                                });
                                eprintln!(
                                    "  ✓ .../screenshots/{display_type}/{screenshot_filename}"
                                );
                                pulled_count += 1;
                            }
                            Err(e) => eprintln!("  ⚠ failed to download screenshot {ss_id}: {e}"),
                        }
                    } else {
                        eprintln!(
                            "  ⚠ screenshot {ss_id} has no downloadable URL (state: not ready)"
                        );
                        std::fs::write(&screenshot_path, "")?;
                    }
                }
                screenshots::write_screenshot_manifest(
                    output_root,
                    &screenshots_dir,
                    &manifest_entries,
                )?;
            }

            // 8. Fetch preview sets
            let vloc_id = vloc["id"].as_str().unwrap_or_default().to_string();
            let preview_sets = fetch_preview_sets(ctx, &vloc_id).await?;

            for pv_set in &preview_sets {
                let pv_set_id = pv_set["id"].as_str().unwrap_or_default().to_string();
                let preview_type = pv_set["attributes"]["previewType"]
                    .as_str()
                    .unwrap_or("UNKNOWN")
                    .to_string();
                let previews = fetch_previews(ctx, &pv_set_id).await?;
                let signature = media_set_signature(&pv_set["attributes"], &previews);
                let signature_key = (
                    platform.clone(),
                    locale.clone(),
                    "previews",
                    preview_type.clone(),
                );
                if media_signatures.get(&signature_key) == Some(&signature) {
                    eprintln!("  • skipped unchanged previews/{preview_type}");
                    continue;
                }
                media_signatures.insert(signature_key, signature);

                let previews_dir = vloc_dir.join("previews").join(&preview_type);
                ensure_dir(&previews_dir)?;

                // 9. Fetch previews
                for (idx, preview) in previews.iter().enumerate() {
                    let pv_id = preview["id"].as_str().unwrap_or_default().to_string();
                    let pv_attrs = preview["attributes"]
                        .as_object()
                        .cloned()
                        .unwrap_or_default();

                    let seq = idx + 1;
                    let file_name = pv_attrs
                        .get("fileName")
                        .and_then(Value::as_str)
                        .unwrap_or("preview.mov");
                    let extension = Path::new(file_name)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("mov");
                    let preview_filename = format!("{:03}_{}.{}", seq, pv_id, extension);
                    let preview_path = previews_dir.join(&preview_filename);

                    if let Some(video_url) = resolve_video_url(&pv_attrs) {
                        match download_file(ctx, &video_url, &preview_path).await {
                            Ok(_) => {
                                eprintln!("  ✓ .../previews/{preview_type}/{preview_filename}")
                            }
                            Err(e) => eprintln!("  ⚠ failed to download preview {pv_id}: {e}"),
                        }
                        pulled_count += 1;
                    } else {
                        eprintln!("  ⚠ preview {pv_id} has no downloadable URL (state: not ready)");
                        std::fs::write(&preview_path, "")?;
                    }
                }
            }

            // 10. Fetch review detail (审核信息) for this version (with dedup)
            let prev_review = review_detail_attrs.get(&platform).cloned();
            let pulled = fetch_and_write_review_detail(
                ctx,
                &version_id,
                &version_dir,
                &platform,
                prev_review.as_ref(),
                &mut review_detail_attrs,
            )
            .await?;
            pulled_count += pulled;
        }
    }

    Ok(pulled_count)
}

struct StagedCatalog {
    target: std::path::PathBuf,
    staging: std::path::PathBuf,
    active: bool,
}

impl StagedCatalog {
    fn new(target: &Path) -> Result<Self> {
        let parent = target.parent().unwrap_or_else(|| Path::new("."));
        ensure_dir(parent)?;
        let name = target
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("catalog");
        let staging = parent.join(format!(".{name}.pull-{}.tmp", uuid::Uuid::new_v4()));
        ensure_dir(&staging)?;
        Ok(Self {
            target: target.to_path_buf(),
            staging,
            active: true,
        })
    }

    fn path(&self) -> &Path {
        &self.staging
    }

    fn commit(mut self) -> Result<()> {
        if !self.target.exists() {
            std::fs::rename(&self.staging, &self.target).with_context(|| {
                format!(
                    "failed to move completed catalog into {}",
                    self.target.display()
                )
            })?;
            self.active = false;
            return Ok(());
        }

        let parent = self.target.parent().unwrap_or_else(|| Path::new("."));
        let name = self
            .target
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("catalog");
        let backup = parent.join(format!(".{name}.pull-{}.backup", uuid::Uuid::new_v4()));
        std::fs::rename(&self.target, &backup).with_context(|| {
            format!(
                "failed to preserve existing catalog {}",
                self.target.display()
            )
        })?;
        if let Err(error) = std::fs::rename(&self.staging, &self.target) {
            let restore = std::fs::rename(&backup, &self.target);
            return match restore {
                Ok(()) => Err(error).with_context(|| {
                    format!(
                        "failed to replace catalog {}; the previous snapshot was restored",
                        self.target.display()
                    )
                }),
                Err(restore_error) => Err(anyhow::anyhow!(
                    "failed to replace catalog {} ({error}) and restore backup {} ({restore_error})",
                    self.target.display(),
                    backup.display()
                )),
            };
        }
        self.active = false;
        if let Err(error) = std::fs::remove_dir_all(&backup) {
            eprintln!(
                "  ⚠ completed catalog installed, but failed to remove backup {}: {error}",
                backup.display()
            );
        }
        Ok(())
    }
}

impl Drop for StagedCatalog {
    fn drop(&mut self) {
        if self.active && self.staging.exists() {
            let _ = std::fs::remove_dir_all(&self.staging);
        }
    }
}

/// Fetch review detail and attachments for a version and write to local YAML.
/// Skips writing if the attributes are unchanged from a previous version's review.
async fn fetch_and_write_review_detail(
    ctx: &AppStoreConnectContext,
    version_id: &str,
    version_dir: &Path,
    platform: &str,
    previous: Option<&asc_types::AppStoreReviewDetailAttributes>,
    attrs_map: &mut HashMap<String, asc_types::AppStoreReviewDetailAttributes>,
) -> Result<u64> {
    let mut count = 0u64;

    // Fetch review detail via raw HTTP
    let fields = [
        "contactEmail",
        "contactFirstName",
        "contactLastName",
        "contactPhone",
        "demoAccountName",
        "demoAccountPassword",
        "demoAccountRequired",
        "notes",
    ];
    let response = ctx
        .http
        .get(ctx.url(&format!(
            "/v1/appStoreVersions/{version_id}/appStoreReviewDetail"
        )))
        .query(&[("fields[appStoreReviewDetails]", &fields.join(","))])
        .send()
        .await
        .context("failed to fetch review detail")?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(count);
    }

    let response: serde_json::Value = response
        .error_for_status()
        .context("failed to fetch review detail")?
        .json()
        .await?;

    let Some(data) = review_detail_data(&response) else {
        return Ok(count);
    };
    let detail_id = data
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let attrs_obj = data
        .get("attributes")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("missing review detail attributes"))?;

    // Build current review attributes
    let current = asc_types::AppStoreReviewDetailAttributes {
        contact_email: attrs_obj
            .get("contactEmail")
            .and_then(Value::as_str)
            .map(|s| s.to_string()),
        contact_first_name: attrs_obj
            .get("contactFirstName")
            .and_then(Value::as_str)
            .map(|s| s.to_string()),
        contact_last_name: attrs_obj
            .get("contactLastName")
            .and_then(Value::as_str)
            .map(|s| s.to_string()),
        contact_phone: attrs_obj
            .get("contactPhone")
            .and_then(Value::as_str)
            .map(|s| s.to_string()),
        demo_account_name: attrs_obj
            .get("demoAccountName")
            .and_then(Value::as_str)
            .map(|s| s.to_string()),
        demo_account_password: attrs_obj
            .get("demoAccountPassword")
            .and_then(Value::as_str)
            .map(|s| s.to_string()),
        demo_account_required: attrs_obj
            .get("demoAccountRequired")
            .and_then(Value::as_bool),
        notes: attrs_obj
            .get("notes")
            .and_then(Value::as_str)
            .map(|s| s.to_string()),
    };

    // Check if unchanged from previous version (dedup)
    let has_changes = match previous {
        Some(prev) => {
            current.contact_email != prev.contact_email
                || current.contact_first_name != prev.contact_first_name
                || current.contact_last_name != prev.contact_last_name
                || current.contact_phone != prev.contact_phone
                || current.demo_account_name != prev.demo_account_name
                || current.demo_account_password != prev.demo_account_password
                || current.demo_account_required != prev.demo_account_required
                || current.notes != prev.notes
        }
        None => true,
    };

    // Store for future dedup (even if we skip writing, we need to track the latest)
    attrs_map.insert(platform.to_string(), current.clone());

    if !has_changes {
        eprintln!("  • skipped unchanged review.yaml");
        // Still fetch attachments for the latest version that has changes
        // For unchanged versions, skip attachments too
        return Ok(count);
    }

    // Write review detail attributes as YAML, preserving _id
    #[derive(serde::Serialize)]
    struct ReviewWithId {
        _id: Option<String>,
        #[serde(flatten)]
        attributes: asc_types::AppStoreReviewDetailAttributes,
    }

    let review_path = version_dir.join("review.yaml");
    write_yaml(
        &review_path,
        &ReviewWithId {
            _id: Some(detail_id.clone()),
            attributes: current,
        },
    )?;
    eprintln!("  ✓ review.yaml (id: {detail_id})");
    count += 1;

    // Fetch attachments if any
    let att_fields = [
        "fileName",
        "fileSize",
        "sourceFileChecksum",
        "assetDeliveryState",
    ];
    let attachments_response = ctx
        .http
        .get(ctx.url(&format!(
            "/v1/appStoreReviewDetails/{detail_id}/appStoreReviewAttachments"
        )))
        .query(&[("fields[appStoreReviewAttachments]", &att_fields.join(","))])
        .send()
        .await
        .context("failed to fetch review attachments")?;

    if attachments_response.status() != reqwest::StatusCode::NOT_FOUND {
        let attachments_value: serde_json::Value = attachments_response
            .error_for_status()
            .context("failed to fetch review attachments")?
            .json()
            .await?;

        if let Some(attachments) = attachments_value
            .get("data")
            .and_then(Value::as_array)
            .filter(|a| !a.is_empty())
        {
            let review_dir = version_dir.join("review.d");
            ensure_dir(&review_dir)?;

            for attachment in attachments {
                let att_id = attachment
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let a_attrs = attachment
                    .get("attributes")
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_default();

                #[derive(serde::Serialize)]
                struct AttachmentYaml {
                    _id: Option<String>,
                    file_name: Option<String>,
                    file_size: Option<i64>,
                    source_file_checksum: Option<String>,
                }

                let att_path = review_dir.join(format!("{att_id}.yaml"));
                write_yaml(
                    &att_path,
                    &AttachmentYaml {
                        _id: Some(att_id.clone()),
                        file_name: a_attrs
                            .get("fileName")
                            .and_then(Value::as_str)
                            .map(|s| s.to_string()),
                        file_size: a_attrs.get("fileSize").and_then(Value::as_i64),
                        source_file_checksum: a_attrs
                            .get("sourceFileChecksum")
                            .and_then(Value::as_str)
                            .map(|s| s.to_string()),
                    },
                )?;
                eprintln!("  ✓ review.d/{att_id}.yaml");
                count += 1;
            }
        }
    }

    Ok(count)
}

fn review_detail_data(response: &Value) -> Option<&Value> {
    response.get("data").filter(|data| !data.is_null())
}

fn sort_versions_for_asset_dedup(versions: &mut [Value]) {
    versions.sort_by(|a, b| {
        let a_platform = a["attributes"]["platform"].as_str().unwrap_or("");
        let b_platform = b["attributes"]["platform"].as_str().unwrap_or("");
        let a_created = a["attributes"]["createdDate"].as_str().unwrap_or("");
        let b_created = b["attributes"]["createdDate"].as_str().unwrap_or("");
        let a_version = a["attributes"]["versionString"].as_str().unwrap_or("");
        let b_version = b["attributes"]["versionString"].as_str().unwrap_or("");

        a_platform
            .cmp(b_platform)
            .then_with(|| a_created.cmp(b_created))
            .then_with(|| compare_version_strings(a_version, b_version))
    });
}

fn compare_version_strings(a: &str, b: &str) -> std::cmp::Ordering {
    let mut a_parts = a.split('.');
    let mut b_parts = b.split('.');

    loop {
        match (a_parts.next(), b_parts.next()) {
            (Some(a_part), Some(b_part)) => {
                let ordering = match (a_part.parse::<u64>(), b_part.parse::<u64>()) {
                    (Ok(a_num), Ok(b_num)) => a_num.cmp(&b_num),
                    _ => a_part.cmp(b_part),
                };
                if ordering != std::cmp::Ordering::Equal {
                    return ordering;
                }
            }
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (None, None) => return std::cmp::Ordering::Equal,
        }
    }
}

fn diff_version_localization_attrs(
    current: &asc_types::AppStoreVersionLocalizationAttributes,
    previous: Option<&asc_types::AppStoreVersionLocalizationAttributes>,
) -> asc_types::AppStoreVersionLocalizationAttributes {
    let mut diff = asc_types::AppStoreVersionLocalizationAttributes::default();

    if changed(
        &current.description,
        previous.and_then(|attrs| attrs.description.as_ref()),
    ) {
        diff.description = current.description.clone();
    }
    if changed(
        &current.keywords,
        previous.and_then(|attrs| attrs.keywords.as_ref()),
    ) {
        diff.keywords = current.keywords.clone();
    }
    if changed(
        &current.marketing_url,
        previous.and_then(|attrs| attrs.marketing_url.as_ref()),
    ) {
        diff.marketing_url = current.marketing_url.clone();
    }
    if changed(
        &current.promotional_text,
        previous.and_then(|attrs| attrs.promotional_text.as_ref()),
    ) {
        diff.promotional_text = current.promotional_text.clone();
    }
    if changed(
        &current.support_url,
        previous.and_then(|attrs| attrs.support_url.as_ref()),
    ) {
        diff.support_url = current.support_url.clone();
    }
    if changed(
        &current.whats_new,
        previous.and_then(|attrs| attrs.whats_new.as_ref()),
    ) {
        diff.whats_new = current.whats_new.clone();
    }

    diff
}

fn changed(current: &Option<String>, previous: Option<&String>) -> bool {
    current.as_ref() != previous
}

fn has_copyright_changed(current: &str, previous: Option<&String>) -> bool {
    previous.map(String::as_str) != Some(current)
}

fn write_version_file(
    version_dir: &Path,
    id: &str,
    platform: &str,
    version_string: &str,
    state: Option<String>,
    copyright: Option<&str>,
    previous: Option<&String>,
) -> Result<()> {
    let path = version_dir.join("version.yaml");
    let copyright = copyright
        .filter(|current| has_copyright_changed(current, previous))
        .map(str::to_owned);
    write_yaml(
        &path,
        &VersionMetadata {
            id: Some(id.to_string()),
            platform: Some(platform.to_string()),
            version_string: Some(version_string.to_string()),
            state,
            copyright,
        },
    )
}

fn has_version_localization_changes(
    attrs: &asc_types::AppStoreVersionLocalizationAttributes,
) -> bool {
    attrs.description.is_some()
        || attrs.keywords.is_some()
        || attrs.marketing_url.is_some()
        || attrs.promotional_text.is_some()
        || attrs.support_url.is_some()
        || attrs.whats_new.is_some()
}

fn media_set_signature(set_attrs: &Value, media: &[Value]) -> Value {
    let mut items: Vec<Value> = media
        .iter()
        .map(|item| media_asset_signature(&item["attributes"]))
        .collect();

    items.sort_by_key(|item| item.to_string());

    serde_json::json!({
        "set": stable_media_attrs(set_attrs),
        "items": items,
    })
}

fn media_asset_signature(attrs: &Value) -> Value {
    serde_json::json!({
        "fileName": attrs.get("fileName"),
        "fileSize": attrs.get("fileSize"),
        "sourceFileChecksum": attrs.get("sourceFileChecksum"),
        "assetType": attrs.get("assetType"),
        "mimeType": attrs.get("mimeType"),
        "previewFrameTimeCode": attrs.get("previewFrameTimeCode"),
        "imageAsset": stable_image_asset(attrs.get("imageAsset")),
        "previewImage": stable_image_asset(attrs.get("previewImage")),
        "previewFrameImage": stable_image_asset(attrs.get("previewFrameImage")),
    })
}

fn stable_media_attrs(attrs: &Value) -> Value {
    serde_json::json!({
        "screenshotDisplayType": attrs.get("screenshotDisplayType"),
        "previewType": attrs.get("previewType"),
    })
}

fn stable_image_asset(asset: Option<&Value>) -> Value {
    let Some(asset) = asset else {
        return Value::Null;
    };

    serde_json::json!({
        "width": asset.get("width"),
        "height": asset.get("height"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_store_version_attributes_preserve_copyright() {
        let attrs: asc_types::AppStoreVersionAttributes =
            serde_json::from_value(serde_json::json!({
                "platform": "IOS",
                "versionString": "1.2.3",
                "copyright": "2026 Example Inc."
            }))
            .unwrap();

        assert_eq!(attrs.copyright.as_deref(), Some("2026 Example Inc."));
        assert_eq!(
            serde_json::to_value(attrs).unwrap()["copyright"],
            "2026 Example Inc."
        );
    }

    #[test]
    fn copyright_changes_are_tracked_per_platform() {
        let previous = "2026 Example Inc.".to_string();

        assert!(has_copyright_changed("2026 Example Inc.", None));
        assert!(!has_copyright_changed("2026 Example Inc.", Some(&previous)));
        assert!(has_copyright_changed("2027 Example Inc.", Some(&previous)));
    }

    #[test]
    fn blank_draft_still_writes_version_identity() {
        let version_dir = std::env::temp_dir().join(format!(
            "fastforge-appstore-copyright-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&version_dir).unwrap();
        write_version_file(
            &version_dir,
            "version-id",
            "MAC_OS",
            "1.0.0",
            Some("PREPARE_FOR_SUBMISSION".into()),
            None,
            None,
        )
        .unwrap();

        let yaml = std::fs::read_to_string(version_dir.join("version.yaml")).unwrap();
        assert!(yaml.contains("_id: version-id"));
        assert!(yaml.contains("platform: MAC_OS"));
        assert!(yaml.contains("versionString: 1.0.0"));
        assert!(yaml.contains("state: PREPARE_FOR_SUBMISSION"));

        std::fs::remove_dir_all(&version_dir).unwrap();
    }

    #[test]
    fn macos_version_query_has_each_filter_once() {
        let query = version_query_params(Some("MAC_OS"), Some("1.0.0")).unwrap();
        let count = |name: &str| query.iter().filter(|(key, _)| *key == name).count();

        assert_eq!(count("fields[appStoreVersions]"), 1);
        assert_eq!(count("filter[platform]"), 1);
        assert_eq!(count("filter[versionString]"), 1);
        assert!(query.contains(&("filter[platform]", "MAC_OS".to_string())));
        assert!(query.contains(&("filter[versionString]", "1.0.0".to_string())));
    }

    #[test]
    fn staged_catalog_replaces_complete_snapshot() {
        let root =
            std::env::temp_dir().join(format!("fastforge-appstore-stage-{}", uuid::Uuid::new_v4()));
        let target = root.join("com.example.app");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("old.yaml"), "old").unwrap();

        let staged = StagedCatalog::new(&target).unwrap();
        std::fs::write(staged.path().join("app.yaml"), "new").unwrap();
        staged.commit().unwrap();

        assert!(!target.join("old.yaml").exists());
        assert_eq!(
            std::fs::read_to_string(target.join("app.yaml")).unwrap(),
            "new"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dropped_staging_preserves_previous_snapshot() {
        let root =
            std::env::temp_dir().join(format!("fastforge-appstore-stage-{}", uuid::Uuid::new_v4()));
        let target = root.join("com.example.app");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("app.yaml"), "complete").unwrap();

        let staging_path = {
            let staged = StagedCatalog::new(&target).unwrap();
            std::fs::write(staged.path().join("app.yaml"), "incomplete").unwrap();
            staged.path().to_path_buf()
        };

        assert!(!staging_path.exists());
        assert_eq!(
            std::fs::read_to_string(target.join("app.yaml")).unwrap(),
            "complete"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn absent_review_detail_is_an_empty_optional_resource() {
        assert!(review_detail_data(&serde_json::json!({ "data": null })).is_none());
    }
}
