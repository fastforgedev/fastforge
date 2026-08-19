use crate::cli::GlobalArgs;
use crate::cli::commands::app::ensure_success;
use crate::{AppGalleryContext, print_json, print_table};
use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Args, Debug)]
pub struct PackageArgs {
    #[command(subcommand)]
    pub command: PackageCommand,
}

#[derive(Subcommand, Debug)]
pub enum PackageCommand {
    #[command(about = "List packages associated with an AppGallery app")]
    List(PackageListArgs),
    #[command(about = "Query an AAB package compilation status")]
    Status(PackageStatusArgs),
}

#[derive(Args, Debug)]
pub struct PackageListArgs {
    #[arg(value_name = "APP_ID")]
    pub app_id: String,
    #[arg(long, default_value_t = 0)]
    pub offset: i32,
    #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(i32).range(1..=100))]
    pub limit: i32,
}

#[derive(Args, Debug)]
pub struct PackageStatusArgs {
    #[arg(value_name = "APP_ID")]
    pub app_id: String,
    #[arg(value_name = "PACKAGE_ID", num_args = 1..)]
    pub package_id: Vec<String>,
}

pub async fn execute(args: &PackageArgs, global: &GlobalArgs) -> Result<()> {
    let context = AppGalleryContext::from_env().await?;
    match &args.command {
        PackageCommand::List(args) => list(args, global, &context).await,
        PackageCommand::Status(args) => status(args, global, &context).await,
    }
}

async fn list(
    args: &PackageListArgs,
    global: &GlobalArgs,
    context: &AppGalleryContext,
) -> Result<()> {
    let response = context
        .client
        .get_package_list(&args.app_id, Some(args.offset), Some(args.limit), None)
        .await?
        .into_inner();
    ensure_success(response.ret.code, &response.ret.msg)?;
    if global.json.is_some() {
        return print_json(&response.pkg_list, global.json.as_deref());
    }
    let rows = response
        .pkg_list
        .into_iter()
        .map(|package| {
            vec![
                package.file_name,
                package.version_name.unwrap_or_else(|| "-".to_string()),
                package.version_code.to_string(),
                package.pkg_version,
                package.package_size.to_string(),
            ]
        })
        .collect::<Vec<_>>();
    print_table(
        &["FILE", "VERSION", "VERSION_CODE", "PACKAGE_ID", "SIZE"],
        &rows,
    );
    Ok(())
}

async fn status(
    args: &PackageStatusArgs,
    global: &GlobalArgs,
    context: &AppGalleryContext,
) -> Result<()> {
    if args.package_id.is_empty() {
        anyhow::bail!("at least one PACKAGE_ID is required");
    }
    let package_ids = args.package_id.join(",");
    let response = context
        .client
        .get_package_compile_status(&args.app_id, &package_ids, None)
        .await?
        .into_inner();
    ensure_success(response.ret.code, &response.ret.msg)?;
    if global.json.is_some() {
        return print_json(&response, global.json.as_deref());
    }
    let rows = response
        .pkg_state_list
        .into_iter()
        .map(|state| {
            vec![
                state.pkg_id,
                state.success_status.to_string(),
                state
                    .aab_compile_status
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                state
                    .fail_reason
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string()),
            ]
        })
        .collect::<Vec<_>>();
    print_table(
        &["PACKAGE_ID", "STATUS", "AAB_COMPILE_STATUS", "FAIL_REASON"],
        &rows,
    );
    Ok(())
}
