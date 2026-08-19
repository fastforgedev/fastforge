use anyhow::{Result, anyhow};
use async_trait::async_trait;
use clap::{Args, Subcommand};
use fastforge_app_store_connect::AppStoreConnectContext;
use fastforge_app_store_connect::cli::commands::catalog::{
    pull as appstore_pull, push as appstore_push,
};
use fastforge_google_play_console::GooglePlayContext;
use fastforge_google_play_console::cli::commands::catalog::{
    pull as googleplay_pull, push as googleplay_push,
};

use crate::config::Config;

#[derive(Args, Debug)]
pub struct StoreArgs {
    #[command(subcommand)]
    pub command: StoreCommands,
}

#[derive(Subcommand, Debug)]
pub enum StoreCommands {
    #[command(about = "List configured distribution stores")]
    List,
    #[command(about = "Manage catalog data for all configured store apps")]
    Catalog(StoreCatalogArgs),
}

#[derive(Args, Debug)]
pub struct StoreCatalogArgs {
    #[command(subcommand)]
    pub command: StoreCatalogCommand,
}

#[derive(Subcommand, Clone, Copy, Debug, Eq, PartialEq)]
pub enum StoreCatalogCommand {
    #[command(about = "Pull catalog data for all configured store apps")]
    Pull,
    #[command(about = "Push catalog data for all configured store apps")]
    Push,
}

pub async fn execute(args: &StoreArgs) -> Result<()> {
    match &args.command {
        StoreCommands::List => list_stores(),
        StoreCommands::Catalog(args) => execute_catalog(args.command).await,
    }
}

fn list_stores() -> Result<()> {
    let config = Config::load()?;
    if config.stores.is_empty() {
        return Err(anyhow!("No stores found in {}.", Config::DEFAULT_PATH));
    }

    if let Some(appstore) = &config.stores.appstore {
        println!("appstore ({})", appstore.auth.auth_type());
        if appstore.apps.is_empty() {
            println!("  apps: none");
        } else {
            for app in &appstore.apps {
                let identifier = app.identifier().unwrap_or("<unknown>");
                let label = app.name.as_deref().unwrap_or(identifier);
                println!("  - {} [ios] {}", label, identifier);
            }
        }
    }

    if let Some(appgallery) = &config.stores.appgallery {
        println!("appgallery ({})", appgallery.auth.auth_type());
        if appgallery.apps.is_empty() {
            println!("  apps: none");
        } else {
            for app in &appgallery.apps {
                let identifier = app.identifier().unwrap_or("<unknown>");
                let label = app.name.as_deref().unwrap_or(identifier);
                println!("  - {} [android] {}", label, identifier);
            }
        }
    }

    if let Some(googleplay) = &config.stores.googleplay {
        println!("googleplay ({})", googleplay.auth.auth_type());
        if googleplay.apps.is_empty() {
            println!("  apps: none");
        } else {
            for app in &googleplay.apps {
                let identifier = app.identifier().unwrap_or("<unknown>");
                println!("  - {} [android] {}", identifier, identifier);
            }
        }
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StoreKind {
    AppStore,
    GooglePlay,
}

impl StoreKind {
    fn name(self) -> &'static str {
        match self {
            Self::AppStore => "appstore",
            Self::GooglePlay => "googleplay",
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct CatalogTarget {
    store: StoreKind,
    identifier: Option<String>,
    config_index: usize,
}

impl CatalogTarget {
    fn label(&self) -> String {
        self.identifier
            .clone()
            .unwrap_or_else(|| format!("<missing identifier at apps[{}]>", self.config_index))
    }
}

#[derive(Debug)]
struct CatalogOutcome {
    store: StoreKind,
    identifier: String,
    error: Option<String>,
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.filter(|value| !value.trim().is_empty())
}

fn configured_catalog_targets(config: &Config) -> Vec<CatalogTarget> {
    let mut targets = Vec::new();

    if let Some(appstore) = &config.stores.appstore {
        targets.extend(appstore.apps.iter().enumerate().map(|(config_index, app)| {
            CatalogTarget {
                store: StoreKind::AppStore,
                identifier: non_empty(app.bundle_id.as_deref())
                    .or_else(|| non_empty(app.app_id.as_deref()))
                    .map(str::to_owned),
                config_index,
            }
        }));
    }

    if let Some(googleplay) = &config.stores.googleplay {
        targets.extend(
            googleplay
                .apps
                .iter()
                .enumerate()
                .map(|(config_index, app)| CatalogTarget {
                    store: StoreKind::GooglePlay,
                    identifier: non_empty(app.package_name.as_deref()).map(str::to_owned),
                    config_index,
                }),
        );
    }

    targets
}

async fn execute_catalog(command: StoreCatalogCommand) -> Result<()> {
    let config = Config::load()?;
    let targets = configured_catalog_targets(&config);
    if targets.is_empty() {
        return Err(anyhow!(
            "No catalog apps found in {}.",
            Config::DEFAULT_PATH
        ));
    }

    let mut executor = LiveCatalogExecutor::default();
    let outcomes = execute_catalog_targets(command, targets, &mut executor).await;
    print_catalog_summary(command, &outcomes);

    let failure_count = outcomes
        .iter()
        .filter(|outcome| outcome.error.is_some())
        .count();
    if failure_count > 0 {
        return Err(anyhow!(
            "{failure_count} catalog target(s) failed during {}",
            command.name()
        ));
    }

    Ok(())
}

impl StoreCatalogCommand {
    fn name(self) -> &'static str {
        match self {
            Self::Pull => "pull",
            Self::Push => "push",
        }
    }
}

#[async_trait]
trait CatalogExecutor {
    async fn execute(
        &mut self,
        command: StoreCatalogCommand,
        store: StoreKind,
        identifier: &str,
    ) -> Result<()>;
}

#[derive(Default)]
struct LiveCatalogExecutor {
    appstore: Option<Result<AppStoreConnectContext, String>>,
    googleplay: Option<Result<GooglePlayContext, String>>,
}

impl LiveCatalogExecutor {
    fn appstore_context(&mut self) -> Result<&AppStoreConnectContext> {
        if self.appstore.is_none() {
            self.appstore =
                Some(AppStoreConnectContext::from_env().map_err(|error| error.to_string()));
        }
        self.appstore
            .as_ref()
            .expect("App Store context must be initialized")
            .as_ref()
            .map_err(|error| anyhow!(error.clone()))
    }

    async fn googleplay_context(&mut self) -> Result<&GooglePlayContext> {
        if self.googleplay.is_none() {
            self.googleplay = Some(
                GooglePlayContext::from_env()
                    .await
                    .map_err(|error| error.to_string()),
            );
        }
        self.googleplay
            .as_ref()
            .expect("Google Play context must be initialized")
            .as_ref()
            .map_err(|error| anyhow!(error.clone()))
    }
}

#[async_trait]
impl CatalogExecutor for LiveCatalogExecutor {
    async fn execute(
        &mut self,
        command: StoreCatalogCommand,
        store: StoreKind,
        identifier: &str,
    ) -> Result<()> {
        match store {
            StoreKind::AppStore => {
                let context = self.appstore_context()?;
                match command {
                    StoreCatalogCommand::Pull => {
                        appstore_pull::execute_with_context(
                            &appstore_pull::PullArgs {
                                app: identifier.to_owned(),
                                version: None,
                                platform: None,
                                output: None,
                            },
                            context,
                        )
                        .await
                    }
                    StoreCatalogCommand::Push => {
                        appstore_push::execute_with_context(
                            &appstore_push::PushArgs {
                                app: identifier.to_owned(),
                                input: None,
                                dry_run: false,
                            },
                            context,
                        )
                        .await
                    }
                }
            }
            StoreKind::GooglePlay => {
                let context = self.googleplay_context().await?;
                match command {
                    StoreCatalogCommand::Pull => {
                        googleplay_pull::execute_with_context(
                            &googleplay_pull::PullArgs {
                                package_name: identifier.to_owned(),
                                output: None,
                            },
                            context,
                        )
                        .await
                    }
                    StoreCatalogCommand::Push => {
                        googleplay_push::execute_with_context(
                            &googleplay_push::PushArgs {
                                package_name: identifier.to_owned(),
                                input: None,
                                dry_run: false,
                            },
                            context,
                        )
                        .await
                    }
                }
            }
        }
    }
}

async fn execute_catalog_targets<E: CatalogExecutor>(
    command: StoreCatalogCommand,
    targets: Vec<CatalogTarget>,
    executor: &mut E,
) -> Vec<CatalogOutcome> {
    let mut outcomes = Vec::with_capacity(targets.len());

    for target in targets {
        let identifier = target.label();
        let result = if let Some(configured_identifier) = target.identifier.as_deref() {
            eprintln!(
                "\n==> {} {} catalog for {}",
                command.name(),
                target.store.name(),
                configured_identifier
            );
            executor
                .execute(command, target.store, configured_identifier)
                .await
        } else {
            Err(anyhow!(
                "missing catalog app identifier in {}",
                Config::DEFAULT_PATH
            ))
        };

        outcomes.push(CatalogOutcome {
            store: target.store,
            identifier,
            error: result.err().map(|error| format!("{error:#}")),
        });
    }

    outcomes
}

fn print_catalog_summary(command: StoreCatalogCommand, outcomes: &[CatalogOutcome]) {
    eprintln!("\nCatalog {} summary:", command.name());
    for outcome in outcomes {
        if let Some(error) = &outcome.error {
            eprintln!(
                "  ✗ {}: {} — {}",
                outcome.store.name(),
                outcome.identifier,
                error
            );
        } else {
            eprintln!("  ✓ {}: {}", outcome.store.name(), outcome.identifier);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser, Debug)]
    struct TestCli {
        #[command(flatten)]
        store: StoreArgs,
    }

    #[derive(Default)]
    struct MockExecutor {
        calls: Vec<(StoreCatalogCommand, StoreKind, String)>,
        fail_identifier: Option<String>,
    }

    #[async_trait]
    impl CatalogExecutor for MockExecutor {
        async fn execute(
            &mut self,
            command: StoreCatalogCommand,
            store: StoreKind,
            identifier: &str,
        ) -> Result<()> {
            self.calls.push((command, store, identifier.to_owned()));
            if self.fail_identifier.as_deref() == Some(identifier) {
                Err(anyhow!("simulated failure"))
            } else {
                Ok(())
            }
        }
    }

    fn config(yaml: &str) -> Config {
        serde_yaml::from_str(yaml).unwrap()
    }

    #[test]
    fn parses_catalog_commands_without_overrides() {
        let pull = TestCli::try_parse_from(["test", "catalog", "pull"]).unwrap();
        assert!(matches!(
            pull.store.command,
            StoreCommands::Catalog(StoreCatalogArgs {
                command: StoreCatalogCommand::Pull
            })
        ));

        let push = TestCli::try_parse_from(["test", "catalog", "push"]).unwrap();
        assert!(matches!(
            push.store.command,
            StoreCommands::Catalog(StoreCatalogArgs {
                command: StoreCatalogCommand::Push
            })
        ));

        assert!(TestCli::try_parse_from(["test", "catalog", "pull", "--app", "example"]).is_err());
    }

    #[test]
    fn builds_targets_in_store_and_config_order() {
        let targets = configured_catalog_targets(&config(
            r#"
stores:
  appstore:
    apps:
      - bundle_id: com.example.ios
        app_id: "123"
      - app_id: "456"
      - name: Missing
  googleplay:
    apps:
      - package_name: com.example.android
"#,
        ));

        assert_eq!(
            targets,
            vec![
                CatalogTarget {
                    store: StoreKind::AppStore,
                    identifier: Some("com.example.ios".to_owned()),
                    config_index: 0,
                },
                CatalogTarget {
                    store: StoreKind::AppStore,
                    identifier: Some("456".to_owned()),
                    config_index: 1,
                },
                CatalogTarget {
                    store: StoreKind::AppStore,
                    identifier: None,
                    config_index: 2,
                },
                CatalogTarget {
                    store: StoreKind::GooglePlay,
                    identifier: Some("com.example.android".to_owned()),
                    config_index: 0,
                },
            ]
        );
    }

    #[tokio::test]
    async fn continues_after_a_target_fails() {
        let targets = configured_catalog_targets(&config(
            r#"
stores:
  appstore:
    apps:
      - bundle_id: com.example.fail
      - bundle_id: com.example.pass
  googleplay:
    apps:
      - package_name: com.example.android
"#,
        ));
        let mut executor = MockExecutor {
            fail_identifier: Some("com.example.fail".to_owned()),
            ..Default::default()
        };

        let outcomes =
            execute_catalog_targets(StoreCatalogCommand::Pull, targets, &mut executor).await;

        assert_eq!(executor.calls.len(), 3);
        assert!(outcomes[0].error.is_some());
        assert!(outcomes[1].error.is_none());
        assert!(outcomes[2].error.is_none());
    }

    #[tokio::test]
    async fn reports_missing_identifiers_without_calling_executor() {
        let targets = configured_catalog_targets(&config(
            r#"
stores:
  appstore:
    apps:
      - name: Missing
"#,
        ));
        let mut executor = MockExecutor::default();

        let outcomes =
            execute_catalog_targets(StoreCatalogCommand::Push, targets, &mut executor).await;

        assert!(executor.calls.is_empty());
        assert!(outcomes[0].error.is_some());
    }
}
