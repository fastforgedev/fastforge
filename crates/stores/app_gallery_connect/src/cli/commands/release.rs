use crate::AppGalleryContext;
use crate::cli::GlobalArgs;
use crate::cli::commands::app::ensure_success;
use anyhow::Result;
use clap::Args;

#[derive(Args, Debug)]
pub struct ReleaseArgs {
    #[arg(value_name = "APP_ID")]
    pub app_id: String,
    #[arg(long, default_value_t = 1)]
    pub release_type: i32,
    #[arg(long, value_name = "UTC_TIME")]
    pub release_time: Option<String>,
}

pub async fn execute(args: &ReleaseArgs, _global: &GlobalArgs) -> Result<()> {
    let context = AppGalleryContext::from_env().await?;
    let response = context
        .client
        .submit_app(
            &args.app_id,
            args.release_time.as_deref(),
            Some(args.release_type),
            None,
        )
        .await?
        .into_inner();
    ensure_success(response.ret.code, &response.ret.msg)?;
    println!("Submitted AppGallery app {} for review.", args.app_id);
    Ok(())
}
