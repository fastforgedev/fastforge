use crate::cli::GlobalArgs;
use crate::{AppGalleryContext, print_json, print_table};
use anyhow::{Result, anyhow};
use clap::{Args, Subcommand};

#[derive(Args, Debug)]
pub struct AppArgs {
    #[command(subcommand)]
    pub command: AppCommand,
}

#[derive(Subcommand, Debug)]
pub enum AppCommand {
    #[command(about = "Resolve AppGallery app IDs from package names")]
    Resolve(ResolveArgs),
    #[command(about = "View AppGallery app information")]
    View(ViewArgs),
}

#[derive(Args, Debug)]
pub struct ResolveArgs {
    #[arg(value_name = "PACKAGE_NAME", num_args = 1..)]
    pub package_name: Vec<String>,
    #[arg(long, value_name = "TYPES")]
    pub package_types: Option<String>,
}

#[derive(Args, Debug)]
pub struct ViewArgs {
    #[arg(value_name = "APP_ID")]
    pub app_id: String,
    #[arg(long)]
    pub lang: Option<String>,
    #[arg(long, default_value_t = 1)]
    pub release_type: i32,
}

pub async fn execute(args: &AppArgs, global: &GlobalArgs) -> Result<()> {
    let context = AppGalleryContext::from_env().await?;
    match &args.command {
        AppCommand::Resolve(args) => resolve(args, global, &context).await,
        AppCommand::View(args) => view(args, global, &context).await,
    }
}

async fn resolve(
    args: &ResolveArgs,
    global: &GlobalArgs,
    context: &AppGalleryContext,
) -> Result<()> {
    if args.package_name.is_empty() {
        return Err(anyhow!("at least one PACKAGE_NAME is required"));
    }
    if args.package_name.len() > 50 {
        return Err(anyhow!("AppGallery accepts at most 50 package names"));
    }
    let package_names = args.package_name.join(",");
    let response = context
        .client
        .get_app_ids(&package_names, args.package_types.as_deref(), None, None)
        .await?
        .into_inner();
    ensure_success(response.ret.code, &response.ret.msg)?;

    if global.json.is_some() {
        return print_json(&response.appids, global.json.as_deref());
    }
    let rows = response
        .appids
        .into_iter()
        .map(|app| {
            vec![
                app.key.unwrap_or_else(|| "-".to_string()),
                app.value.unwrap_or_else(|| "-".to_string()),
            ]
        })
        .collect::<Vec<_>>();
    print_table(&["APP", "APP_ID"], &rows);
    Ok(())
}

async fn view(args: &ViewArgs, global: &GlobalArgs, context: &AppGalleryContext) -> Result<()> {
    let response = context
        .client
        .get_app_info(
            &args.app_id,
            args.lang.as_deref(),
            Some(args.release_type),
            None,
        )
        .await?
        .into_inner();
    ensure_success(response.ret.code, &response.ret.msg)?;

    if global.json.is_some() {
        return print_json(&response, global.json.as_deref());
    }
    let app = response
        .app_info
        .ok_or_else(|| anyhow!("AppGallery response did not include appInfo"))?;
    let name = response
        .languages
        .iter()
        .find_map(|language| language.app_name.as_deref())
        .unwrap_or("-");
    print_table(
        &["APP_ID", "NAME", "VERSION", "VERSION_CODE", "STATE"],
        &[vec![
            args.app_id.clone(),
            name.to_string(),
            app.version_number.unwrap_or_else(|| "-".to_string()),
            app.version_code
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            app.release_state
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
        ]],
    );
    Ok(())
}

pub(crate) fn ensure_success(code: i64, message: &str) -> Result<()> {
    if code == 0 {
        Ok(())
    } else {
        Err(anyhow!("AppGallery API error {code}: {message}"))
    }
}
