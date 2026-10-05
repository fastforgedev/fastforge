use clap::{Parser, Subcommand};

mod cli;
mod config;
mod utils;

use cli::{
    AnalyzeArgs, BuildArgs, HostArgs, PackageArgs, PublishArgs, ReleaseArgs, RemoteAgentArgs,
    RunArgs, StoreArgs, UpgradeArgs, VersionCheckArgs, WorkflowArgs,
};
use fastforge_app_gallery_connect::cli::AppGalleryConnectArgs;
use fastforge_app_store_connect::cli::AppStoreConnectArgs;
use fastforge_google_play_console::cli::GooglePlayConsoleArgs;

#[derive(Parser)]
#[command(name = "fastforge")]
#[command(about = "Package and publish your apps with ease.")]
#[command(version = env!("FASTFORGE_BUILD_VERSION"))]
struct Cli {
    /// Check for updates when this command runs (default: on).
    #[arg(
        long = "version-check",
        global = true,
        overrides_with = "no_version_check"
    )]
    version_check: bool,
    /// Do not check for updates when this command runs.
    #[arg(
        long = "no-version-check",
        global = true,
        overrides_with = "version_check"
    )]
    no_version_check: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Analyze your project")]
    Analyze(AnalyzeArgs),
    #[command(about = "Build your project")]
    Build(BuildArgs),
    #[command(
        about = "Package the current Flutter application for distribution",
        long_about = "Package the current Flutter application for distribution\n\n\
                      Options prefixed with --build- are passed directly to 'flutter build'\n\
                      For more details on build options, refer to the 'flutter build' documentation."
    )]
    Package(PackageArgs),
    #[command(
        about = "Publish the built Flutter application artifacts to distribution platforms",
        long_about = "Publish the built Flutter application artifacts to distribution platforms\n\n\
                      This command uploads your application bundle to specified target providers\n\
                      Use --targets to specify one or more distribution platforms"
    )]
    Publish(Box<PublishArgs>),
    #[command(about = "Release the current Flutter application")]
    Release(ReleaseArgs),
    #[command(
        about = "Run the app on a device, like `flutter run`",
        long_about = "Run the app on a device, like `flutter run`\n\n\
                      Arguments after `--` are passed to 'flutter run'. With --host the app runs on a\n\
                      remote host; hot reload (r/R) syncs the project first and the URLs it prints\n\
                      (VM service, DevTools, web server) are forwarded to this machine."
    )]
    Run(RunArgs),
    #[command(about = "Manage remote hosts that package and publish on other machines")]
    Host(HostArgs),
    #[command(name = "remote-agent", hide = true)]
    RemoteAgent(RemoteAgentArgs),
    #[command(about = "Manage distribution store configuration")]
    Store(StoreArgs),
    #[command(about = "Open Studio to manage local projects")]
    Studio(studio_cli::StudioArgs),
    #[command(about = "Update Fastforge to the latest version.")]
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

/// Routes `log`/`tracing` records to stderr. Quiet by default (warnings and
/// errors and Studio's server address); set `RUST_LOG=info` (or `debug`) for
/// more detail.
fn init_logging() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn,studio_cli=info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .without_time()
        .try_init();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging();
    // The remote agent's output is relayed to a client; keep it to the run's own.
    if !std::env::args().any(|arg| arg == "remote-agent") {
        print_rename_notice_if_needed();
        print_local_build_notice_if_needed();
    }
    let cli = Cli::parse();

    // Mirrors the Dart CLI's `--[no-]version-check` (default on). The
    // `upgrade` / `version-check` commands perform their own check.
    let checks_itself = matches!(
        cli.command,
        Commands::Upgrade(_) | Commands::VersionCheck(_) | Commands::RemoteAgent(_)
    );
    if !cli.no_version_check && !checks_itself {
        cli::version_check::run_startup_check().await;
    }

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
        Commands::Run(args) => {
            cli::run::execute(args).await?;
        }
        Commands::Host(args) => {
            cli::host::execute(args).await?;
        }
        Commands::RemoteAgent(args) => {
            cli::remote_agent::execute(args).await?;
        }
        Commands::Store(args) => {
            cli::store::execute(args).await?;
        }
        Commands::Studio(args) => {
            studio_cli::execute(args).await?;
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
