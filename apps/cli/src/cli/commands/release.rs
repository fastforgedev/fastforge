use anyhow::{Context, Result, anyhow};
use clap::Args;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::time::Instant;

use crate::config::DistributeOptions;
use crate::config::distribute_options::resolve_env_references;
use crate::config::release::ReleaseJob;
use crate::utils::global_variables;

use super::package::{PackageRequest, package};
use super::publish::publish_artifact_with_env;

#[derive(Args)]
pub struct ReleaseArgs {
    /// The name of the release to run. When omitted, every release in
    /// `distribute_options.yaml` runs (like the Dart CLI).
    #[arg(long = "name", value_name = "NAME")]
    pub name: Option<String>,

    /// Comma-separated list of jobs to run for the specified release.
    #[arg(long = "jobs", value_name = "JOB,...")]
    pub jobs: Option<String>,

    /// Comma-separated list of jobs to skip for the specified release.
    #[arg(long = "skip-jobs", value_name = "JOB,...")]
    pub skip_jobs: Option<String>,

    /// Whether or not to skip 'flutter clean' before packaging.
    #[arg(long = "skip-clean", overrides_with = "no_skip_clean")]
    pub skip_clean: bool,
    #[arg(long = "no-skip-clean", overrides_with = "skip_clean", hide = true)]
    pub no_skip_clean: bool,

    /// Perform a dry run: print which jobs would execute without actually
    /// running them.
    #[arg(long = "dry-run", default_value_t = false)]
    pub dry_run: bool,
}

fn split_list(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

pub async fn execute(args: &ReleaseArgs) -> Result<()> {
    let started = Instant::now();
    let result = run(args);
    let seconds = started.elapsed().as_secs();
    if args.dry_run {
        return result;
    }
    println!();
    match &result {
        Ok(()) => println!("\x1b[1;32mRELEASE SUCCESSFUL in {}s\x1b[0m", seconds),
        Err(error) => eprintln!(
            "\x1b[1;31mRELEASE FAILED in {}s\x1b[0m\n\x1b[31m{:#}\x1b[0m",
            seconds, error
        ),
    }
    result
}

/// Dart's `UnifiedDistributor.release`.
fn run(args: &ReleaseArgs) -> Result<()> {
    let release_name = args.name.clone().unwrap_or_default();
    let job_names = split_list(args.jobs.as_deref());
    let skip_job_names = split_list(args.skip_jobs.as_deref());

    let opts = DistributeOptions::load()?;
    std::fs::create_dir_all(&opts.output)
        .with_context(|| format!("Failed to create {}", opts.output))?;

    let global_vars = global_variables(&opts);

    let releases: Vec<_> = opts
        .releases
        .iter()
        .filter(|r| release_name.is_empty() || r.name == release_name)
        .collect();
    if releases.is_empty() {
        return Err(anyhow!(
            "Missing/incomplete `distribute_options.yaml` file.{}",
            if release_name.is_empty() {
                String::new()
            } else {
                format!(" (no release named '{}')", release_name)
            }
        ));
    }

    for release in releases {
        let filtered_jobs = release.filter_jobs(&job_names, &skip_job_names);
        if filtered_jobs.is_empty() {
            return Err(anyhow!("No available jobs found in {}.", release.name));
        }

        // Clean at most once per release (mirrors Dart).
        let mut clean_before_build = !args.skip_clean;

        for job in filtered_jobs {
            println!();
            println!(
                "\x1b[34m===>\x1b[0m \x1b[1;37mReleasing\x1b[0m {}:\x1b[1;32m{}\x1b[0m",
                release.name, job.name
            );

            // global < release-level < job-level
            let mut variables = global_vars.clone();
            // An empty value (such as an unset `${NAME}`) leaves the key to
            // the outer level, as for global variables.
            for level in [&release.variables, &job.variables].into_iter().flatten() {
                variables.extend(
                    resolve_env_references(level.clone())
                        .into_iter()
                        .filter(|(_, value)| !value.is_empty()),
                );
            }

            if args.dry_run {
                println!(
                    "[dry-run] Would package {}:{} ({}/{})",
                    release.name, job.name, job.package.platform, job.package.target,
                );
                if let Some(target) = job.publish_target() {
                    println!(
                        "[dry-run] Would publish {}:{} to {}",
                        release.name, job.name, target,
                    );
                }
                continue;
            }

            let targets = [job.package.target.clone()];
            let packaged = package(PackageRequest {
                platform: &job.package.platform,
                targets: &targets,
                channel: job.package.channel.clone(),
                artifact_name: opts.artifact_name.clone(),
                clean_before_build,
                skip_build: false,
                build_arguments: yaml_map_to_json_map(job.package.build_args.as_ref())?,
                variables: variables.clone(),
                hooks: job.package.hooks.as_ref(),
                output: &opts.output,
            })?;
            clean_before_build = false;

            if let Some(target) = job.publish_target() {
                // Like Dart, only the first artifact of the first result is
                // published.
                let artifact = packaged
                    .first()
                    .and_then(|result| result.artifacts.first())
                    .ok_or_else(|| {
                        anyhow!(
                            "Job '{}' produced no artifact to publish to {}.",
                            job.name,
                            target
                        )
                    })?;
                publish_artifact_with_env(
                    &artifact.to_string_lossy(),
                    target,
                    publish_args(job, &variables)?,
                    variables.clone(),
                )?;
            }
        }
    }

    Ok(())
}

fn yaml_map_to_json_map(map: Option<&serde_yaml::Mapping>) -> Result<Map<String, Value>> {
    let mut output = Map::new();
    for (key, value) in map.into_iter().flat_map(|m| m.iter()) {
        let key = match key {
            serde_yaml::Value::String(key) => key.clone(),
            other => serde_yaml::to_string(other)?.trim().to_string(),
        };
        output.insert(
            key.clone(),
            serde_json::to_value(value).map_err(|e| anyhow!("Invalid build arg {key}: {e}"))?,
        );
    }
    Ok(output)
}

fn publish_args(
    job: &ReleaseJob,
    variables: &HashMap<String, String>,
) -> Result<HashMap<String, String>> {
    let mut args = HashMap::new();

    if let Some(publish) = &job.publish
        && let Some(raw_args) = &publish.args
    {
        for (key, value) in raw_args {
            args.insert(key.clone(), yaml_value_to_string(key, value)?);
        }
    }

    copy_variable_arg(&mut args, variables, "APPSTORE_USERNAME", "username");
    copy_variable_arg(&mut args, variables, "APPSTORE_PASSWORD", "password");
    copy_variable_arg(&mut args, variables, "APPSTORE_APIKEY", "api-key");
    copy_variable_arg(&mut args, variables, "APPSTORE_APIISSUER", "api-issuer");
    copy_variable_arg(&mut args, variables, "APP_STORE_CONNECT_KEY_ID", "key-id");
    copy_variable_arg(
        &mut args,
        variables,
        "APP_STORE_CONNECT_ISSUER_ID",
        "issuer-id",
    );
    copy_variable_arg(
        &mut args,
        variables,
        "APP_STORE_CONNECT_KEY_PATH",
        "key-path",
    );

    Ok(args)
}

fn copy_variable_arg(
    args: &mut HashMap<String, String>,
    variables: &HashMap<String, String>,
    env_key: &str,
    arg_key: &str,
) {
    if !args.contains_key(arg_key)
        && let Some(value) = variables.get(env_key).filter(|v| !v.trim().is_empty())
    {
        args.insert(arg_key.to_string(), value.clone());
    }
}

/// Publish arguments are strings; a list of `key=value` items (which Dart
/// turns into a map) is joined with commas.
fn yaml_value_to_string(key: &str, value: &serde_yaml::Value) -> Result<String> {
    match value {
        serde_yaml::Value::String(value) => Ok(value.clone()),
        serde_yaml::Value::Number(value) => Ok(value.to_string()),
        serde_yaml::Value::Bool(value) => Ok(value.to_string()),
        serde_yaml::Value::Null => Ok(String::new()),
        serde_yaml::Value::Sequence(items) => items
            .iter()
            .map(|item| yaml_value_to_string(key, item))
            .collect::<Result<Vec<_>>>()
            .map(|items| items.join(",")),
        _ => Err(anyhow!("Publish arg '{key}' must be a scalar value")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_args_keep_yaml_order() {
        let mapping: serde_yaml::Mapping =
            serde_yaml::from_str("zeta: 1\nalpha: true\nmid: x\n").unwrap();
        let map = yaml_map_to_json_map(Some(&mapping)).unwrap();
        let keys: Vec<_> = map.keys().cloned().collect();
        assert_eq!(keys, ["zeta", "alpha", "mid"]);
    }
}
