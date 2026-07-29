use crate::AppStoreConnectContext;
use crate::Error as AscError;
use crate::cli::GlobalArgs;
use crate::cli::commands::app::resolve_app;
use crate::types::{
    self as asc_types, AppInfoLocalizationAttributes, AppStoreVersionAttributes,
    AppStoreVersionLocalizationAttributes, AppStoreVersionLocalizationUpdateRequest,
    AppStoreVersionLocalizationUpdateRequestData,
    AppStoreVersionLocalizationUpdateRequestDataAttributes,
    AppStoreVersionLocalizationUpdateRequestDataType, Platform,
};
use anyhow::{Context, Result, anyhow};
use clap::Args;
use serde_json::{Value, json};
use std::path::Path;

use super::{APP_INFO_CATEGORY_FIELDS, AppInfoCategories, VersionMetadata, screenshots};

#[derive(Args, Debug)]
pub struct PushArgs {
    #[arg(long = "app")]
    pub app: String,
    #[arg(long = "input")]
    pub input: Option<String>,
    #[arg(long = "dry-run", default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug)]
#[allow(dead_code)]
struct PushAction {
    resource_type: String,
    resource_id: Option<String>,
    locale: Option<String>,
    action: &'static str,
    details: String,
}

pub async fn execute(args: &PushArgs, _global: &GlobalArgs) -> Result<()> {
    let ctx = AppStoreConnectContext::from_env()?;
    execute_with_context(args, &ctx).await
}

/// Push catalog data using an existing App Store Connect context.
pub async fn execute_with_context(args: &PushArgs, ctx: &AppStoreConnectContext) -> Result<()> {
    let mut actions: Vec<PushAction> = Vec::new();

    eprintln!("🔍 Resolving app '{}'...", args.app);
    let app_row = resolve_app(ctx, &args.app).await?;
    let bundle_id = &app_row.bundle_id;
    let base_dir = args
        .input
        .as_deref()
        .map(Path::new)
        .unwrap_or_else(|| Path::new(".fastforge/stores/appstore"))
        .join(bundle_id);
    eprintln!("📂 Reading sync directory: {}", base_dir.display());

    // Scan phase
    let app_info_path = base_dir.join("app_info.yaml");
    if app_info_path.exists() {
        let categories: AppInfoCategories = read_yaml(&app_info_path)?;
        if !categories.is_empty() {
            actions.push(PushAction {
                resource_type: "appInfos".into(),
                resource_id: None,
                locale: None,
                action: "update",
                details: "app_info.yaml".into(),
            });
        }
    }

    let loc_dir = base_dir.join("info");
    if loc_dir.exists() {
        for entry in std::fs::read_dir(&loc_dir).context("reading info localizations")? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("yaml") {
                let loc: AppInfoLocalizationAttributes = read_yaml(&path)?;
                let locale = loc.locale.clone().unwrap_or_default();
                actions.push(PushAction {
                    resource_type: "appInfoLocalizations".into(),
                    resource_id: None,
                    locale: Some(locale.clone()),
                    action: "create",
                    details: format!("info/{locale}.yaml"),
                });
            }
        }
    }

    let versions_dir = base_dir.join("versions");
    for version_path in version_dirs(&versions_dir)? {
        let (platform_dir, version_dir) = version_path_segments(&version_path);
        let version_yaml_path = version_path.join("version.yaml");
        if version_yaml_path.exists() {
            let metadata: VersionMetadata = read_yaml(&version_yaml_path)?;
            if metadata.copyright.is_some() {
                actions.push(PushAction {
                    resource_type: "appStoreVersions".into(),
                    resource_id: None,
                    locale: None,
                    action: "update",
                    details: format!("versions/{platform_dir}/{version_dir}/version.yaml"),
                });
            }
        }

        for vloc_path in version_localization_dirs(&version_path)? {
            let locale = vloc_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            if let Some(vloc_yaml_path) = version_localization_file(&vloc_path)? {
                let _v: AppStoreVersionLocalizationAttributes = read_yaml(&vloc_yaml_path)?;
                let file_name = vloc_yaml_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("localization.yaml");
                actions.push(PushAction {
                    resource_type: "appStoreVersionLocalizations".into(),
                    resource_id: None,
                    locale: Some(locale.clone()),
                    action: "create",
                    details: format!("versions/{platform_dir}/{version_dir}/{locale}/{file_name}"),
                });
            }

            for (display_type, file_count) in screenshots::discover_screenshot_sets(&vloc_path)? {
                actions.push(PushAction {
                    resource_type: "appScreenshots".into(),
                    resource_id: None,
                    locale: Some(locale.clone()),
                    action: "sync",
                    details: format!(
                        "versions/{platform_dir}/{version_dir}/{locale}/screenshots/{display_type}/ ({file_count} files)"
                    ),
                });
            }
        }
    }

    if args.dry_run {
        eprintln!("\n📋 Dry run — would push {} actions:", actions.len());
        for a in &actions {
            eprintln!(
                "  [{:>8}] {} {} {}",
                a.action,
                a.resource_type,
                a.locale.as_deref().unwrap_or(""),
                a.details
            );
        }
        return Ok(());
    }

    eprintln!("\n🚀 Executing push...");
    let app_id = app_row.id.clone();

    // Execute: app info localizations
    let current_app_info = fetch_current_app_info(ctx, &app_id).await?;
    if app_info_path.exists() {
        push_app_info_categories(ctx, &current_app_info, &app_info_path).await?;
    }
    let existing_locales = fetch_current_app_info_localizations(ctx, &current_app_info.id).await?;
    if loc_dir.exists() {
        for entry in std::fs::read_dir(&loc_dir).context("reading info localizations")? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("yaml") {
                let loc: AppInfoLocalizationAttributes = read_yaml(&path)?;
                let locale = loc.locale.clone().unwrap_or_default();
                if let Some(existing) = existing_locales.iter().find(|e| e.locale == locale) {
                    let updated = push_update_app_info_localization(
                        ctx,
                        &existing.id,
                        &loc,
                        &existing.attributes,
                    )
                    .await?;
                    if updated {
                        eprintln!("  ✓ info/{locale}.yaml (id: {})", existing.id);
                    } else {
                        eprintln!("  - info/{locale}.yaml (unchanged, id: {})", existing.id);
                    }
                } else {
                    push_create_app_info_localization(ctx, &current_app_info.id, &loc).await?;
                    eprintln!("  ✓ info/{locale}.yaml (new)");
                }
            }
        }
    }

    // Execute: versions
    for version_path in version_dirs(&versions_dir)? {
        let (platform_dir, version_dir) = version_path_segments(&version_path);
        let (platform, version_str) =
            resolve_version_identity(&version_path, &platform_dir, &version_dir)?;
        let existing_versions =
            fetch_all_versions_simple(ctx, &app_id, &platform, &version_str).await?;
        let existing_version = if let Some(v) = existing_versions.first() {
            v
        } else {
            eprintln!("  ⚠ Skipping versions/{platform_dir}/{version_dir}: version not found");
            continue;
        };
        let version_id = existing_version.id.clone();

        let version_yaml_path = version_path.join("version.yaml");
        if version_yaml_path.exists() {
            push_version_metadata(
                ctx,
                &version_id,
                existing_version.copyright.as_deref(),
                &version_yaml_path,
            )
            .await?;
        }

        let existing_vlocs = fetch_version_localizations_simple(ctx, &version_id).await?;
        for vloc_path in version_localization_dirs(&version_path)? {
            let locale = vloc_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            let Some(vloc_yaml_path) = version_localization_file(&vloc_path)? else {
                continue;
            };

            let mut vloc_yaml: AppStoreVersionLocalizationAttributes = read_yaml(&vloc_yaml_path)?;
            vloc_yaml.locale = Some(locale.clone());
            if let Some(existing) = existing_vlocs.iter().find(|e| e.locale == locale) {
                let updated = push_update_version_localization(
                    ctx,
                    &existing.id,
                    &vloc_yaml,
                    &existing.attributes,
                )
                .await?;
                if updated {
                    eprintln!(
                        "  ✓ versions/{platform_dir}/{version_dir}/{locale} (id: {})",
                        existing.id
                    );
                } else {
                    eprintln!(
                        "  - versions/{platform_dir}/{version_dir}/{locale} (unchanged, id: {})",
                        existing.id
                    );
                }
            } else {
                push_create_version_localization(ctx, &version_id, &vloc_yaml).await?;
                eprintln!("  ✓ versions/{platform_dir}/{version_dir}/{locale} (new)");
            }
        }

        // Execute: screenshots. Refresh localizations because the metadata pass above may
        // have created a locale that did not exist when the version was first queried.
        let current_vlocs = fetch_version_localizations_simple(ctx, &version_id).await?;
        for vloc_path in version_localization_dirs(&version_path)? {
            if screenshots::discover_screenshot_sets(&vloc_path)?.is_empty() {
                continue;
            }
            let locale = vloc_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            let localization = current_vlocs
                .iter()
                .find(|localization| localization.locale == locale)
                .ok_or_else(|| {
                    anyhow!(
                        "cannot push screenshots for locale {locale}: App Store version localization not found"
                    )
                })?;
            let summary = screenshots::sync_localization_screenshots(
                ctx,
                &base_dir,
                &localization.id,
                &vloc_path,
            )
            .await?;
            eprintln!(
                "  ✓ screenshots/{locale}: {} reused, {} uploaded, {} deleted",
                summary.reused, summary.uploaded, summary.deleted
            );
        }

        // Execute: review detail
        let review_yaml_path = version_path.join("review.yaml");
        if review_yaml_path.exists() {
            push_review_detail(ctx, &version_id, &review_yaml_path).await?;

            // Execute: review attachments
            let review_dir = version_path.join("review.d");
            if review_dir.exists() {
                let review_detail_id = resolve_review_detail_id(ctx, &version_id).await?;
                for entry in std::fs::read_dir(&review_dir).context("reading review.d")? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("yaml") {
                        push_review_attachment(ctx, &review_detail_id, &path).await?;
                    }
                }
            }
        }
    }

    eprintln!("\n✅ Push complete.");
    Ok(())
}

fn read_yaml<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    serde_yaml::from_str(&content).with_context(|| format!("failed to parse {}", path.display()))
}

async fn push_version_metadata(
    ctx: &AppStoreConnectContext,
    version_id: &str,
    remote_copyright: Option<&str>,
    yaml_path: &Path,
) -> Result<()> {
    let metadata: VersionMetadata = read_yaml(yaml_path)?;
    let Some(copyright) = metadata.copyright else {
        return Ok(());
    };
    if !copyright_needs_update(&copyright, remote_copyright) {
        eprintln!("  - version.yaml (unchanged, id: {version_id})");
        return Ok(());
    }

    ctx.http
        .patch(ctx.url(&format!("/v1/appStoreVersions/{version_id}")))
        .json(&version_update_body(version_id, copyright))
        .send()
        .await?
        .error_for_status()
        .with_context(|| format!("failed to update copyright for version {version_id}"))?;
    eprintln!("  ✓ version.yaml (id: {version_id})");
    Ok(())
}

fn copyright_needs_update(local: &str, remote: Option<&str>) -> bool {
    remote != Some(local)
}

fn version_update_body(version_id: &str, copyright: String) -> Value {
    json!({
        "data": {
            "type": "appStoreVersions",
            "id": version_id,
            "attributes": { "copyright": copyright }
        }
    })
}

fn version_dirs(versions_dir: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut dirs = Vec::new();
    if !versions_dir.exists() {
        return Ok(dirs);
    }

    for platform_entry in std::fs::read_dir(versions_dir).context("reading versions")? {
        let platform_entry = platform_entry?;
        let platform_path = platform_entry.path();
        if !platform_path.is_dir() {
            continue;
        }

        for version_entry in
            std::fs::read_dir(&platform_path).context("reading platform versions")?
        {
            let version_entry = version_entry?;
            let version_path = version_entry.path();
            if version_path.is_dir() {
                dirs.push(version_path);
            }
        }
    }

    Ok(dirs)
}

fn version_path_segments(version_path: &Path) -> (String, String) {
    let version = version_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let platform = version_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    (platform, version)
}

fn resolve_version_identity(
    version_path: &Path,
    platform_dir: &str,
    version_dir: &str,
) -> Result<(String, String)> {
    let version_yaml_path = version_path.join("version.yaml");
    if !version_yaml_path.exists() {
        return Ok((platform_dir.to_string(), version_dir.to_string()));
    }

    let version_yaml: AppStoreVersionAttributes = read_yaml(&version_yaml_path)?;
    let platform = version_yaml
        .platform
        .as_ref()
        .map(|p: &Platform| p.to_string())
        .unwrap_or_else(|| platform_dir.to_string());
    let version = version_yaml
        .version_string
        .as_deref()
        .unwrap_or(version_dir)
        .to_string();
    Ok((platform, version))
}

fn version_localization_dirs(version_path: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut dirs = Vec::new();

    for entry in std::fs::read_dir(version_path).context("reading version localizations")? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name == "screenshots" || name == "previews" {
            continue;
        }

        dirs.push(path);
    }

    Ok(dirs)
}

fn version_localization_file(localization_dir: &Path) -> Result<Option<std::path::PathBuf>> {
    let localization_yaml = localization_dir.join("localization.yaml");
    let legacy_version_yaml = localization_dir.join("version.yaml");

    if localization_yaml.exists() && legacy_version_yaml.exists() {
        return Err(anyhow!(
            "both {} and {} exist; keep only localization.yaml",
            localization_yaml.display(),
            legacy_version_yaml.display()
        ));
    }

    if localization_yaml.exists() {
        return Ok(Some(localization_yaml));
    }

    if legacy_version_yaml.exists() {
        return Ok(Some(legacy_version_yaml));
    }

    Ok(None)
}

struct CurrentAppInfo {
    id: String,
    categories: AppInfoCategories,
}

async fn fetch_current_app_info(
    ctx: &AppStoreConnectContext,
    app_id: &str,
) -> Result<CurrentAppInfo> {
    let resp: Value = ctx
        .http
        .get(ctx.url(&format!("/v1/apps/{app_id}/appInfos")))
        .query(&[
            ("fields[appInfos]", APP_INFO_CATEGORY_FIELDS),
            ("include", APP_INFO_CATEGORY_FIELDS),
            ("limit", "1"),
        ])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let info_id = resp["data"][0]["id"]
        .as_str()
        .ok_or_else(|| anyhow!("no appInfo found for app {app_id}"))?
        .to_string();
    let categories = AppInfoCategories::from_relationships(&resp["data"][0]["relationships"]);
    Ok(CurrentAppInfo {
        id: info_id,
        categories,
    })
}

async fn push_app_info_categories(
    ctx: &AppStoreConnectContext,
    current: &CurrentAppInfo,
    yaml_path: &Path,
) -> Result<()> {
    let categories: AppInfoCategories = read_yaml(yaml_path)?;
    if categories.is_empty() {
        return Err(anyhow!("app_info.yaml does not contain any category IDs"));
    }
    if !app_info_categories_have_changes(&categories, &current.categories) {
        eprintln!("  - app_info.yaml (unchanged, id: {})", current.id);
        return Ok(());
    }

    let body = app_info_category_update_body(&current.id, &categories);
    ctx.http
        .patch(ctx.url(&format!("/v1/appInfos/{}", current.id)))
        .json(&body)
        .send()
        .await?
        .error_for_status()
        .with_context(|| format!("failed to update categories for app info {}", current.id))?;
    eprintln!("  ✓ app_info.yaml (id: {})", current.id);
    Ok(())
}

fn app_info_categories_have_changes(
    desired: &AppInfoCategories,
    current: &AppInfoCategories,
) -> bool {
    category_changed(&desired.primary_category, &current.primary_category)
        || category_changed(
            &desired.primary_subcategory_one,
            &current.primary_subcategory_one,
        )
        || category_changed(
            &desired.primary_subcategory_two,
            &current.primary_subcategory_two,
        )
        || category_changed(&desired.secondary_category, &current.secondary_category)
        || category_changed(
            &desired.secondary_subcategory_one,
            &current.secondary_subcategory_one,
        )
        || category_changed(
            &desired.secondary_subcategory_two,
            &current.secondary_subcategory_two,
        )
}

fn category_changed(desired: &Option<String>, current: &Option<String>) -> bool {
    desired
        .as_ref()
        .is_some_and(|desired| current.as_ref() != Some(desired))
}

fn app_info_category_update_body(app_info_id: &str, categories: &AppInfoCategories) -> Value {
    let mut relationships = serde_json::Map::new();
    insert_category_relationship(
        &mut relationships,
        "primaryCategory",
        categories.primary_category.as_deref(),
    );
    insert_category_relationship(
        &mut relationships,
        "primarySubcategoryOne",
        categories.primary_subcategory_one.as_deref(),
    );
    insert_category_relationship(
        &mut relationships,
        "primarySubcategoryTwo",
        categories.primary_subcategory_two.as_deref(),
    );
    insert_category_relationship(
        &mut relationships,
        "secondaryCategory",
        categories.secondary_category.as_deref(),
    );
    insert_category_relationship(
        &mut relationships,
        "secondarySubcategoryOne",
        categories.secondary_subcategory_one.as_deref(),
    );
    insert_category_relationship(
        &mut relationships,
        "secondarySubcategoryTwo",
        categories.secondary_subcategory_two.as_deref(),
    );

    json!({
        "data": {
            "type": "appInfos",
            "id": app_info_id,
            "relationships": relationships,
        }
    })
}

fn insert_category_relationship(
    relationships: &mut serde_json::Map<String, Value>,
    name: &str,
    category_id: Option<&str>,
) {
    if let Some(category_id) = category_id {
        relationships.insert(
            name.to_string(),
            json!({ "data": { "type": "appCategories", "id": category_id } }),
        );
    }
}

struct CurrentAppInfoLocalization {
    id: String,
    locale: String,
    attributes: serde_json::Value,
}

async fn fetch_current_app_info_localizations(
    ctx: &AppStoreConnectContext,
    app_info_id: &str,
) -> Result<Vec<CurrentAppInfoLocalization>> {
    let resp: Value = ctx
        .http
        .get(ctx.url(&format!("/v1/appInfos/{app_info_id}/appInfoLocalizations")))
        .query(&[
            ("limit", "200"),
            (
                "fields[appInfoLocalizations]",
                "locale,name,subtitle,privacyPolicyUrl,privacyPolicyText,privacyChoicesUrl",
            ),
        ])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(resp["data"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| {
            Some(CurrentAppInfoLocalization {
                id: item["id"].as_str()?.to_string(),
                locale: item["attributes"]["locale"].as_str()?.to_string(),
                attributes: item["attributes"].clone(),
            })
        })
        .collect())
}

async fn push_update_app_info_localization(
    ctx: &AppStoreConnectContext,
    id: &str,
    yaml: &AppInfoLocalizationAttributes,
    remote_attrs: &serde_json::Value,
) -> Result<bool> {
    let attrs = asc_types::AppInfoLocalizationUpdateRequestDataAttributes {
        name: yaml.name.clone(),
        subtitle: yaml.subtitle.clone(),
        privacy_policy_url: yaml.privacy_policy_url.clone(),
        privacy_policy_text: yaml.privacy_policy_text.clone(),
        privacy_choices_url: yaml.privacy_choices_url.clone(),
    };

    // Skip update if nothing changed
    if attributes_match_remote_localization(&attrs, remote_attrs) {
        return Ok(false);
    }

    let body = asc_types::AppInfoLocalizationUpdateRequest {
        data: asc_types::AppInfoLocalizationUpdateRequestData {
            attributes: Some(attrs),
            id: id.to_string(),
            type_: asc_types::AppInfoLocalizationUpdateRequestDataType::AppInfoLocalizations,
        },
    };

    match ctx
        .client
        .app_info_localizations_update_instance(id, &body)
        .await
    {
        Ok(_) => Ok(true),
        Err(AscError::ErrorResponse(ref resp)) if resp.status().as_u16() == 409 => {
            eprintln!("  ⚠ info/... (id: {id}) — 409 Conflict, resource may be locked; skipping");
            Ok(false)
        }
        Err(e) => Err(anyhow!("failed to update app info localization {id}: {e}")),
    }
}

/// Check if the local attributes match the remote attributes for all
/// editable fields.
fn attributes_match_remote_localization(
    local: &asc_types::AppInfoLocalizationUpdateRequestDataAttributes,
    remote: &serde_json::Value,
) -> bool {
    let fields: Vec<(&str, Option<&str>)> = vec![
        ("name", local.name.as_deref()),
        ("subtitle", local.subtitle.as_deref()),
        ("privacyPolicyUrl", local.privacy_policy_url.as_deref()),
        ("privacyPolicyText", local.privacy_policy_text.as_deref()),
        ("privacyChoicesUrl", local.privacy_choices_url.as_deref()),
    ];
    for (key, local_val) in fields {
        let remote_val = remote.get(key).and_then(|v| v.as_str());
        match (local_val, remote_val) {
            (None, None) => continue,
            (Some(l), Some(r)) if l == r => continue,
            _ => return false,
        }
    }
    true
}

async fn push_create_app_info_localization(
    ctx: &AppStoreConnectContext,
    app_info_id: &str,
    yaml: &AppInfoLocalizationAttributes,
) -> Result<()> {
    let locale = yaml.locale.clone().unwrap_or_default();
    let mut attrs = json!({"locale": locale});
    if let Some(ref v) = yaml.name {
        attrs["name"] = json!(v);
    }
    if let Some(ref v) = yaml.subtitle {
        attrs["subtitle"] = json!(v);
    }
    if let Some(ref v) = yaml.privacy_policy_url {
        attrs["privacyPolicyUrl"] = json!(v);
    }
    if let Some(ref v) = yaml.privacy_policy_text {
        attrs["privacyPolicyText"] = json!(v);
    }
    if let Some(ref v) = yaml.privacy_choices_url {
        attrs["privacyChoicesUrl"] = json!(v);
    }
    ctx.http.post(ctx.url("/v1/appInfoLocalizations"))
        .json(&json!({"data": {"type": "appInfoLocalizations", "attributes": attrs, "relationships": {"appInfo": {"data": {"type": "appInfos", "id": app_info_id}}}}}))
        .send().await?.error_for_status()?;
    Ok(())
}

async fn push_update_version_localization(
    ctx: &AppStoreConnectContext,
    id: &str,
    yaml: &AppStoreVersionLocalizationAttributes,
    remote: &AppStoreVersionLocalizationAttributes,
) -> Result<bool> {
    let attrs = changed_version_localization_attributes(yaml, remote);
    if !has_version_localization_update_attributes(&attrs) {
        return Ok(false);
    }

    let body = AppStoreVersionLocalizationUpdateRequest {
        data: AppStoreVersionLocalizationUpdateRequestData {
            attributes: Some(attrs),
            id: id.to_string(),
            type_: AppStoreVersionLocalizationUpdateRequestDataType::AppStoreVersionLocalizations,
        },
    };
    ctx.client
        .app_store_version_localizations_update_instance(id, &body)
        .await
        .map_err(|e| anyhow!("failed to update version localization {id}: {e}"))?;
    Ok(true)
}

fn changed_version_localization_attributes(
    desired: &AppStoreVersionLocalizationAttributes,
    remote: &AppStoreVersionLocalizationAttributes,
) -> AppStoreVersionLocalizationUpdateRequestDataAttributes {
    AppStoreVersionLocalizationUpdateRequestDataAttributes {
        description: changed_value(&desired.description, &remote.description),
        keywords: changed_value(&desired.keywords, &remote.keywords),
        marketing_url: changed_value(&desired.marketing_url, &remote.marketing_url),
        promotional_text: changed_value(&desired.promotional_text, &remote.promotional_text),
        support_url: changed_value(&desired.support_url, &remote.support_url),
        whats_new: changed_value(&desired.whats_new, &remote.whats_new),
    }
}

fn changed_value(desired: &Option<String>, remote: &Option<String>) -> Option<String> {
    desired
        .as_ref()
        .filter(|desired| remote.as_ref() != Some(*desired))
        .cloned()
}

fn has_version_localization_update_attributes(
    attrs: &AppStoreVersionLocalizationUpdateRequestDataAttributes,
) -> bool {
    attrs.description.is_some()
        || attrs.keywords.is_some()
        || attrs.marketing_url.is_some()
        || attrs.promotional_text.is_some()
        || attrs.support_url.is_some()
        || attrs.whats_new.is_some()
}

async fn push_create_version_localization(
    ctx: &AppStoreConnectContext,
    version_id: &str,
    yaml: &AppStoreVersionLocalizationAttributes,
) -> Result<()> {
    let locale = yaml.locale.clone().unwrap_or_default();
    let mut attrs = json!({"locale": locale});
    if let Some(ref v) = yaml.description {
        attrs["description"] = json!(v);
    }
    if let Some(ref v) = yaml.keywords {
        attrs["keywords"] = json!(v);
    }
    if let Some(ref v) = yaml.marketing_url {
        attrs["marketingUrl"] = json!(v);
    }
    if let Some(ref v) = yaml.promotional_text {
        attrs["promotionalText"] = json!(v);
    }
    if let Some(ref v) = yaml.support_url {
        attrs["supportUrl"] = json!(v);
    }
    if let Some(ref v) = yaml.whats_new {
        attrs["whatsNew"] = json!(v);
    }
    ctx.http.post(ctx.url("/v1/appStoreVersionLocalizations"))
        .json(&json!({"data": {"type": "appStoreVersionLocalizations", "attributes": attrs, "relationships": {"appStoreVersion": {"data": {"type": "appStoreVersions", "id": version_id}}}}}))
        .send().await?.error_for_status()?;
    Ok(())
}

struct CurrentAppVersion {
    id: String,
    copyright: Option<String>,
}

async fn fetch_all_versions_simple(
    ctx: &AppStoreConnectContext,
    app_id: &str,
    platform: &str,
    version_str: &str,
) -> Result<Vec<CurrentAppVersion>> {
    let filter_platform = Some(vec![match platform {
        "MAC_OS" => asc_types::AppsAppStoreVersionsGetToManyRelatedFilterPlatformItem::MacOs,
        "TV_OS" => asc_types::AppsAppStoreVersionsGetToManyRelatedFilterPlatformItem::TvOs,
        "VISION_OS" => asc_types::AppsAppStoreVersionsGetToManyRelatedFilterPlatformItem::VisionOs,
        _ => asc_types::AppsAppStoreVersionsGetToManyRelatedFilterPlatformItem::Ios,
    }]);
    let filter_version = Some(vec![version_str.to_string()]);
    let resp = ctx
        .client
        .apps_app_store_versions_get_to_many_related(
            app_id,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            filter_platform.as_ref(),
            filter_version.as_ref(),
            None,
            Some(1),
            None,
            None,
            None,
        )
        .await
        .map_err(|e| anyhow!("failed to fetch versions: {e}"))?;
    Ok(resp
        .into_inner()
        .data
        .into_iter()
        .map(|v| CurrentAppVersion {
            id: v.id,
            copyright: v.attributes.and_then(|attrs| attrs.copyright),
        })
        .collect())
}
struct CurrentAppVersionLocalization {
    id: String,
    locale: String,
    attributes: AppStoreVersionLocalizationAttributes,
}

async fn fetch_version_localizations_simple(
    ctx: &AppStoreConnectContext,
    version_id: &str,
) -> Result<Vec<CurrentAppVersionLocalization>> {
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
        .map_err(|e| anyhow!("failed to fetch version localizations: {e}"))?;
    Ok(resp
        .into_inner()
        .data
        .into_iter()
        .filter_map(|v| {
            let attributes = v.attributes?;
            Some(CurrentAppVersionLocalization {
                id: v.id,
                locale: attributes.locale.clone()?,
                attributes,
            })
        })
        .collect())
}

// ── App Store Review Detail (审核信息) push ─────────────────────────

#[derive(serde::Deserialize)]
struct ReviewAttachmentYaml {
    _id: Option<String>,
    file_name: Option<String>,
    file_size: Option<i64>,
    source_file_checksum: Option<String>,
}

/// Resolve the review detail ID for a version, if one exists.
async fn resolve_review_detail_id(
    ctx: &AppStoreConnectContext,
    version_id: &str,
) -> Result<String> {
    let resp = ctx
        .client
        .app_store_versions_app_store_review_detail_get_to_one_related(
            version_id,
            None::<&Vec<asc_types::AppStoreVersionsAppStoreReviewDetailGetToOneRelatedFieldsAppStoreReviewAttachmentsItem>>,
            None::<&Vec<asc_types::AppStoreVersionsAppStoreReviewDetailGetToOneRelatedFieldsAppStoreReviewDetailsItem>>,
            None::<&Vec<asc_types::AppStoreVersionsAppStoreReviewDetailGetToOneRelatedFieldsAppStoreVersionsItem>>,
            None::<&Vec<asc_types::AppStoreVersionsAppStoreReviewDetailGetToOneRelatedIncludeItem>>,
            None::<i64>,
        )
        .await
        .map_err(|_| anyhow!("no review detail found for version {version_id}"))?;
    Ok(resp.into_inner().data.id)
}

/// Push review detail: update if exists (by resolving server-side), otherwise create.
async fn push_review_detail(
    ctx: &AppStoreConnectContext,
    version_id: &str,
    yaml_path: &Path,
) -> Result<()> {
    let yaml: asc_types::AppStoreReviewDetailAttributes = read_yaml(yaml_path)?;

    // Try to resolve an existing review detail from the server first
    let existing_id = ctx
        .client
        .app_store_versions_app_store_review_detail_get_to_one_related(
            version_id,
            None::<&Vec<asc_types::AppStoreVersionsAppStoreReviewDetailGetToOneRelatedFieldsAppStoreReviewAttachmentsItem>>,
            None::<&Vec<asc_types::AppStoreVersionsAppStoreReviewDetailGetToOneRelatedFieldsAppStoreReviewDetailsItem>>,
            None::<&Vec<asc_types::AppStoreVersionsAppStoreReviewDetailGetToOneRelatedFieldsAppStoreVersionsItem>>,
            None::<&Vec<asc_types::AppStoreVersionsAppStoreReviewDetailGetToOneRelatedIncludeItem>>,
            None::<i64>,
        )
        .await
        .ok()
        .map(|r| r.into_inner().data.id);

    if let Some(ref detail_id) = existing_id {
        // Fetch existing review detail to get current values
        let existing_resp = ctx
            .client
            .app_store_review_details_get_instance(
                detail_id,
                None::<&Vec<asc_types::AppStoreReviewDetailsGetInstanceFieldsAppStoreReviewAttachmentsItem>>,
                None::<&Vec<asc_types::AppStoreReviewDetailsGetInstanceFieldsAppStoreReviewDetailsItem>>,
                None::<&Vec<asc_types::AppStoreReviewDetailsGetInstanceFieldsAppStoreVersionsItem>>,
                None::<&Vec<asc_types::AppStoreReviewDetailsGetInstanceIncludeItem>>,
                None::<i64>,
            )
            .await
            .map_err(|e| anyhow!("failed to fetch existing review detail: {e}"))?;
        let existing_attrs = existing_resp.into_inner().data.attributes;

        // Extract existing values for fallback
        let (
            existing_email,
            existing_first_name,
            existing_last_name,
            existing_phone,
            existing_demo_name,
            existing_demo_pw,
            existing_demo_req,
            existing_notes,
        ) = existing_attrs
            .map(|a| {
                (
                    a.contact_email,
                    a.contact_first_name,
                    a.contact_last_name,
                    a.contact_phone,
                    a.demo_account_name,
                    a.demo_account_password,
                    a.demo_account_required,
                    a.notes,
                )
            })
            .unwrap_or_default();

        // Only update if YAML values differ from existing server values
        let has_changes = yaml.contact_email != existing_email
            || yaml.contact_first_name != existing_first_name
            || yaml.contact_last_name != existing_last_name
            || yaml.contact_phone != existing_phone
            || yaml.demo_account_name != existing_demo_name
            || yaml.demo_account_password != existing_demo_pw
            || yaml.demo_account_required != existing_demo_req
            || yaml.notes != existing_notes;

        if !has_changes {
            eprintln!("  - review.yaml (unchanged, id: {detail_id})");
            return Ok(());
        }

        let attrs = asc_types::AppStoreReviewDetailUpdateRequestDataAttributes {
            contact_first_name: yaml.contact_first_name.clone().or(existing_first_name),
            contact_last_name: yaml.contact_last_name.clone().or(existing_last_name),
            contact_phone: yaml.contact_phone.clone().or(existing_phone),
            contact_email: yaml.contact_email.clone().or(existing_email),
            demo_account_name: yaml.demo_account_name.clone().or(existing_demo_name),
            demo_account_password: yaml.demo_account_password.clone().or(existing_demo_pw),
            demo_account_required: yaml.demo_account_required.or(existing_demo_req),
            notes: yaml.notes.clone().or(existing_notes),
        };

        let body = asc_types::AppStoreReviewDetailUpdateRequest {
            data: asc_types::AppStoreReviewDetailUpdateRequestData {
                attributes: Some(attrs),
                id: detail_id.clone(),
                type_: asc_types::AppStoreReviewDetailUpdateRequestDataType::AppStoreReviewDetails,
            },
        };
        ctx.client
            .app_store_review_details_update_instance(detail_id, &body)
            .await
            .map_err(|e| anyhow!("failed to update review detail: {e}"))?;
        eprintln!("  ✓ review.yaml (updated, id: {detail_id})");
    } else {
        // Create new review detail
        let attrs = asc_types::AppStoreReviewDetailCreateRequestDataAttributes {
            contact_email: yaml.contact_email.clone(),
            contact_first_name: yaml.contact_first_name.clone(),
            contact_last_name: yaml.contact_last_name.clone(),
            contact_phone: yaml.contact_phone.clone(),
            demo_account_name: yaml.demo_account_name.clone(),
            demo_account_password: yaml.demo_account_password.clone(),
            demo_account_required: yaml.demo_account_required,
            notes: yaml.notes.clone(),
        };

        let body = asc_types::AppStoreReviewDetailCreateRequest {
            data: asc_types::AppStoreReviewDetailCreateRequestData {
                attributes: Some(attrs),
                relationships: asc_types::AppStoreReviewDetailCreateRequestDataRelationships {
                    app_store_version:
                        asc_types::AppStoreReviewDetailCreateRequestDataRelationshipsAppStoreVersion {
                            data:
                                asc_types::AppStoreReviewDetailCreateRequestDataRelationshipsAppStoreVersionData {
                                    id: version_id.to_string(),
                                    type_: asc_types::AppStoreReviewDetailCreateRequestDataRelationshipsAppStoreVersionDataType::AppStoreVersions,
                                },
                        },
                },
                type_: asc_types::AppStoreReviewDetailCreateRequestDataType::AppStoreReviewDetails,
            },
        };
        ctx.client
            .app_store_review_details_create_instance(&body)
            .await
            .map_err(|e| anyhow!("failed to create review detail: {e}"))?;
        eprintln!("  ✓ review.yaml (created)");
    }

    Ok(())
}

/// Push a review attachment: create or update based on _id presence.
async fn push_review_attachment(
    ctx: &AppStoreConnectContext,
    _review_detail_id: &str,
    yaml_path: &Path,
) -> Result<()> {
    let yaml: ReviewAttachmentYaml = read_yaml(yaml_path)?;
    let file_name = yaml
        .file_name
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    if let Some(attachment_id) = &yaml._id {
        // Update via PATCH
        let mut attrs = asc_types::AppStoreReviewAttachmentUpdateRequestDataAttributes::default();
        if let Some(checksum) = &yaml.source_file_checksum {
            attrs.source_file_checksum = Some(checksum.clone());
        }
        if yaml.file_size.is_some() {
            attrs.uploaded = Some(true);
        }

        let body = asc_types::AppStoreReviewAttachmentUpdateRequest {
            data: asc_types::AppStoreReviewAttachmentUpdateRequestData {
                attributes: Some(attrs),
                id: attachment_id.clone(),
                type_: asc_types::AppStoreReviewAttachmentUpdateRequestDataType::AppStoreReviewAttachments,
            },
        };
        ctx.client
            .app_store_review_attachments_update_instance(attachment_id, &body)
            .await
            .map_err(|e| anyhow!("failed to update attachment {attachment_id}: {e}"))?;
        eprintln!("  ✓ review.d/{attachment_id}.yaml (updated)");
    } else {
        // Create via POST
        // New attachments require actual file upload operations which is complex;
        // For now, just log a warning that creating new attachments needs manual handling
        eprintln!(
            "  ⚠ review.d/{file_name}.yaml: new attachment creation requires upload operations"
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_update_only_contains_editable_copyright() {
        assert_eq!(
            version_update_body("version-id", "2026 Example Inc.".to_string()),
            json!({
                "data": {
                    "type": "appStoreVersions",
                    "id": "version-id",
                    "attributes": { "copyright": "2026 Example Inc." }
                }
            })
        );
    }

    #[test]
    fn version_metadata_ignores_identity_fields() {
        let metadata: VersionMetadata = serde_yaml::from_str(
            "platform: IOS\nversionString: 1.2.3\ncopyright: 2026 Example Inc.\n",
        )
        .unwrap();

        assert_eq!(metadata.copyright.as_deref(), Some("2026 Example Inc."));
    }

    #[test]
    fn copyright_is_only_updated_when_remote_differs() {
        assert!(!copyright_needs_update(
            "2026 Example Inc.",
            Some("2026 Example Inc.")
        ));
        assert!(copyright_needs_update(
            "2026 Example Inc.",
            Some("2025 Example Inc.")
        ));
        assert!(copyright_needs_update("2026 Example Inc.", None));
    }

    #[test]
    fn app_info_category_update_uses_relationship_ids() {
        let categories = AppInfoCategories {
            primary_category: Some("GAMES".into()),
            primary_subcategory_one: Some("GAMES_ACTION".into()),
            secondary_category: Some("ENTERTAINMENT".into()),
            ..Default::default()
        };

        assert_eq!(
            app_info_category_update_body("app-info-id", &categories),
            json!({
                "data": {
                    "type": "appInfos",
                    "id": "app-info-id",
                    "relationships": {
                        "primaryCategory": {
                            "data": { "type": "appCategories", "id": "GAMES" }
                        },
                        "primarySubcategoryOne": {
                            "data": { "type": "appCategories", "id": "GAMES_ACTION" }
                        },
                        "secondaryCategory": {
                            "data": { "type": "appCategories", "id": "ENTERTAINMENT" }
                        }
                    }
                }
            })
        );
    }

    #[test]
    fn category_changes_only_consider_configured_fields() {
        let current = AppInfoCategories {
            primary_category: Some("GAMES".into()),
            secondary_category: Some("BUSINESS".into()),
            ..Default::default()
        };
        let unchanged_partial = AppInfoCategories {
            primary_category: Some("GAMES".into()),
            ..Default::default()
        };
        let changed_partial = AppInfoCategories {
            primary_category: Some("UTILITIES".into()),
            ..Default::default()
        };

        assert!(!app_info_categories_have_changes(
            &unchanged_partial,
            &current
        ));
        assert!(app_info_categories_have_changes(&changed_partial, &current));
    }

    #[test]
    fn rejects_ambiguous_localization_files() {
        let dir = std::env::temp_dir().join(format!(
            "fastforge-appstore-localization-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("localization.yaml"), "description: current\n").unwrap();
        std::fs::write(dir.join("version.yaml"), "description: legacy\n").unwrap();

        let error = version_localization_file(&dir).unwrap_err();
        std::fs::remove_dir_all(&dir).unwrap();

        assert!(error.to_string().contains("keep only localization.yaml"));
    }

    #[test]
    fn version_localization_update_only_contains_changed_configured_fields() {
        let desired = AppStoreVersionLocalizationAttributes {
            description: Some("Same description".into()),
            keywords: Some("new,keywords".into()),
            support_url: Some("https://example.com/support".into()),
            ..Default::default()
        };
        let remote = AppStoreVersionLocalizationAttributes {
            description: Some("Same description".into()),
            keywords: Some("old,keywords".into()),
            marketing_url: Some("https://example.com/marketing".into()),
            support_url: Some("https://example.com/support".into()),
            ..Default::default()
        };

        let attrs = changed_version_localization_attributes(&desired, &remote);

        assert_eq!(attrs.description, None);
        assert_eq!(attrs.keywords.as_deref(), Some("new,keywords"));
        assert_eq!(attrs.marketing_url, None);
        assert_eq!(attrs.support_url, None);
        assert!(has_version_localization_update_attributes(&attrs));
    }

    #[test]
    fn unchanged_version_localization_does_not_need_an_update() {
        let desired = AppStoreVersionLocalizationAttributes {
            description: Some("Same description".into()),
            promotional_text: Some("Same promotion".into()),
            ..Default::default()
        };
        let remote = desired.clone();

        let attrs = changed_version_localization_attributes(&desired, &remote);

        assert!(!has_version_localization_update_attributes(&attrs));
    }
}
