use anyhow::Result;
use clap::Parser;
use fastforge_app_gallery_connect::cli::{self, AppGalleryConnectArgs};

#[derive(Parser)]
#[command(name = "fastforge_app_gallery_connect")]
#[command(about = "Huawei AppGallery Connect command line tool.")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(flatten)]
    command: AppGalleryConnectArgs,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    cli::execute(&cli.command).await
}
