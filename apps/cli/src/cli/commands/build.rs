use anyhow::{Result, anyhow};
use clap::Args;
use fastforge_app_builder::{FlutterAppBuilder, Platform};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Args)]
pub struct BuildArgs {
    /// Target platform (auto-detected from the target and project layout
    /// when omitted).
    #[arg(short, long = "platform")]
    pub platform: Option<String>,
    #[arg(short, long = "target")]
    pub target: Option<String>,
    #[arg(long = "clean", default_value_t = false)]
    pub clean: bool,
    #[arg(long = "flutter-build-args")]
    pub flutter_build_args: Option<String>,
    #[arg(long = "build-target")]
    pub build_target: Option<String>,
    #[arg(long = "build-flavor")]
    pub build_flavor: Option<String>,
    #[arg(long = "build-target-platform")]
    pub build_target_platform: Option<String>,
    #[arg(long = "build-export-options-plist")]
    pub build_export_options_plist: Option<String>,
    #[arg(long = "build-export-method")]
    pub build_export_method: Option<String>,
    #[arg(long = "build-dart-define")]
    pub build_dart_define: Vec<String>,
    #[arg(long = "build-obfuscate", default_value_t = false)]
    pub build_obfuscate: bool,
    #[arg(long = "build-split-debug-info")]
    pub build_split_debug_info: Option<String>,
    #[arg(long = "build-tree-shake-icons", default_value_t = false)]
    pub build_tree_shake_icons: bool,
    #[arg(long = "build-profile", default_value_t = false)]
    pub build_profile: bool,
}

pub async fn execute(args: &BuildArgs) -> Result<()> {
    log::info!("Executing build command");
    let platform: Platform = match args.platform.as_deref() {
        Some(platform_str) => Platform::from_str(platform_str)
            .map_err(|e| anyhow!("Invalid platform '{}': {}", platform_str, e))?,
        None => {
            let targets: Vec<&str> = args.target.as_deref().into_iter().collect();
            super::platform_infer::infer_platform(&targets)?
        }
    };

    let mut build_arguments = generate_build_args(args);
    merge_flutter_build_args(&mut build_arguments, args.flutter_build_args.as_deref())?;

    let env: HashMap<String, String> = std::env::vars().collect();
    let builder = FlutterAppBuilder::default();
    if args.clean {
        builder.clean(Some(&env)).map_err(|e| anyhow!("{}", e))?;
    }

    let result = builder
        .build(
            &platform,
            args.target.as_deref(),
            build_arguments,
            Some(env),
        )
        .map_err(|e| anyhow!("{}", e))?;

    println!(
        "{}",
        serde_json::to_string_pretty(&result.to_json_compatible())?
    );
    Ok(())
}

fn generate_build_args(args: &BuildArgs) -> Map<String, Value> {
    let mut build_arguments = Map::<String, Value>::new();

    if let Some(value) = &args.build_target {
        build_arguments.insert("target".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &args.build_flavor {
        build_arguments.insert("flavor".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &args.build_target_platform {
        build_arguments.insert("target-platform".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &args.build_export_options_plist {
        build_arguments.insert(
            "export-options-plist".to_string(),
            Value::String(value.clone()),
        );
    }
    if let Some(value) = &args.build_export_method {
        build_arguments.insert("export-method".to_string(), Value::String(value.clone()));
    }
    if args.build_obfuscate {
        build_arguments.insert("obfuscate".to_string(), Value::Bool(true));
    }
    if let Some(value) = &args.build_split_debug_info {
        build_arguments.insert("split-debug-info".to_string(), Value::String(value.clone()));
    }
    if args.build_tree_shake_icons {
        build_arguments.insert("tree-shake-icons".to_string(), Value::Bool(true));
    }
    if args.build_profile {
        build_arguments.insert("profile".to_string(), Value::Bool(true));
    }
    if !args.build_dart_define.is_empty() {
        let mut map = Map::<String, Value>::new();
        for item in &args.build_dart_define {
            if let Some((k, v)) = item.split_once('=') {
                map.insert(k.to_string(), Value::String(v.to_string()));
            }
        }
        if !map.is_empty() {
            build_arguments.insert("dart-define".to_string(), Value::Object(map));
        }
    }

    build_arguments
}

fn merge_flutter_build_args(
    build_arguments: &mut Map<String, Value>,
    flutter_build_args: Option<&str>,
) -> Result<()> {
    for arg in flutter_build_args.unwrap_or_default().split(',') {
        let trimmed = arg.trim();
        if trimmed.is_empty() {
            continue;
        }
        let pieces: Vec<&str> = trimmed.split('=').collect();
        match pieces.len() {
            1 => {
                build_arguments.insert(pieces[0].to_string(), Value::Bool(true));
            }
            2 => {
                build_arguments.insert(pieces[0].to_string(), Value::String(pieces[1].to_string()));
            }
            _ => {
                return Err(anyhow!("Invalid --flutter-build-args item: {}", trimmed));
            }
        }
    }
    Ok(())
}
