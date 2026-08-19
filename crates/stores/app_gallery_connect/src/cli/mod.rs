pub mod commands;

use anyhow::Result;
use clap::{Args, Subcommand};

pub use commands::api::ApiArgs;
pub use commands::app::AppArgs;
pub use commands::package::PackageArgs;
pub use commands::release::ReleaseArgs;

#[derive(Args, Debug)]
pub struct AppGalleryConnectArgs {
    #[command(flatten)]
    pub global: GlobalArgs,

    #[command(subcommand)]
    pub command: AppGalleryConnectCommand,
}

#[derive(Subcommand, Debug)]
pub enum AppGalleryConnectCommand {
    #[command(about = "Resolve and query AppGallery apps")]
    App(AppArgs),
    #[command(about = "Query AppGallery packages and compilation status")]
    Package(PackageArgs),
    #[command(about = "Submit an AppGallery app for review")]
    Release(ReleaseArgs),
    #[command(about = "Call raw AppGallery Connect API endpoints")]
    Api(ApiArgs),
}

pub async fn execute(root: &AppGalleryConnectArgs) -> Result<()> {
    match &root.command {
        AppGalleryConnectCommand::App(args) => commands::app::execute(args, &root.global).await,
        AppGalleryConnectCommand::Package(args) => {
            commands::package::execute(args, &root.global).await
        }
        AppGalleryConnectCommand::Release(args) => {
            commands::release::execute(args, &root.global).await
        }
        AppGalleryConnectCommand::Api(args) => commands::api::execute(args, &root.global).await,
    }
}

#[derive(Args, Debug, Clone)]
pub struct GlobalArgs {
    #[arg(long = "json", value_name = "FIELDS", global = true)]
    pub json: Option<String>,
    #[arg(long = "verbose", default_value_t = false, global = true)]
    pub verbose: bool,
    #[arg(long = "debug", default_value_t = false, global = true)]
    pub debug: bool,
    #[arg(long = "no-color", default_value_t = false, global = true)]
    pub no_color: bool,
}
