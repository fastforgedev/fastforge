use clap::{Parser, Subcommand};

mod cli;
mod config;

use cli::{
    AnalyzeArgs, BuildArgs, PackageArgs, PublishArgs, ReleaseArgs, StoreArgs, UpgradeArgs,
    VersionCheckArgs, WorkflowArgs,
};
use fastforge_app_gallery_connect::cli::AppGalleryConnectArgs;
use fastforge_app_store_connect::cli::AppStoreConnectArgs;
use fastforge_google_play_console::cli::GooglePlayConsoleArgs;

#[derive(Parser)]
#[command(name = "fastforge")]
#[command(about = "Package and publish your apps with ease.")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Analyze your project")]
    Analyze(AnalyzeArgs),
    #[command(about = "Build your project")]
    Build(BuildArgs),
    #[command(about = "Package your project")]
    Package(PackageArgs),
    #[command(about = "Publish your project")]
    Publish(PublishArgs),
    #[command(about = "Release the current Flutter application")]
    Release(ReleaseArgs),
    #[command(about = "Manage distribution store configuration")]
    Store(StoreArgs),
    #[command(about = "Update Fastforge to the latest version")]
    Upgrade(UpgradeArgs),
    #[command(
        name = "version-check",
        about = "Check for a newer version of fastforge"
    )]
    VersionCheck(VersionCheckArgs),
    #[command(about = "Execute workflows locally")]
    Workflow(WorkflowArgs),
    #[command(name = "appstore", about = "Use App Store Connect")]
    AppStore(AppStoreConnectArgs),
    #[command(name = "appgallery", about = "Use Huawei AppGallery Connect")]
    AppGallery(AppGalleryConnectArgs),
    #[command(name = "googleplay", about = "Use Google Play Console")]
    GooglePlay(GooglePlayConsoleArgs),
}

/// Prints the rename notice when the binary is invoked under the legacy
/// `flutter_distributor` name (mirrors the Dart CLI's banner).
fn print_rename_notice_if_needed() {
    let invoked_as = std::env::args()
        .next()
        .map(std::path::PathBuf::from)
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
        .unwrap_or_default();
    if invoked_as.starts_with("flutter_distributor") {
        eprintln!(
            "\x1b[1;33m╔════════════════════════════════════════════════════════════════════════════╗\n\
             ║ Important Notice: flutter_distributor has been renamed to fastforge.       ║\n\
             ║ You can continue to use flutter_distributor, but we recommend migrating to ║\n\
             ║ fastforge for the latest features and updates.                             ║\n\
             ║                                                                            ║\n\
             ║ Please visit https://fastforge.dev for more information.                   ║\n\
             ╚════════════════════════════════════════════════════════════════════════════╝\x1b[0m\n"
        );
    }
}

/// Prints a one-line notice when the binary was not produced by the GitHub
/// Actions release pipeline, i.e. it was built/installed locally from source.
///
/// `GITHUB_ACTIONS` is set in every GitHub Actions step and captured here at
/// compile time, so official (Actions-built) binaries stay silent.
fn print_local_build_notice_if_needed() {
    if option_env!("GITHUB_ACTIONS").is_none() {
        eprintln!(
            "\x1b[1;30;43m ⚠ UNOFFICIAL BUILD \x1b[0m \x1b[1;33mbuilt locally; official releases: https://fastforge.dev\x1b[0m\n"
        );
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    print_rename_notice_if_needed();
    print_local_build_notice_if_needed();
    let cli = Cli::parse();

    match &cli.command {
        Commands::Analyze(args) => {
            cli::analyze::execute(args).await?;
        }
        Commands::Build(args) => {
            cli::build::execute(args).await?;
        }
        Commands::Package(args) => {
            cli::package::execute(args).await?;
        }
        Commands::Publish(args) => {
            cli::publish::execute(args).await?;
        }
        Commands::Release(args) => {
            cli::release::execute(args).await?;
        }
        Commands::Store(args) => {
            cli::store::execute(args).await?;
        }
        Commands::Upgrade(args) => {
            cli::upgrade::execute(args).await?;
        }
        Commands::VersionCheck(args) => {
            cli::version_check::execute(args).await?;
        }
        Commands::Workflow(args) => {
            cli::workflow::execute(args).await?;
        }
        Commands::AppStore(args) => {
            fastforge_app_store_connect::cli::execute(args).await?;
        }
        Commands::AppGallery(args) => {
            fastforge_app_gallery_connect::cli::execute(args).await?;
        }
        Commands::GooglePlay(args) => {
            fastforge_google_play_console::cli::execute(args).await?;
        }
    }

    Ok(())
}
