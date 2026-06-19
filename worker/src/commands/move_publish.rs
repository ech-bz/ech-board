use crate::Ctx;
use crate::config::load_job_config;
use crate::sui_cli::SuiCli;
use crate::sui_cli::client::ObjectChange;
use anyhow::Context;
use ech_board_common::MovePublishConfig;
use ech_board_common::keys::{FORUM_REGISTRY, MOVE_ORIGINAL_ID};
use ech_k8s::StoreExt;
use k8s_openapi::api::core::v1::Secret;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

pub(crate) async fn run(ctx: &Ctx, config_path: &Path) -> anyhow::Result<()> {
    let config: MovePublishConfig = load_job_config(config_path)?;
    let sui = SuiCli::new(&config.rpc_url)?;
    let work_dir = tempfile::tempdir()?;

    let repo_dir = work_dir.path().join("repo");
    let status = Command::new("git")
        .arg("init")
        .arg(&repo_dir)
        .status()
        .context("git init failed")?;
    anyhow::ensure!(status.success(), "git init exited with {status}");

    let status = Command::new("git")
        .args(["remote", "add", "origin", &config.repo])
        .current_dir(&repo_dir)
        .status()
        .context("git remote add failed")?;
    anyhow::ensure!(status.success(), "git remote add exited with {status}");

    let status = Command::new("git")
        .args([
            "fetch",
            "--depth=1",
            "--filter=blob:none",
            "origin",
            &config.git_ref,
        ])
        .current_dir(&repo_dir)
        .status()
        .context("git fetch failed")?;
    anyhow::ensure!(status.success(), "git fetch exited with {status}");

    let status = Command::new("git")
        .args(["sparse-checkout", "set", &config.package_path])
        .current_dir(&repo_dir)
        .status()
        .context("git sparse-checkout failed")?;
    anyhow::ensure!(status.success(), "git sparse-checkout exited with {status}");

    let status = Command::new("git")
        .args(["checkout", "FETCH_HEAD"])
        .current_dir(&repo_dir)
        .status()
        .context("git checkout failed")?;
    anyhow::ensure!(status.success(), "git checkout exited with {status}");

    let package_dir = repo_dir.join(&config.package_path);

    let chain_id = sui.client().chain_identifier()?;
    tracing::info!(chain_id, "resolved chain identifier");
    std::fs::write(
        package_dir.join("Move.toml"),
        format!(
            "{}\n[environments]\nlocalnet = \"{chain_id}\"\n",
            std::fs::read_to_string(package_dir.join("Move.toml"))
                .context("failed to read Move.toml")?
        ),
    )
    .context("failed to append environments to Move.toml")?;

    sui.move_()
        .build(&package_dir)
        .context("sui move build failed")?;

    std::fs::write(
        crate::sui_cli::SUI_KEYSTORE_PATH,
        format!("[\"{}\"]", config.publisher_key_base64),
    )
    .context("failed to write keystore")?;

    if config.original_id.is_some() {
        anyhow::bail!("upgrade is not supported without graphql");
    } else {
        let output = sui
            .client()
            .publish(&package_dir)
            .context("sui client publish failed")?;

        let package_id = output
            .object_changes
            .iter()
            .find_map(|c| match c {
                ObjectChange::Published { package_id, .. } => Some(package_id.clone()),
                _ => None,
            })
            .context("published package not found in publish output")?;

        let forum_registry = output
            .object_changes
            .iter()
            .find_map(|c| match c {
                ObjectChange::Created {
                    object_type,
                    object_id,
                    ..
                } if object_type.starts_with(&format!("{package_id}::forum::ForumObject<")) => {
                    Some(object_id.clone())
                }
                _ => None,
            })
            .context("ForumObject not found in publish output")?;

        tracing::info!(
            network = %config.worker.network_name,
            package_id,
            forum_registry,
            git_ref = %config.git_ref,
            "move package published"
        );

        ctx.k8s
            .namespaced::<Secret>(&config.worker.namespace)
            .store_put(
                &config.output_name,
                BTreeMap::new(),
                BTreeMap::from([
                    (MOVE_ORIGINAL_ID.to_string(), package_id),
                    (FORUM_REGISTRY.to_string(), forum_registry),
                ]),
            )
            .await?;
    }

    Ok(())
}
