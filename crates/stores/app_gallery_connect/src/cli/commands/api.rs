use crate::AppGalleryContext;
use crate::cli::GlobalArgs;
use anyhow::{Context as _, Result, anyhow};
use clap::{Args, Subcommand};
use serde_json::Value;
use std::fs;

#[derive(Args, Debug)]
pub struct ApiArgs {
    #[command(subcommand)]
    pub command: ApiCommand,
}

#[derive(Subcommand, Debug)]
pub enum ApiCommand {
    Get(ApiRequestArgs),
    Post(ApiBodyRequestArgs),
    Put(ApiBodyRequestArgs),
    Patch(ApiBodyRequestArgs),
    Delete(ApiRequestArgs),
}

#[derive(Args, Debug)]
pub struct ApiRequestArgs {
    #[arg(value_name = "PATH")]
    pub path: String,
    #[arg(long = "query", value_name = "KEY=VALUE")]
    pub query: Vec<String>,
}

#[derive(Args, Debug)]
pub struct ApiBodyRequestArgs {
    #[arg(value_name = "PATH")]
    pub path: String,
    #[arg(long = "query", value_name = "KEY=VALUE")]
    pub query: Vec<String>,
    #[arg(long = "input")]
    pub input: Option<String>,
}

pub async fn execute(args: &ApiArgs, _global: &GlobalArgs) -> Result<()> {
    let context = AppGalleryContext::from_env().await?;
    match &args.command {
        ApiCommand::Get(args) => send_without_body(&context, reqwest::Method::GET, args).await,
        ApiCommand::Delete(args) => {
            send_without_body(&context, reqwest::Method::DELETE, args).await
        }
        ApiCommand::Post(args) => send_with_body(&context, reqwest::Method::POST, args).await,
        ApiCommand::Put(args) => send_with_body(&context, reqwest::Method::PUT, args).await,
        ApiCommand::Patch(args) => send_with_body(&context, reqwest::Method::PATCH, args).await,
    }
}

async fn send_without_body(
    context: &AppGalleryContext,
    method: reqwest::Method,
    args: &ApiRequestArgs,
) -> Result<()> {
    validate_path(&args.path)?;
    let mut request = context.http.request(method, context.api_url(&args.path));
    for (key, value) in parse_query(&args.query)? {
        request = request.query(&[(key, value)]);
    }
    print_response(request.send().await?).await
}

async fn send_with_body(
    context: &AppGalleryContext,
    method: reqwest::Method,
    args: &ApiBodyRequestArgs,
) -> Result<()> {
    validate_path(&args.path)?;
    let mut request = context.http.request(method, context.api_url(&args.path));
    for (key, value) in parse_query(&args.query)? {
        request = request.query(&[(key, value)]);
    }
    if let Some(input) = &args.input {
        let content = fs::read_to_string(input)
            .with_context(|| format!("failed to read input JSON file: {input}"))?;
        let body: Value = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse input JSON file: {input}"))?;
        request = request.json(&body);
    }
    print_response(request.send().await?).await
}

async fn print_response(response: reqwest::Response) -> Result<()> {
    let status = response.status();
    let text = response.text().await?;
    if !status.is_success() {
        return Err(anyhow!("AppGallery API request failed: {status}\n{text}"));
    }
    if text.trim().is_empty() {
        return Ok(());
    }
    match serde_json::from_str::<Value>(&text) {
        Ok(value) => println!("{}", serde_json::to_string_pretty(&value)?),
        Err(_) => println!("{text}"),
    }
    Ok(())
}

fn validate_path(path: &str) -> Result<()> {
    if !path.starts_with("/api/") || path.contains("://") {
        return Err(anyhow!("API path must start with /api/"));
    }
    Ok(())
}

fn parse_query(items: &[String]) -> Result<Vec<(&str, &str)>> {
    items
        .iter()
        .map(|item| {
            item.split_once('=')
                .ok_or_else(|| anyhow!("invalid --query item `{item}`; expected KEY=VALUE"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_accepts_relative_api_paths() {
        assert!(validate_path("/api/publish/v2/app-info").is_ok());
        assert!(validate_path("https://example.com/api/test").is_err());
        assert!(validate_path("/publish/v2/app-info").is_err());
    }
}
