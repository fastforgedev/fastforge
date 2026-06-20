use anyhow::{Result, anyhow};
use clap::Args;
use serde_json::{Map, Value};
use std::collections::HashMap;

use crate::config::DistributeOptions;
use crate::config::release_job::ReleaseJob;

use super::package::package_flutter_artifact;
use super::publish::publish_artifact;

#[derive(Args)]
pub struct ReleaseArgs {
    /// Name of the release to run (matches a `name:` key in
    /// `distribute_options.yaml`). Required.
    #[arg(long = "name", value_name = "NAME")]
    pub name: Option<String>,

    /// Comma-separated list of job names to run.
    /// When specified, only these jobs are executed.
    #[arg(long = "jobs", value_name = "JOB,...")]
    pub jobs: Option<String>,

    /// Comma-separated list of job names to skip.
    /// Ignored when `--jobs` is also provided.
    #[arg(long = "skip-jobs", value_name = "JOB,...")]
    pub skip_jobs: Option<String>,

    /// Skip `flutter clean` before packaging.
    #[arg(long = "skip-clean", default_value_t = false)]
    pub skip_clean: bool,

    /// Perform a dry run: print which jobs would execute without actually
    /// running them.
    #[arg(long = "dry-run", default_value_t = false)]
    pub dry_run: bool,
}

pub async fn execute(args: &ReleaseArgs) -> Result<()> {
    let release_name = args
        .name
        .as_deref()
        .ok_or_else(|| anyhow!("The 'name' option is mandatory!"))?;

    let job_names: Vec<String> = args
        .jobs
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();

    let skip_job_names: Vec<String> = args
        .skip_jobs
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();

    let opts = DistributeOptions::load()?;

    // Resolve global variables: env vars < distribute_options.variables
    let mut global_vars: std::collections::HashMap<String, String> = std::env::vars().collect();
    global_vars.extend(opts.resolved_variables());

    // Select matching releases
    let matching: Vec<_> = opts
        .releases
        .iter()
        .filter(|r| r.name == release_name)
        .collect();

    if matching.is_empty() {
        return Err(anyhow!(
            "No release named '{}' found in distribute_options.yaml.",
            release_name
        ));
    }

    for release in &matching {
        let filtered_jobs = release.filter_jobs(&job_names, &skip_job_names);

        if filtered_jobs.is_empty() {
            return Err(anyhow!(
                "No available jobs found in release '{}'.",
                release.name
            ));
        }

        for job in &filtered_jobs {
            // Merge variables: global < release-level < job-level
            let mut _vars = global_vars.clone();
            if let Some(rv) = &release.variables {
                _vars.extend(rv.clone());
            }
            if let Some(jv) = &job.variables {
                _vars.extend(jv.clone());
            }

            log::info!(
                "===> Releasing {}:{} (platform={}, target={})",
                release.name,
                job.name,
                job.package.platform,
                job.package.target,
            );

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

            let artifacts = package_flutter_artifact(
                &job.package.platform,
                &job.package.target,
                yaml_map_to_json_map(job.package.build_args.as_ref())?,
                _vars.clone(),
                &opts.output,
                opts.artifact_name.clone(),
                !args.skip_clean,
                job.package.hooks.as_ref(),
            )?;

            for artifact in &artifacts {
                println!(
                    "Release job '{}:{}': packaged {}",
                    release.name,
                    job.name,
                    artifact.display(),
                );
            }

            if let Some(target) = job.publish_target() {
                let publish_args = publish_args(job, &_vars)?;
                for artifact in &artifacts {
                    let message = publish_artifact(
                        &artifact.to_string_lossy(),
                        target,
                        publish_args.clone(),
                    )?;
                    println!(
                        "Release job '{}:{}': published to {} ({})",
                        release.name, job.name, target, message,
                    );
                }
            }
        }
    }

    Ok(())
}

fn yaml_map_to_json_map(
    map: Option<&HashMap<String, serde_yaml::Value>>,
) -> Result<Map<String, Value>> {
    let mut output = Map::new();
    for (key, value) in map.into_iter().flat_map(|m| m.iter()) {
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

    if let Some(publish) = &job.publish {
        if let Some(raw_args) = &publish.args {
            for (key, value) in raw_args {
                args.insert(key.clone(), yaml_value_to_string(key, value)?);
            }
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
    if !args.contains_key(arg_key) {
        if let Some(value) = variables.get(env_key).filter(|v| !v.trim().is_empty()) {
            args.insert(arg_key.to_string(), value.clone());
        }
    }
}

fn yaml_value_to_string(key: &str, value: &serde_yaml::Value) -> Result<String> {
    match value {
        serde_yaml::Value::String(value) => Ok(value.clone()),
        serde_yaml::Value::Number(value) => Ok(value.to_string()),
        serde_yaml::Value::Bool(value) => Ok(value.to_string()),
        serde_yaml::Value::Null => Ok(String::new()),
        _ => Err(anyhow!("Publish arg '{key}' must be a scalar value")),
    }
}
